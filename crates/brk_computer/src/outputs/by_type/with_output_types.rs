//! Generic `all` + per-`OutputType` container (12 output types, including
//! op_return).

use brk_cohort::{ByAddrType, ByType, Filter};
use brk_error::Result;
use brk_traversable::Traversable;
use brk_types::{Height, PartsPerMillion32, StoredU64, Version};
use vecdb::{
    AnyStoredVec, AnyVec, CachedBoxedVec, CachedReadableVec, Database, ReadableCloneableVec,
    ReadableVec, TypedVec, WritableVec,
};

use crate::{
    indexes,
    internal::{
        CachedBlockCountReader, CachedCountPerBlockCumulativeRolling, CachedWindowStartVec,
        LazyPerBlockCumulativeRolling, LazyPercentCumulativeRolling, PerBlockCumulativeRolling,
        RatioU64, Windows,
    },
};

pub(super) fn identity(_: Height, value: StoredU64) -> StoredU64 {
    value
}

/// `all` aggregate plus per-`OutputType` breakdown across all 12 output
/// types (spendable + op_return).
#[derive(Traversable)]
pub struct WithOutputTypes<V> {
    pub all: LazyPerBlockCumulativeRolling<StoredU64>,
    #[traversable(skip)]
    cached_all: CachedBoxedVec<Height, StoredU64>,
    #[traversable(flatten)]
    pub by_type: ByType<V>,
}

impl<V> WithOutputTypes<V> {
    fn new<S>(
        all_name: &str,
        version: Version,
        (all_source, all_transform): (S, fn(Height, StoredU64) -> StoredU64),
        by_type: ByType<V>,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&CachedWindowStartVec>,
    ) -> Self
    where
        S: TypedVec<I = Height, T = StoredU64>
            + ReadableVec<Height, StoredU64>
            + CachedReadableVec<Height, StoredU64>
            + Clone
            + 'static,
    {
        let cached_all = all_source.cached_boxed_clone();
        Self {
            all: LazyPerBlockCumulativeRolling::from_uncached_indexed_source(
                all_name,
                version,
                &all_source,
                all_transform,
                cached_starts,
                indexes,
            ),
            cached_all,
            by_type,
        }
    }

    fn min_stateful_len_with(&self, len: impl Fn(&V) -> usize) -> usize {
        self.by_type.iter().map(len).min().unwrap()
    }

    fn try_for_each_type_mut(&mut self, mut apply: impl FnMut(&mut V) -> Result<()>) -> Result<()> {
        self.by_type.iter_mut().try_for_each(&mut apply)
    }

    fn push_block_with(&mut self, per_type: &[u64; brk_types::OutputType::COUNT], mut push: impl FnMut(&mut V, StoredU64)) {
        for (output_type, vec) in self.by_type.iter_typed_mut() {
            push(vec, StoredU64::from(per_type[output_type as usize]));
        }
    }

    pub(crate) fn lazy_share(
        &self,
        name: &str,
        version: Version,
        numerator: &(impl ReadableCloneableVec<Height, StoredU64> + 'static),
        cached_starts: &Windows<&CachedWindowStartVec>,
        indexes: &indexes::Vecs,
    ) -> LazyPercentCumulativeRolling<PartsPerMillion32> {
        LazyPercentCumulativeRolling::from_cumulative_ratio::<
            StoredU64,
            StoredU64,
            RatioU64<PartsPerMillion32>,
        >(
            name,
            version,
            numerator,
            self.cached_all.clone(),
            cached_starts,
            indexes,
        )
    }
}

impl WithOutputTypes<CachedCountPerBlockCumulativeRolling> {
    pub(crate) fn forced_import_counts<S>(
        db: &Database,
        all_name: &str,
        per_type_name: impl Fn(&str) -> String,
        version: Version,
        all: (S, fn(Height, StoredU64) -> StoredU64),
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&CachedWindowStartVec>,
    ) -> Result<Self>
    where
        S: TypedVec<I = Height, T = StoredU64>
            + ReadableVec<Height, StoredU64>
            + CachedReadableVec<Height, StoredU64>
            + Clone
            + 'static,
    {
        let by_type = ByType::try_new(|_, name| {
            CachedCountPerBlockCumulativeRolling::forced_import(
                db,
                &per_type_name(name),
                version,
                indexes,
                cached_starts,
            )
        })?;
        Ok(Self::new(
            all_name,
            version,
            all,
            by_type,
            indexes,
            cached_starts,
        ))
    }

