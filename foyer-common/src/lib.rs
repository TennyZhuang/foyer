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

#![feature(trait_alias)]
#![feature(lint_reasons)]
#![feature(bound_map)]
#![cfg_attr(coverage_nightly, feature(no_coverage))]

pub mod batch;
pub mod bits;
pub mod code;
pub mod continuum;
pub mod erwlock;
pub mod queue;
pub mod range;
pub mod rate;
pub mod rated_random;
pub mod rated_ticket;
pub mod runtime;
