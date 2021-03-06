// Copyright 2014 The Prometheus Authors
// Copyright 2018 PingCAP, Inc.
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//     http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// See the License for the specific language governing permissions and
// limitations under the License.

use std::cmp::*;
use std::f64;
use std::ops::*;
use std::sync::atomic::{AtomicI64 as StdAtomicI64, AtomicU64 as StdAtomicU64, Ordering};

/// An interface for numbers. Used to generically model float metrics and integer metrics, i.e.
/// [`Counter`](::Counter) and [`IntCounter`](::IntCounter).
pub trait Number:
    Sized + AddAssign + SubAssign + PartialOrd + PartialEq + Copy + Send + Sync
{
    /// `std::convert::From<i64> for f64` is not implemented, so that we need to implement our own.
    fn from_i64(v: i64) -> Self;
    /// Convert to a f64.
    fn into_f64(self) -> f64;
}

impl Number for i64 {
    #[inline]
    fn from_i64(v: i64) -> Self {
        v
    }

    #[inline]
    fn into_f64(self) -> f64 {
        self as f64
    }
}

impl Number for u64 {
    #[inline]
    fn from_i64(v: i64) -> Self {
        v as u64
    }

    #[inline]
    fn into_f64(self) -> f64 {
        self as f64
    }
}

impl Number for f64 {
    #[inline]
    fn from_i64(v: i64) -> Self {
        v as f64
    }

    #[inline]
    fn into_f64(self) -> f64 {
        self
    }
}

/// An interface for atomics. Used to generically model float metrics and integer metrics, i.e.
/// [`Counter`](::Counter) and [`IntCounter`](::IntCounter).
pub trait Atomic: Send + Sync {
    /// The numeric type associated with this atomic.
    type T: Number;
    /// Create a new atomic value.
    fn new(val: Self::T) -> Self;
    /// Set the value to the provided value.
    fn set(&self, val: Self::T);
    /// Get the value.
    fn get(&self) -> Self::T;
    /// Increment the value by a given amount.
    fn inc_by(&self, delta: Self::T);
    /// Decrement the value by a given amount.
    fn dec_by(&self, delta: Self::T);
}

/// A atomic float.
pub struct AtomicF64 {
    inner: StdAtomicU64,
}

#[inline]
fn u64_to_f64(val: u64) -> f64 {
    f64::from_bits(val)
}

#[inline]
fn f64_to_u64(val: f64) -> u64 {
    f64::to_bits(val)
}

impl Atomic for AtomicF64 {
    type T = f64;

    fn new(val: Self::T) -> AtomicF64 {
        AtomicF64 {
            inner: StdAtomicU64::new(f64_to_u64(val)),
        }
    }

    #[inline]
    fn set(&self, val: Self::T) {
        self.inner.store(f64_to_u64(val), Ordering::Relaxed);
    }

    #[inline]
    fn get(&self) -> Self::T {
        u64_to_f64(self.inner.load(Ordering::Relaxed))
    }

    #[inline]
    fn inc_by(&self, delta: Self::T) {
        loop {
            let current = self.inner.load(Ordering::Acquire);
            let new = u64_to_f64(current) + delta;
            let swapped = self
                .inner
                .compare_and_swap(current, f64_to_u64(new), Ordering::Release);
            if swapped == current {
                return;
            }
        }
    }

    #[inline]
    fn dec_by(&self, delta: Self::T) {
        self.inc_by(-delta);
    }
}

/// A atomic signed integer.
pub struct AtomicI64 {
    inner: StdAtomicI64,
}

impl Atomic for AtomicI64 {
    type T = i64;

    fn new(val: Self::T) -> AtomicI64 {
        AtomicI64 {
            inner: StdAtomicI64::new(val),
        }
    }

    #[inline]
    fn set(&self, val: Self::T) {
        self.inner.store(val, Ordering::Relaxed);
    }

    #[inline]
    fn get(&self) -> Self::T {
        self.inner.load(Ordering::Relaxed)
    }

    #[inline]
    fn inc_by(&self, delta: Self::T) {
        self.inner.fetch_add(delta, Ordering::Relaxed);
    }

    #[inline]
    fn dec_by(&self, delta: Self::T) {
        self.inner.fetch_sub(delta, Ordering::Relaxed);
    }
}

/// A atomic unsigned integer.
pub struct AtomicU64 {
    inner: StdAtomicU64,
}

impl Atomic for AtomicU64 {
    type T = u64;

    fn new(val: Self::T) -> AtomicU64 {
        AtomicU64 {
            inner: StdAtomicU64::new(val),
        }
    }

    #[inline]
    fn set(&self, val: Self::T) {
        self.inner.store(val, Ordering::Relaxed);
    }

    #[inline]
    fn get(&self) -> Self::T {
        self.inner.load(Ordering::Relaxed)
    }

    #[inline]
    fn inc_by(&self, delta: Self::T) {
        self.inner.fetch_add(delta, Ordering::Relaxed);
    }

    #[inline]
    fn dec_by(&self, delta: Self::T) {
        self.inner.fetch_sub(delta, Ordering::Relaxed);
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::f64::consts::PI;
    use std::f64::{self, EPSILON};

    #[test]
    fn test_atomic_f64() {
        let table: Vec<f64> = vec![0.0, 1.0, PI, f64::MIN, f64::MAX];

        for f in table {
            assert!((f - AtomicF64::new(f).get()).abs() < EPSILON);
        }
    }

    #[test]
    fn test_atomic_i64() {
        let ai64 = AtomicI64::new(0);
        assert_eq!(ai64.get(), 0);

        ai64.inc_by(1);
        assert_eq!(ai64.get(), 1);

        ai64.inc_by(-5);
        assert_eq!(ai64.get(), -4);
    }

    #[test]
    fn test_atomic_u64() {
        let au64 = AtomicU64::new(0);
        assert_eq!(au64.get(), 0);

        au64.inc_by(123);
        assert_eq!(au64.get(), 123);
    }
}