    pub(crate) fn min_stateful_len(&self) -> usize {
        self.min_stateful_len_with(|vec| vec.min_stateful_len())
    }

    pub(crate) fn cached_addr_type_counts(&self) -> ByAddrType<CachedBlockCountReader> {
        ByAddrType::new(|filter| {
            let Filter::Type(output_type) = filter else {
                unreachable!()
            };
            self.by_type.get(output_type).cached_cumulative()
        })
    }

    pub(crate) fn lazy_shares(
        &self,
        version: Version,
        name: impl Fn(&str) -> String,
        cached_starts: &Windows<&CachedWindowStartVec>,
        indexes: &indexes::Vecs,
    ) -> ByType<LazyPercentCumulativeRolling<PartsPerMillion32>> {
        ByType::new(|filter, type_name| {
            let Filter::Type(output_type) = filter else {
                unreachable!()
            };
            let numerator = self.by_type.get(output_type).cached_cumulative();
            self.lazy_share(
                &name(type_name),
                version,
                &numerator,
                cached_starts,
                indexes,
            )
        })
    }

    pub(crate) fn write(&mut self) -> Result<()> {
        self.try_for_each_type_mut(|vec| vec.write())
    }

    pub(crate) fn validate_and_truncate(
        &mut self,
        dependency_version: Version,
        at_height: Height,
    ) -> Result<()> {
        self.try_for_each_type_mut(|vec| vec.validate_and_truncate(dependency_version, at_height))
    }

    pub(crate) fn truncate_if_needed_at(&mut self, len: usize) -> Result<()> {
        self.try_for_each_type_mut(|vec| vec.truncate_if_needed_at(len))
    }

    #[inline]
    pub(crate) fn push_block(&mut self, per_type: &[u64; brk_types::OutputType::COUNT]) {
        self.push_block_with(per_type, |vec, value| vec.push_block(value));
    }
}

impl WithOutputTypes<PerBlockCumulativeRolling<StoredU64>> {
    pub(crate) fn forced_import<S>(
        db: &Database,
        all_name: &str,
        per_type_name: impl Fn(&str) -> String,
        version: Version,
        all: (S, fn(Height, StoredU64) -> StoredU64),
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&CachedWindowStartVec>,
    ) -> Result<Self>
    where
        S: TypedVec<I = Height, T = StoredU64>
            + ReadableVec<Height, StoredU64>
            + CachedReadableVec<Height, StoredU64>
            + Clone
            + 'static,
    {
        let by_type = ByType::try_new(|_, name| {
            PerBlockCumulativeRolling::forced_import(
                db,
                &per_type_name(name),
                version,
                indexes,
                cached_starts,
            )
        })?;
        Ok(Self::new(
            all_name,
            version,
            all,
            by_type,
            indexes,
            cached_starts,
        ))
    }

    pub(crate) fn min_stateful_len(&self) -> usize {
        self.min_stateful_len_with(|vec| vec.cumulative.height.len())
    }

    pub(crate) fn lazy_shares(
        &self,
        version: Version,
        name: impl Fn(&str) -> String,
        cached_starts: &Windows<&CachedWindowStartVec>,
        indexes: &indexes::Vecs,
    ) -> ByType<LazyPercentCumulativeRolling<PartsPerMillion32>> {
        ByType::new(|filter, type_name| {
            let Filter::Type(output_type) = filter else {
                unreachable!()
            };
            self.lazy_share(
                &name(type_name),
                version,
                &self.by_type.get(output_type).cumulative.height,
                cached_starts,
                indexes,
            )
        })
    }

    pub(crate) fn write(&mut self) -> Result<()> {
        self.try_for_each_type_mut(|vec| {
            vec.cumulative.height.write()?;
            Ok(())
        })
    }

    pub(crate) fn validate_and_truncate(
        &mut self,
        dependency_version: Version,
        at_height: Height,
    ) -> Result<()> {
        self.try_for_each_type_mut(|vec| {
            Ok(vec
                .cumulative
                .height
                .validate_and_truncate(dependency_version, at_height)?)
        })
    }

    pub(crate) fn truncate_if_needed_at(&mut self, len: usize) -> Result<()> {
        self.try_for_each_type_mut(|vec| Ok(vec.cumulative.height.truncate_if_needed_at(len)?))
    }

    #[inline]
    pub(crate) fn push_block(&mut self, per_type: &[u64; brk_types::OutputType::COUNT]) {
        self.push_block_with(per_type, |vec, value| vec.push_block(value));
    }
}
