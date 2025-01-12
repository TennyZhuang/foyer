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

use std::sync::{LazyLock, OnceLock};

use prometheus::{
    core::{AtomicU64, GenericGauge, GenericGaugeVec},
    opts, register_histogram_vec_with_registry, register_int_counter_vec_with_registry, Histogram,
    HistogramVec, IntCounter, IntCounterVec, Registry,
};
type UintGaugeVec = GenericGaugeVec<AtomicU64>;
type UintGauge = GenericGauge<AtomicU64>;

macro_rules! register_gauge_vec {
    ($TYPE:ident, $OPTS:expr, $LABELS_NAMES:expr, $REGISTRY:expr $(,)?) => {{
        let gauge_vec = $TYPE::new($OPTS, $LABELS_NAMES).unwrap();
        $REGISTRY
            .register(Box::new(gauge_vec.clone()))
            .map(|_| gauge_vec)
    }};
}

macro_rules! register_uint_gauge_vec_with_registry {
    ($OPTS:expr, $LABELS_NAMES:expr, $REGISTRY:expr $(,)?) => {{
        register_gauge_vec!(UintGaugeVec, $OPTS, $LABELS_NAMES, $REGISTRY)
    }};

    ($NAME:expr, $HELP:expr, $LABELS_NAMES:expr, $REGISTRY:expr $(,)?) => {{
        register_uint_gauge_vec_with_registry!(opts!($NAME, $HELP), $LABELS_NAMES, $REGISTRY)
    }};
}

static REGISTRY: OnceLock<Registry> = OnceLock::new();

/// Set metrics registry for `foyer`.
///
/// Metrics registry must be set before `open`.
///
/// Return `true` if set succeeds.
pub fn set_metrics_registry(registry: Registry) -> bool {
    REGISTRY.set(registry).is_ok()
}

pub fn get_metrics_registry() -> &'static Registry {
    REGISTRY.get_or_init(|| prometheus::default_registry().clone())
}

/// Multiple foyer instance will share the same global metrics with different label `foyer` name.
pub static METRICS: LazyLock<GlobalMetrics> = LazyLock::new(GlobalMetrics::default);

#[derive(Debug)]
pub struct GlobalMetrics {
    op_duration: HistogramVec,
    slow_op_duration: HistogramVec,
    op_bytes: IntCounterVec,
    total_bytes: UintGaugeVec,

    inner_op_duration: HistogramVec,
}

impl Default for GlobalMetrics {
    fn default() -> Self {
        Self::new(get_metrics_registry())
    }
}

impl GlobalMetrics {
    pub fn new(registry: &Registry) -> Self {
        let op_duration = register_histogram_vec_with_registry!(
            "foyer_storage_op_duration",
            "foyer storage op duration",
            &["foyer", "op", "extra"],
            vec![0.0001, 0.001, 0.005, 0.01, 0.02, 0.05, 0.075, 0.1, 0.25, 0.5, 0.75, 1.0],
            registry,
        )
        .unwrap();

        let slow_op_duration = register_histogram_vec_with_registry!(
            "foyer_storage_slow_op_duration",
            "foyer storage slow op duration",
            &["foyer", "op", "extra"],
            vec![0.01, 0.1, 0.5, 0.77, 1.0, 2.5, 5.0, 7.5, 10.0],
            registry,
        )
        .unwrap();

        let op_bytes = register_int_counter_vec_with_registry!(
            "foyer_storage_op_bytes",
            "foyer storage op bytes",
            &["foyer", "op", "extra"],
            registry,
        )
        .unwrap();

        let total_bytes = register_uint_gauge_vec_with_registry!(
            "foyer_storage_total_bytes",
            "foyer storage total bytes",
            &["foyer"],
            registry,
        )
        .unwrap();

        let inner_op_duration = register_histogram_vec_with_registry!(
            "foyer_storage_inner_op_duration",
            "foyer storage inner op duration",
            &["foyer", "op", "extra"],
            vec![0.0001, 0.01, 0.02, 0.05, 0.075, 0.1, 0.25, 0.5, 0.75, 1.0],
            registry,
        )
        .unwrap();

        Self {
            op_duration,
            slow_op_duration,
            op_bytes,
            total_bytes,

            inner_op_duration,
        }
    }

