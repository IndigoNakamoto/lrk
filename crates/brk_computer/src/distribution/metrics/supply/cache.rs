use brk_types::{Height, Sats};
use vecdb::{
    CachedBoxedVec, CachedReadableVec, CachedVec, EagerVec, PcoVec, PcodecStrategy, ReadOnlyClone,
    ReadOnlyCompressedVec,
};

/// Pinned in-memory snapshot of the all-cohort supply.
///
/// Every cohort dominance vec shares this cache. It intentionally bypasses the
/// global cache budget because evicting it would make each lazy read hit disk.
#[derive(Clone)]
pub(crate) struct AllSupplyCache {
    cache: CachedVec<ReadOnlyCompressedVec<Height, Sats, PcodecStrategy<Sats>>>,
}

impl AllSupplyCache {
    pub(crate) fn new(source: &EagerVec<PcoVec<Height, Sats>>) -> Self {
        Self {
            cache: CachedVec::wrap(source.read_only_clone()),
        }
    }

    pub(crate) fn cached_boxed_clone(&self) -> CachedBoxedVec<Height, Sats> {
        self.cache.cached_boxed_clone()
    }

    pub(crate) fn clear(&self) {
        self.cache.clear();
    }
}

#[cfg(test)]
mod tests {
    use brk_types::Version;
    use vecdb::{AnyStoredVec, Database, ImportableVec, WritableVec};

    use super::*;

    #[test]
    fn clear_refreshes_a_same_length_rewrite() {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "brk-all-supply-cache-{}-{suffix}",
            std::process::id()
        ));
        let db = Database::open(&path).unwrap();
        let mut source: EagerVec<PcoVec<Height, Sats>> =
            EagerVec::forced_import(&db, "supply", Version::ONE).unwrap();

        source.push(Sats::new(10));
        source.push(Sats::new(20));
        source.write().unwrap();

        let cache = AllSupplyCache::new(&source);
        let reader = cache.cached_boxed_clone();
        assert_eq!(&*reader.cached(), &[Sats::new(10), Sats::new(20)]);

        source.truncate_if_needed_at(1).unwrap();
        source.push(Sats::new(30));
        source.write().unwrap();

        assert_eq!(&*reader.cached(), &[Sats::new(10), Sats::new(20)]);
        cache.clear();
        assert_eq!(&*reader.cached(), &[Sats::new(10), Sats::new(30)]);

        drop(reader);
        drop(cache);
        drop(source);
        drop(db);
        std::fs::remove_dir_all(path).unwrap();
    }
}
