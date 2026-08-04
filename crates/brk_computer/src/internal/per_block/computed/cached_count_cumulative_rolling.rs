use brk_error::Result;
use brk_traversable::Traversable;
use brk_types::{Height, StoredU16, StoredU64, Version};
use vecdb::{
    AnyStoredVec, AnyVec, CachedVec, Database, EagerVec, ImportableVec, LazyVecFrom1, PcoVec, Rw,
    StorageMode, WritableVec,
};

use crate::{
    indexes,
    internal::{
        CachedBlockCountReader, CachedWindowStartVec, Identity, LazyPerBlock,
        LazyRollingAvgsFromHeight, LazyRollingSumsFromHeight, StoredU16ToStoredU64, Windows,
    },
};

#[derive(Traversable)]
pub struct CachedCountPerBlockCumulativeRolling<M: StorageMode = Rw> {
    pub block: LazyVecFrom1<Height, StoredU64, Height, StoredU16>,
    pub cumulative: LazyPerBlock<StoredU64>,
    pub sum: LazyRollingSumsFromHeight<StoredU64>,
    pub average: LazyRollingAvgsFromHeight<StoredU64>,
    #[traversable(hidden)]
    source: CachedVec<M::Stored<EagerVec<PcoVec<Height, StoredU16>>>>,
    #[traversable(skip)]
    cached_cumulative: CachedBlockCountReader,
}

impl CachedCountPerBlockCumulativeRolling {
    pub(crate) fn forced_import(
        db: &Database,
        name: &str,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&CachedWindowStartVec>,
    ) -> Result<Self> {
        let source = CachedVec::wrap(EagerVec::forced_import(db, name, version)?);
        let cached_cumulative = CachedBlockCountReader::new(source.read_only_cached_boxed_clone());
        let block = LazyVecFrom1::transformed::<StoredU16ToStoredU64>(
            name,
            version,
            source.read_only_boxed_clone(),
        );
        let cumulative = LazyPerBlock::from_uncached_height_source::<Identity<StoredU64>, _>(
            &format!("{name}_cumulative"),
            version,
            cached_cumulative.clone(),
            indexes,
        );
        let sum = LazyRollingSumsFromHeight::new_uncached(
            &format!("{name}_sum"),
            version,
            &cached_cumulative,
            cached_starts,
            indexes,
        );
        let average = LazyRollingAvgsFromHeight::new(
            &format!("{name}_average"),
            version,
            &cached_cumulative,
            cached_starts,
            indexes,
        );

        Ok(Self {
            block,
            cumulative,
            sum,
            average,
            source,
            cached_cumulative,
        })
    }

    #[inline(always)]
    pub(crate) fn cached_cumulative(&self) -> CachedBlockCountReader {
        self.cached_cumulative.clone()
    }

    #[inline(always)]
    pub(crate) fn min_stateful_len(&self) -> usize {
        self.source.len()
    }

    #[inline(always)]
    pub(crate) fn push_block(&mut self, value: StoredU64) {
        let value = u64::from(value);
        debug_assert!(u16::try_from(value).is_ok());
        self.source.inner.push(StoredU16::new(value as u16));
    }

    pub(crate) fn validate_and_truncate(
        &mut self,
        dependency_version: Version,
        at_height: Height,
    ) -> Result<()> {
        self.source
            .inner
            .validate_and_truncate(dependency_version, at_height)?;
        self.source.clear();
        Ok(())
    }

    pub(crate) fn truncate_if_needed_at(&mut self, len: usize) -> Result<()> {
        self.source.inner.truncate_if_needed_at(len)?;
        self.source.clear();
        Ok(())
    }

    pub(crate) fn write(&mut self) -> Result<()> {
        self.source.inner.write()?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use brk_indexer::Indexer;
    use brk_traversable::Traversable;
    use vecdb::{AnyVec, ReadableVec};

    use super::*;
    use crate::blocks::LookbackVecs;

    #[test]
    fn owns_retains_and_refreshes_its_compact_source() {
        let suffix = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!(
            "brk-cached-count-owner-{}-{suffix}",
            std::process::id()
        ));

        let indexer = Indexer::forced_import(&path).unwrap();
        let indexes = indexes::Vecs::forced_import(&path, Version::ONE, &indexer).unwrap();
        let lookback = LookbackVecs::new(
            Version::ONE,
            indexes.timestamp.monotonic.read_only_cached_boxed_clone(),
        );
        let cached_starts = lookback.cached_window_starts();
        let db = Database::open(&path.join("counts")).unwrap();
        let mut count = CachedCountPerBlockCumulativeRolling::forced_import(
            &db,
            "count",
            Version::ONE,
            &indexes,
            &cached_starts,
        )
        .unwrap();

        count
            .validate_and_truncate(Version::ONE, Height::ZERO)
            .unwrap();
        for value in [1_u64, 2, 3] {
            count.push_block(StoredU64::from(value));
        }
        count.write().unwrap();

        assert_eq!(
            count.block.collect_range_at(0, 3),
            [1_u64, 2, 3].map(StoredU64::from)
        );
        assert_eq!(
            count.cumulative.height.collect_range_at(0, 3),
            [1_u64, 3, 6].map(StoredU64::from)
        );

        let source_regions = count.source.region_names();
        assert!(!source_regions.is_empty());
        let exportable_regions = count
            .iter_any_exportable()
            .flat_map(AnyVec::region_names)
            .collect::<Vec<_>>();
        assert!(
            source_regions
                .iter()
                .all(|region| exportable_regions.contains(region))
        );
        assert!(
            count
                .iter_any_visible()
                .flat_map(AnyVec::region_names)
                .next()
                .is_none()
        );

        count.truncate_if_needed_at(0).unwrap();
        for value in [4_u64, 5, 6] {
            count.push_block(StoredU64::from(value));
        }
        count.write().unwrap();

        assert_eq!(
            count.block.collect_range_at(0, 3),
            [4_u64, 5, 6].map(StoredU64::from)
        );
        assert_eq!(
            count.cumulative.height.collect_range_at(0, 3),
            [4_u64, 9, 15].map(StoredU64::from)
        );

        drop(count);
        drop(db);
        drop(lookback);
        drop(indexes);
        drop(indexer);
        std::fs::remove_dir_all(path).unwrap();
    }
}
