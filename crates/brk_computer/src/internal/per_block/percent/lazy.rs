use brk_traversable::Traversable;
use brk_types::{StoredF32, Version};
use derive_more::{Deref, DerefMut};
use vecdb::{ReadableCloneableVec, UnaryTransform};

use crate::internal::{FixedRatio, LazyPerBlock, Percent, PercentPerBlock};

/// Fully lazy variant of `PercentPerBlock` — no stored vecs.
///
/// Raw values are lazily derived from a source `PercentPerBlock` via a unary transform,
/// and ratio/percent float views are chained from them.
#[derive(Clone, Deref, DerefMut, Traversable)]
#[traversable(transparent)]
pub struct LazyPercentPerBlock<B: FixedRatio>(
    pub Percent<LazyPerBlock<B, B>, LazyPerBlock<StoredF32, B>>,
);

impl<B: FixedRatio> LazyPercentPerBlock<B> {
    /// Create from a stored `PercentPerBlock` source via a same-unit unary transform.
    pub(crate) fn from_percent<F: UnaryTransform<B, B>>(
        name: &str,
        version: Version,
        source: &PercentPerBlock<B>,
    ) -> Self {
        let raw = LazyPerBlock::from_computed::<F>(
            &format!("{name}_{}", B::SUFFIX),
            version,
            source.raw.height.read_only_boxed_clone(),
            &source.raw,
        );

        let ratio =
            LazyPerBlock::from_lazy::<B::ToRatio, B>(&format!("{name}_ratio"), version, &raw);

        let percent = LazyPerBlock::from_lazy::<B::ToPercent, B>(name, version, &raw);

        Self(Percent {
            raw,
            ratio,
            percent,
        })
    }
}
