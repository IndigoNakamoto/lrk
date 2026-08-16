use std::sync::{
    Arc,
    atomic::{AtomicU64, Ordering::Relaxed},
};

use parking_lot::RwLock;

mod any_vec;
mod budget;
mod clone;
mod cloneable;
mod read_only_clone;
mod readable;
mod typed;

pub use budget::{CachedVecBudget, NoBudget};
pub use cloneable::{CachedBoxedVec, CachedReadableVec};

use crate::{ReadOnlyClone, ReadableVec, StoredVec, TypedVec, VecIndex, Version};

static NO_BUDGET: NoBudget = NoBudget;

struct CacheState<T> {
    len: usize,
    version: Version,
    generation: u64,
    data: Arc<[T]>,
}

impl<T> CacheState<T> {
    fn empty() -> Self {
        Self {
            len: 0,
            version: Version::ZERO,
            generation: 0,
            data: Arc::from([]),
        }
    }

    fn matches(&self, len: usize, version: Version) -> bool {
        self.len == len && self.version == version
    }

    fn invalidate(&mut self) {
        self.len = 0;
        self.version = Version::ZERO;
        self.generation = self.generation.wrapping_add(1);
        self.data = Arc::from([]);
    }

    fn replace(&mut self, len: usize, version: Version, data: Arc<[T]>) {
        self.len = len;
        self.version = version;
        self.data = data;
    }
}

/// Cached wrapper around any readable vec, refreshed when len or version changes.
///
/// Wraps a concrete vec `V` and adds an in-memory cache layer.
/// Reads check the cache first; on miss, the inner vec is read and cached.
///
/// For writes, access the inner vec directly via the `inner` field.
/// After a same-length, same-version rewrite, call [`Self::clear`] after the
/// mutation and before dependent reads.
///
/// When constructed with a budget, materialization is gated: if the budget
/// is exhausted, reads fall through to the inner vec without caching.
pub struct CachedVec<V: TypedVec> {
    pub inner: V,
    cache: Arc<RwLock<CacheState<V::T>>>,
    pub(super) budget: &'static dyn CachedVecBudget,
    pub(super) access_count: Option<Arc<AtomicU64>>,
}

impl<V: TypedVec> CachedVec<V> {
    pub fn wrap(inner: V) -> Self {
        Self {
            inner,
            cache: Arc::new(RwLock::new(CacheState::empty())),
            budget: &NO_BUDGET,
            access_count: None,
        }
    }

    pub fn wrap_budgeted(
        inner: V,
        budget: &'static dyn CachedVecBudget,
        access_count: Arc<AtomicU64>,
    ) -> Self {
        Self {
            inner,
            cache: Arc::new(RwLock::new(CacheState::empty())),
            budget,
            access_count: Some(access_count),
        }
    }

    #[inline(always)]
    pub fn version(&self) -> Version {
        self.inner.version()
    }

    pub fn clear(&self) {
        self.cache.write().invalidate();
        if let Some(c) = &self.access_count {
            c.store(0, Relaxed);
        }
    }
}

impl<V: TypedVec + ReadableVec<V::I, V::T>> CachedVec<V> {
    /// Returns the full cached snapshot. Always materializes on miss (ignores budget).
    #[inline(always)]
    pub fn cached(&self) -> Arc<[V::T]> {
        self.materialize(false).unwrap()
    }

    /// Returns the value at the given typed index from the cached snapshot.
    #[inline(always)]
    pub fn get(&self, index: V::I) -> Option<V::T> {
        self.get_at(index.to_usize())
    }

    /// Returns the value at the given raw index from the cached snapshot.
    #[inline(always)]
    pub fn get_at(&self, index: usize) -> Option<V::T> {
        self.cached().get(index).cloned()
    }

    /// Returns `None` when budget is exhausted or below min access threshold.
    #[inline]
    pub(super) fn try_cached(&self) -> Option<Arc<[V::T]>> {
        self.materialize(true)
    }