    pub fn foyer(&self, name: &str) -> Metrics {
        Metrics::new(self, name)
    }
}

#[derive(Debug)]
pub struct Metrics {
    pub op_duration_insert_inserted: Histogram,
    pub op_duration_insert_filtered: Histogram,
    pub op_duration_insert_dropped: Histogram,
    pub op_duration_lookup_hit: Histogram,
    pub op_duration_lookup_miss: Histogram,
    pub op_duration_remove: Histogram,
    pub slow_op_duration_flush: Histogram,
    pub slow_op_duration_reclaim: Histogram,

    pub op_bytes_insert: IntCounter,
    pub op_bytes_lookup: IntCounter,
    pub op_bytes_flush: IntCounter,
    pub op_bytes_reclaim: IntCounter,
    pub op_bytes_reinsert: IntCounter,

    pub total_bytes: UintGauge,

    pub inner_op_duration_acquire_clean_region: Histogram,
    pub inner_op_duration_acquire_clean_buffer: Histogram,
}

impl Metrics {
    pub fn new(global: &GlobalMetrics, foyer: &str) -> Self {
        let op_duration_insert_inserted = global
            .op_duration
            .with_label_values(&[foyer, "insert", "inserted"]);
        let op_duration_insert_filtered = global
            .op_duration
            .with_label_values(&[foyer, "insert", "filtered"]);
        let op_duration_insert_dropped = global
            .op_duration
            .with_label_values(&[foyer, "insert", "dropped"]);
        let op_duration_lookup_hit = global
            .op_duration
            .with_label_values(&[foyer, "lookup", "hit"]);
        let op_duration_lookup_miss = global
            .op_duration
            .with_label_values(&[foyer, "lookup", "miss"]);
        let op_duration_remove = global.op_duration.with_label_values(&[foyer, "remove", ""]);
        let slow_op_duration_flush = global
            .slow_op_duration
            .with_label_values(&[foyer, "flush", ""]);
        let slow_op_duration_reclaim = global
            .slow_op_duration
            .with_label_values(&[foyer, "reclaim", ""]);

        let op_bytes_insert = global.op_bytes.with_label_values(&[foyer, "insert", ""]);
        let op_bytes_lookup = global.op_bytes.with_label_values(&[foyer, "lookup", ""]);
        let op_bytes_flush = global.op_bytes.with_label_values(&[foyer, "flush", ""]);
        let op_bytes_reclaim = global.op_bytes.with_label_values(&[foyer, "reclaim", ""]);
        let op_bytes_reinsert = global.op_bytes.with_label_values(&[foyer, "reinsert", ""]);

        let total_bytes = global.total_bytes.with_label_values(&[foyer]);

        let inner_op_duration_acquire_clean_region =
            global
                .inner_op_duration
                .with_label_values(&[foyer, "acquire_clean_region", ""]);
        let inner_op_duration_acquire_clean_buffer =
            global
                .inner_op_duration
                .with_label_values(&[foyer, "acquire_clean_buffer", ""]);

        Self {
            op_duration_insert_inserted,
            op_duration_insert_filtered,
            op_duration_insert_dropped,
            op_duration_lookup_hit,
            op_duration_lookup_miss,
            op_duration_remove,
            slow_op_duration_flush,
            slow_op_duration_reclaim,

            op_bytes_insert,
            op_bytes_lookup,
            op_bytes_flush,
            op_bytes_reclaim,
            op_bytes_reinsert,

            total_bytes,

            inner_op_duration_acquire_clean_region,
            inner_op_duration_acquire_clean_buffer,
        }
    }
}
