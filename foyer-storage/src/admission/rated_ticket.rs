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

use std::{fmt::Debug, marker::PhantomData, sync::Arc};

use foyer_common::{
    code::{Key, Value},
    rated_ticket::RatedTicket,
};

use crate::metrics::Metrics;

use super::AdmissionPolicy;

#[derive(Debug)]
pub struct RatedTicketAdmissionPolicy<K, V>
where
    K: Key,
    V: Value,
{
    inner: RatedTicket,

    _marker: PhantomData<(K, V)>,
}

impl<K, V> RatedTicketAdmissionPolicy<K, V>
where
    K: Key,
    V: Value,
{
    pub fn new(rate: usize) -> Self {
        Self {
            inner: RatedTicket::new(rate as f64),
            _marker: PhantomData,
        }
    }
}

impl<K, V> AdmissionPolicy for RatedTicketAdmissionPolicy<K, V>
where
    K: Key,
    V: Value,
{
    type Key = K;

    type Value = V;

    fn judge(&self, _key: &Self::Key, _weight: usize, _metrics: &Arc<Metrics>) -> bool {
        self.inner.probe()
    }

    fn on_insert(&self, _key: &Self::Key, weight: usize, _metrics: &Arc<Metrics>, _judge: bool) {
        self.inner.reduce(weight as f64);
    }

    fn on_drop(&self, _key: &Self::Key, _weight: usize, _metrics: &Arc<Metrics>, _judge: bool) {}
}
