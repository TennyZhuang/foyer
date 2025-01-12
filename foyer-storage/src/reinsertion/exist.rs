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

use std::{
    marker::PhantomData,
    sync::{Arc, OnceLock},
};

use foyer_common::code::{Key, Value};

use crate::catalog::Catalog;

use super::ReinsertionPolicy;

#[derive(Debug)]
pub struct ExistReinsertionPolicy<K, V>
where
    K: Key,
    V: Value,
{
    indices: OnceLock<Arc<Catalog<K>>>,
    _marker: PhantomData<V>,
}

impl<K, V> Default for ExistReinsertionPolicy<K, V>
where
    K: Key,
    V: Value,
{
    fn default() -> Self {
        Self {
            indices: OnceLock::new(),
            _marker: PhantomData,
        }
    }
}

impl<K, V> ReinsertionPolicy for ExistReinsertionPolicy<K, V>
where
    K: Key,
    V: Value,
{
    type Key = K;

    type Value = V;

    fn init(&self, indices: &Arc<Catalog<Self::Key>>) {
        self.indices.get_or_init(|| indices.clone());
    }

    fn judge(
        &self,
        key: &Self::Key,
        _weight: usize,
        _metrics: &std::sync::Arc<crate::metrics::Metrics>,
    ) -> bool {
        let indices = self.indices.get().unwrap();
        indices.lookup(key).is_some()
    }

    fn on_insert(
        &self,
        _key: &Self::Key,
        _weight: usize,
        _metrics: &Arc<crate::metrics::Metrics>,
        _judge: bool,
    ) {
    }

    fn on_drop(
        &self,
        _key: &Self::Key,
        _weight: usize,
        _metrics: &Arc<crate::metrics::Metrics>,
        _judge: bool,
    ) {
    }
}
