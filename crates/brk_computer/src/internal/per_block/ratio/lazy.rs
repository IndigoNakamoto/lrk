use brk_traversable::Traversable;
use brk_types::{StoredF32, Version};
use schemars::JsonSchema;
use vecdb::{ReadableCloneableVec, UnaryTransform};

use crate::internal::{FixedRatio, LazyPerBlock, NumericValue, PerBlock};

/// Fully lazy variant of `RatioPerBlock` derived from one per-block source.
#[derive(Clone, Traversable)]
pub struct LazyRatioPerBlock<R, S = R>
where
    R: FixedRatio,
    S: NumericValue + JsonSchema,
{
    pub ppm: LazyPerBlock<R, S>,
    pub ratio: LazyPerBlock<StoredF32, R>,
}

impl<R, S> LazyRatioPerBlock<R, S>
where
    R: FixedRatio,
    S: NumericValue + JsonSchema,
{
    pub(crate) fn from_source<F>(name: &str, version: Version, source: &PerBlock<S>) -> Self
    where
        F: UnaryTransform<S, R>,
    {
        let ppm = LazyPerBlock::from_computed::<F>(
            &format!("{name}_{}", R::SUFFIX),
            version,
            source.height.read_only_boxed_clone(),
            source,
        );
        let ratio = LazyPerBlock::from_lazy::<R::ToRatio, S>(name, version, &ppm);

        Self { ppm, ratio }
    }
}
