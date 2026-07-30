use brk_traversable::Traversable;
use brk_types::{Height, Version};
use derive_more::{Deref, DerefMut};
use schemars::JsonSchema;
use vecdb::{DeltaAvg, LazyDeltaVec, ReadOnlyClone, ReadableCloneableVec, UnaryTransform};

use crate::{
    indexes,
    internal::{FixedRatio, NumericValue, PercentRollingWindows, WindowStartVec, Windows},
};

use super::LazyPercentPerBlock;

/// Fully lazy rolling percent windows — 4 windows (24h, 1w, 1m, 1y),
/// each with lazy PPM + lazy ratio/percent float views.
///
/// No stored vecs. All values are derived from one source.
#[derive(Clone, Deref, DerefMut, Traversable)]
#[traversable(transparent)]
pub struct LazyPercentRollingWindows<B: FixedRatio>(pub Windows<LazyPercentPerBlock<B>>);

impl<B: FixedRatio> LazyPercentRollingWindows<B> {
    /// Rolling percentages derived lazily from a single cumulative source.
    pub(crate) fn from_cumulative_average<T>(
        name: &str,
        version: Version,
        cumulative: &(impl ReadableCloneableVec<Height, T> + 'static),
        cached_starts: &Windows<&WindowStartVec>,
        indexes: &indexes::Vecs,
    ) -> Self
    where
        T: NumericValue + JsonSchema,
    {
        let cumulative_source = cumulative.read_only_boxed_clone();

        Self(cached_starts.map_with_suffix(|suffix, cached_start| {
            let full_name = format!("{name}_{suffix}");
            let cached = cached_start.read_only_clone();
            let starts_version = cached.version();
            let average = LazyDeltaVec::<Height, T, B, DeltaAvg>::new(
                &format!("{full_name}_{}_source", B::SUFFIX),
                version,
                cumulative_source.clone(),
                starts_version,
                move || cached.cached(),
            );

            LazyPercentPerBlock::from_indexed_source(
                &full_name,
                version,
                &average,
                |_, ratio| ratio,
                indexes,
            )
        }))
    }

    /// Create from a stored source via a same-unit unary transform.
    pub(crate) fn from_rolling<F: UnaryTransform<B, B>>(
        name: &str,
        version: Version,
        source: &PercentRollingWindows<B>,
    ) -> Self {
        Self(source.0.map_with_suffix(|suffix, source_window| {
            LazyPercentPerBlock::from_percent::<F>(
                &format!("{name}_{suffix}"),
                version,
                source_window,
            )
        }))
    }
}
