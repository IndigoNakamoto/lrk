use brk_traversable::Traversable;
use brk_types::{Height, StoredF32, Version};
use derive_more::{Deref, DerefMut};
use vecdb::{LazyVecFrom1, ReadableCloneableVec, VecValue};

use crate::internal::{FixedRatio, Percent};

/// Fully lazy lightweight percent container with no derived resolutions.
#[derive(Clone, Deref, DerefMut, Traversable)]
#[traversable(transparent)]
#[allow(clippy::type_complexity)]
pub struct LazyPercentVec<B: FixedRatio, S: VecValue>(
    pub Percent<LazyVecFrom1<Height, B, Height, S>, LazyVecFrom1<Height, StoredF32, Height, B>>,
);

impl<B: FixedRatio, S: VecValue> LazyPercentVec<B, S> {
    pub(crate) fn from_indexed_source(
        name: &str,
        version: Version,
        source: &(impl ReadableCloneableVec<Height, S> + 'static),
        compute: fn(Height, S) -> B,
    ) -> Self {
        let ppm = LazyVecFrom1::init(
            &format!("{name}_{}", B::SUFFIX),
            version,
            source.read_only_boxed_clone(),
            compute,
        );
        let ppm_source = ppm.read_only_boxed_clone();
        let ratio = LazyVecFrom1::transformed::<B::ToRatio>(
            &format!("{name}_ratio"),
            version,
            ppm_source.clone(),
        );
        let percent = LazyVecFrom1::transformed::<B::ToPercent>(name, version, ppm_source);

        Self(Percent {
            ppm,
            ratio,
            percent,
        })
    }
}