    fn materialize(&self, check_budget: bool) -> Option<Arc<[V::T]>> {
        let count = self
            .access_count
            .as_ref()
            .map(|c| c.fetch_add(1, Relaxed) + 1)
            .unwrap_or(0);
        let mut reserved = false;

        loop {
            let len = self.inner.len();
            let version = self.inner.version();
            let generation = {
                let cache = self.cache.read();
                if cache.matches(len, version) {
                    return Some(cache.data.clone());
                }
                cache.generation
            };

            if check_budget && !reserved {
                if !self.budget.try_reserve(count) {
                    return None;
                }
                reserved = true;
            }

            let data: Arc<[V::T]> = self.inner.collect_range_dyn(0, len).into();
            let mut cache = self.cache.write();
            if cache.matches(len, version) {
                return Some(cache.data.clone());
            }
            if cache.generation != generation {
                continue;
            }
            cache.replace(len, version, data.clone());

            return Some(data);
        }
    }
}

impl<V: StoredVec> CachedVec<V> {
    /// Boxes a read-only clone for use with type-erased APIs (e.g. LazyVec).
    #[inline]
    pub fn read_only_boxed_clone(&self) -> crate::ReadableBoxedVec<V::I, V::T> {
        Box::new(self.read_only_clone())
    }
}

#[cfg(test)]
mod tests {
    use std::sync::{
        Arc, Barrier,
        atomic::{AtomicBool, Ordering::SeqCst},
    };

    use crate::{AnyVec, PrintableIndex, ReadableVec, TypedVec, short_type_name};

    use super::*;

    #[derive(Clone)]
    struct BlockingVec {
        values: Arc<RwLock<Vec<u32>>>,
        started: Arc<Barrier>,
        resume: Arc<Barrier>,
        block_once: Arc<AtomicBool>,
    }

    impl BlockingVec {
        fn new(values: impl IntoIterator<Item = u32>) -> Self {
            Self {
                values: Arc::new(RwLock::new(values.into_iter().collect())),
                started: Arc::new(Barrier::new(2)),
                resume: Arc::new(Barrier::new(2)),
                block_once: Arc::new(AtomicBool::new(true)),
            }
        }

        fn replace(&self, index: usize, value: u32) {
            self.values.write()[index] = value;
        }

        fn values(&self, from: usize, to: usize) -> Vec<u32> {
            let values = self.values.read();
            values[from.min(values.len())..to.min(values.len())].to_vec()
        }
    }

    impl AnyVec for BlockingVec {
        fn version(&self) -> Version {
            Version::ONE
        }

        fn name(&self) -> &str {
            "blocking"
        }

        fn len(&self) -> usize {
            self.values.read().len()
        }

        fn index_type_to_string(&self) -> &'static str {
            <usize as PrintableIndex>::to_string()
        }

        fn region_names(&self) -> Vec<String> {
            Vec::new()
        }

        fn value_type_to_size_of(&self) -> usize {
            size_of::<u32>()
        }

        fn value_type_to_string(&self) -> &'static str {
            short_type_name::<u32>()
        }
    }

    impl TypedVec for BlockingVec {
        type I = usize;
        type T = u32;
    }

    impl ReadableVec<usize, u32> for BlockingVec {
        fn read_into_at(&self, from: usize, to: usize, buf: &mut Vec<u32>) {
            let values = self.values(from, to);
            if self.block_once.swap(false, SeqCst) {
                self.started.wait();
                self.resume.wait();
            }
            buf.extend(values);
        }

        fn for_each_range_dyn_at(&self, from: usize, to: usize, each: &mut dyn FnMut(u32)) {
            self.values(from, to).into_iter().for_each(each);
        }

        fn fold_range_at<B, F: FnMut(B, u32) -> B>(
            &self,
            from: usize,
            to: usize,
            init: B,
            fold: F,
        ) -> B {
            self.values(from, to).into_iter().fold(init, fold)
        }

        fn try_fold_range_at<B, E, F: FnMut(B, u32) -> Result<B, E>>(
            &self,
            from: usize,
            to: usize,
            init: B,
            fold: F,
        ) -> Result<B, E> {
            self.values(from, to).into_iter().try_fold(init, fold)
        }
    }

    #[test]
    fn clear_rejects_an_in_flight_stale_materialization() {
        let source = BlockingVec::new([0, 1]);
        let cached = CachedVec::wrap(source.clone());
        let reader = cached.clone();
        let handle = std::thread::spawn(move || reader.cached());

        source.started.wait();
        source.replace(1, 2);
        cached.clear();
        source.resume.wait();

        assert_eq!(handle.join().unwrap().as_ref(), [0, 2]);
        assert_eq!(cached.cached().as_ref(), [0, 2]);
    }
}
