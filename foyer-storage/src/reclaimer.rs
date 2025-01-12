//  Copyright 2023 MrCroxx
//
//  Licensed under the Apache License, Version 2.0 (the "License");
//  you may not use this file except in compliance with the License.
//  You may obtain a copy of the License at
//
//  http://www.apache.org/licenses/LICENSE-2.0
//
//  Unless required by applicable law or agreed to in writing, software
//  distributed under the License is distributed on an "AS IS" BASIS,
//  WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
//  See the License for the specific language governing permissions and
//  limitations under the License.

use std::{sync::Arc, time::Duration};

use crate::{
    device::Device,
    error::Result,
    generic::{GenericStore, RegionEntryIter},
    judge::Judges,
    metrics::Metrics,
    region_manager::{RegionEpItemAdapter, RegionManager},
    storage::Storage,
};
use bytes::BufMut;
use foyer_common::{
    code::{Key, Value},
    rate::RateLimiter,
};
use foyer_intrusive::{core::adapter::Link, eviction::EvictionPolicy};
use tokio::sync::broadcast;

#[derive(Debug)]
pub struct Reclaimer<K, V, D, EP, EL>
where
    K: Key,
    V: Value,
    D: Device,
    EP: EvictionPolicy<Adapter = RegionEpItemAdapter<EL>>,
    EL: Link,
{
    threshold: usize,

    store: GenericStore<K, V, D, EP, EL>,

    region_manager: Arc<RegionManager<D, EP, EL>>,

    rate_limiter: Option<Arc<RateLimiter>>,

    metrics: Arc<Metrics>,

    stop_rx: broadcast::Receiver<()>,
}

impl<K, V, D, EP, EL> Reclaimer<K, V, D, EP, EL>
where
    K: Key,
    V: Value,
    D: Device,
    EP: EvictionPolicy<Adapter = RegionEpItemAdapter<EL>>,
    EL: Link,
{
    pub fn new(
        threshold: usize,
        store: GenericStore<K, V, D, EP, EL>,
        region_manager: Arc<RegionManager<D, EP, EL>>,
        rate_limiter: Option<Arc<RateLimiter>>,
        metrics: Arc<Metrics>,
        stop_rx: broadcast::Receiver<()>,
    ) -> Self {
        Self {
            threshold,
            store,
            region_manager,
            rate_limiter,
            metrics,
            stop_rx,
        }
    }

    pub async fn run(mut self) -> Result<()> {
        let mut watch = self.region_manager.clean_regions().watch();
        loop {
            tokio::select! {
                biased;
                Ok(()) = watch.changed() => {
                    self.handle().await?;
                }
                _ = self.stop_rx.recv() => {
                    tracing::info!("[reclaimer] exit");
                    return Ok(())
                }
            }
        }
    }

    async fn handle(&self) -> Result<()> {
        if self.region_manager.clean_regions().len() >= self.threshold {
            return Ok(());
        }

        // TODO(MrCroxx): subscribe evictable region changes.
        let region_id = loop {
            match self.region_manager.eviction_pop() {
                Some(id) => break id,
                None => tokio::time::sleep(Duration::from_millis(100)).await,
            }
        };

        let _timer = self.metrics.slow_op_duration_reclaim.start_timer();

        let region = self.region_manager.region(&region_id);

        // step 1: drop indices
        let _indices = self.store.catalog().take_region(&region_id);

        // after drop indices and acquire exclusive lock, no writers or readers are supposed to access the region
        {
            let guard = region.exclusive(false, false, false).await;
            tracing::trace!(
                "[reclaimer] region {}, writers: {}, buffered readers: {}, physical readers: {}",
                region.id(),
                guard.writers(),
                guard.buffered_readers(),
                guard.physical_readers()
            );
            drop(guard);
        }

        // step 2: do reinsertion
        let reinsert = || {
            let region = region.clone();
            let metrics = self.metrics.clone();
            let rate = self.rate_limiter.clone();
            let reinsertions = self.store.reinsertions().clone();

            tracing::info!("[reclaimer] begin reinsertion, region: {}", region_id);

            async move {
                let mut iter = match RegionEntryIter::<K, V, D>::open(region).await {
                    Ok(Some(iter)) => iter,
                    Ok(None) => return Ok(true),
                    Err(e) => return Err(e),
                };

                while let Some((key, value)) = iter.next_kv().await? {
                    let weight = key.serialized_len() + value.serialized_len();

                    let mut judges = Judges::new(reinsertions.len());
                    for (index, reinsertion) in reinsertions.iter().enumerate() {
                        let judge = reinsertion.judge(&key, weight, &metrics);
                        judges.set(index, judge);
                    }
                    if !judges.judge() {
                        for (index, reinsertion) in reinsertions.iter().enumerate() {
                            let judge = judges.get(index);
                            reinsertion.on_drop(&key, weight, &metrics, judge);
                        }
                        continue;
                    }

                    // TODO(MrCroxx): Should reclaimer use wait if exceed limitation?
                    if let Some(rate) = rate.as_ref() && let Some(wait) = rate.consume(weight as f64) {
                        tokio::time::sleep(wait).await;
                    }

                    let mut writer = self.store.writer(key.clone(), weight);
                    writer.set_skippable();

                    if !writer.judge() {
                        continue;
                    }

                    if writer.finish(value).await? {
                        for (index, reinsertion) in reinsertions.iter().enumerate() {
                            let judge = judges.get(index);
                            reinsertion.on_insert(&key, weight, &metrics, judge);
                        }
                    } else {
                        for (index, reinsertion) in reinsertions.iter().enumerate() {
                            let judge = judges.get(index);
                            reinsertion.on_drop(&key, weight, &metrics, judge);
                        }
                        // The writer is already been judged and admitted, but not inserted successfully and skipped.
                        // That means allocating timeouts and there is no clean region available.
                        // Reinsertion should be interrupted to make sure foreground insertion.
                        return Ok(false);
                    }

                    metrics.op_bytes_reinsert.inc_by(weight as u64);
                }

                tracing::info!("[reclaimer] finish reinsertion, region: {}", region_id);

                Ok(true)
            }
        };

        if !self.store.reinsertions().is_empty() {
            match reinsert().await {
                Ok(true) => {
                    tracing::info!("[reclaimer] reinsertion finish, region: {}", region_id)
                }
                Ok(false) => {
                    tracing::info!("[reclaimer] reinsertion skipped, region: {}", region_id)
                }
                Err(e) => tracing::warn!("reinsert region {:?} error: {:?}", region, e),
            }
        }

        // step 3: set region last block zero
        let align = region.device().align();
        let mut buf = region.device().io_buffer(align, align);
        (&mut buf[..]).put_slice(&vec![0; align]);
        let (res, _buf) = region.device().write(buf, .., region_id, 0).await;
        res?;

        // step 4: send clean region
        self.region_manager.clean_regions().release(region_id);

        tracing::info!("[reclaimer] finish reclaim task, region: {}", region_id);

        self.metrics
            .op_bytes_reclaim
            .inc_by(region.device().region_size() as u64);
        self.metrics
            .total_bytes
            .sub(region.device().region_size() as u64);

        Ok(())
    }
}
