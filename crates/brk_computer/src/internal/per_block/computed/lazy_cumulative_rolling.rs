//! Lazy counterpart to `PerBlockCumulativeRolling`.

use brk_traversable::Traversable;
use brk_types::{Height, Version};
use schemars::JsonSchema;
use vecdb::{ReadableCloneableVec, VecValue};

use crate::{
    indexes,
    internal::{
        Identity, LazyPerBlock, LazyPreviousDeltaVec, LazyRollingAvgsFromHeight,
        LazyRollingSumsFromHeight, NumericValue, PerBlock, WindowStartVec, Windows,
    },
};

pub(super) fn lazy_parts<T>(
    name: &str,
    version: Version,
    cumulative: &(impl ReadableCloneableVec<Height, T> + 'static),
    cached_starts: &Windows<&WindowStartVec>,
    indexes: &indexes::Vecs,
) -> (
    LazyPreviousDeltaVec<Height, T>,
    LazyRollingSumsFromHeight<T>,
    LazyRollingAvgsFromHeight<T>,
)
where
    T: NumericValue + JsonSchema,
{
    (
        LazyPreviousDeltaVec::new(name, version, cumulative.read_only_boxed_clone()),
        LazyRollingSumsFromHeight::new(
            &format!("{name}_sum"),
            version,
            cumulative,
            cached_starts,
            indexes,
        ),
        LazyRollingAvgsFromHeight::new(
            &format!("{name}_average"),
            version,
            cumulative,
            cached_starts,
            indexes,
        ),
    )
}

#[derive(Clone, Traversable)]
pub struct LazyPerBlockCumulativeRolling<T>
where
    T: NumericValue + JsonSchema,
{
    pub block: LazyPreviousDeltaVec<Height, T>,
    pub cumulative: LazyPerBlock<T>,
    pub sum: LazyRollingSumsFromHeight<T>,
    pub average: LazyRollingAvgsFromHeight<T>,
}

impl<T> LazyPerBlockCumulativeRolling<T>
where
    T: NumericValue + JsonSchema,
{
    fn from_cumulative(
        name: &str,
        version: Version,
        cumulative: LazyPerBlock<T>,
        cached_starts: &Windows<&WindowStartVec>,
        indexes: &indexes::Vecs,
    ) -> Self {
        let (block, sum, average) =
            lazy_parts(name, version, &cumulative.height, cached_starts, indexes);

        Self {
            block,
            cumulative,
            sum,
            average,
        }
    }

    pub(crate) fn from_source(
        name: &str,
        version: Version,
        source: &PerBlock<T>,
        cached_starts: &Windows<&WindowStartVec>,
        indexes: &indexes::Vecs,
    ) -> Self {
        let cumulative = LazyPerBlock::from_computed::<Identity<T>>(
            &format!("{name}_cumulative"),
            version,
            source.height.read_only_boxed_clone(),
            source,
        );

        Self::from_cumulative(name, version, cumulative, cached_starts, indexes)
    }

    pub(crate) fn from_indexed_source<S>(
        name: &str,
        version: Version,
        source: &(impl ReadableCloneableVec<Height, S> + 'static),
        compute_cumulative: fn(Height, S) -> T,
        cached_starts: &Windows<&WindowStartVec>,
        indexes: &indexes::Vecs,
    ) -> Self
    where
        S: VecValue,
    {
        let cumulative = LazyPerBlock::from_indexed_source(
            &format!("{name}_cumulative"),
            version,
            source,
            compute_cumulative,
            indexes,
        );

        Self::from_cumulative(name, version, cumulative, cached_starts, indexes)
    }
}
