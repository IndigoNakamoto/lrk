use brk_traversable::Traversable;
use brk_types::Version;
use schemars::JsonSchema;
use vecdb::{ReadableCloneableVec, UnaryTransform};

use crate::{
    indexes,
    internal::{
        ComputedVecValue, LazyPerBlock, LazyRollingComplete, NumericValue, PerBlock,
        RollingComplete, WindowStartVec, Windows,
    },
};

/// Lazy analog of `PerBlockRolling<T>`: lazy cumulative + lazy rolling complete.
/// Derived by transforming another metric's cumulative and rolling parts.
/// Zero stored vecs.
#[derive(Clone, Traversable)]
pub struct LazyPerBlockRolling<T, S1T>
where
    T: NumericValue + JsonSchema,
    S1T: ComputedVecValue + JsonSchema,
{
    pub cumulative: LazyPerBlock<T, S1T>,
    #[traversable(flatten)]
    pub rolling: LazyRollingComplete<T, S1T>,
}

impl<T, S1T> LazyPerBlockRolling<T, S1T>
where
    T: NumericValue + JsonSchema + 'static,
    S1T: NumericValue + JsonSchema,
{
    pub(crate) fn from_full_parts<F: UnaryTransform<S1T, T>>(
        name: &str,
        version: Version,
        source_cumulative: &PerBlock<S1T>,
        source_rolling: &RollingComplete<S1T>,
        cached_starts: &Windows<&WindowStartVec>,
        indexes: &indexes::Vecs,
    ) -> Self {
        let cumulative = LazyPerBlock::from_computed::<F>(
            &format!("{name}_cumulative"),
            version,
            source_cumulative.height.read_only_boxed_clone(),
            source_cumulative,
        );

        let rolling = LazyRollingComplete::from_rolling_complete::<F>(
            name,
            version,
            &cumulative.height,
            source_rolling,
            cached_starts,
            indexes,
        );

        Self {
            cumulative,
            rolling,
        }
    }
}
