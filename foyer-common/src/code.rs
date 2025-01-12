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

use bytes::{Buf, BufMut};
use paste::paste;

#[expect(unused_variables)]
pub trait Key:
    Sized
    + Send
    + Sync
    + 'static
    + std::hash::Hash
    + Eq
    + PartialEq
    + Ord
    + PartialOrd
    + Clone
    + std::fmt::Debug
{
    #[cfg_attr(coverage_nightly, no_coverage)]
    fn weight(&self) -> usize {
        std::mem::size_of::<Self>()
    }

    #[cfg_attr(coverage_nightly, no_coverage)]
    fn serialized_len(&self) -> usize {
        panic!("Method `serialized_len` must be implemented for `Key` if storage is used.")
    }

    #[cfg_attr(coverage_nightly, no_coverage)]
    fn write(&self, buf: &mut [u8]) {
        panic!("Method `write` must be implemented for `Key` if storage is used.")
    }

    #[cfg_attr(coverage_nightly, no_coverage)]
    fn read(buf: &[u8]) -> Self {
        panic!("Method `read` must be implemented for `Key` if storage is used.")
    }
}

#[expect(unused_variables)]
pub trait Value: Sized + Send + Sync + 'static + std::fmt::Debug {
    #[cfg_attr(coverage_nightly, no_coverage)]
    fn weight(&self) -> usize {
        std::mem::size_of::<Self>()
    }

    #[cfg_attr(coverage_nightly, no_coverage)]
    fn serialized_len(&self) -> usize {
        panic!("Method `serialized_len` must be implemented for `Value` if storage is used.")
    }

    #[cfg_attr(coverage_nightly, no_coverage)]
    fn write(&self, buf: &mut [u8]) {
        panic!("Method `write` must be implemented for `Value` if storage is used.")
    }

    #[cfg_attr(coverage_nightly, no_coverage)]
    fn read(buf: &[u8]) -> Self {
        panic!("Method `read` must be implemented for `Value` if storage is used.")
    }
}

macro_rules! for_all_primitives {
    ($macro:ident) => {
        $macro! {
            u8, u16, u32, u64,
            i8, i16, i32, i64,
        }
    };
}

macro_rules! impl_key {
    ($( $type:ty, )*) => {
        paste! {
            $(
                impl Key for $type {
                    #[cfg_attr(coverage_nightly, no_coverage)]
                    fn serialized_len(&self) -> usize {
                        std::mem::size_of::<$type>()
                    }

                    #[cfg_attr(coverage_nightly, no_coverage)]
                    fn write(&self, mut buf: &mut [u8]) {
                        buf.[< put_ $type>](*self)
                    }

                    #[cfg_attr(coverage_nightly, no_coverage)]
                    fn read(mut buf: &[u8]) -> Self {
                        buf.[< get_ $type>]()
                    }
                }
            )*
        }
    };
}

macro_rules! impl_value {
    ($( $type:ty, )*) => {
        paste! {
            $(
                impl Value for $type {
                    #[cfg_attr(coverage_nightly, no_coverage)]
                    fn serialized_len(&self) -> usize {
                        std::mem::size_of::<$type>()
                    }

                    #[cfg_attr(coverage_nightly, no_coverage)]
                    fn write(&self, mut buf: &mut [u8]) {
                        buf.[< put_ $type>](*self)
                    }

                    #[cfg_attr(coverage_nightly, no_coverage)]
                    fn read(mut buf: &[u8]) -> Self {
                        buf.[< get_ $type>]()
                    }
                }
            )*
        }
    };
}

for_all_primitives! { impl_key }
for_all_primitives! { impl_value }

impl Value for Vec<u8> {
    #[cfg_attr(coverage_nightly, no_coverage)]
    fn weight(&self) -> usize {
        self.len()
    }

    #[cfg_attr(coverage_nightly, no_coverage)]
    fn serialized_len(&self) -> usize {
        self.len()
    }

    #[cfg_attr(coverage_nightly, no_coverage)]
    fn write(&self, mut buf: &mut [u8]) {
        buf.put_slice(self);
    }

    #[cfg_attr(coverage_nightly, no_coverage)]
    fn read(buf: &[u8]) -> Self {
        buf.to_vec()
    }
}
