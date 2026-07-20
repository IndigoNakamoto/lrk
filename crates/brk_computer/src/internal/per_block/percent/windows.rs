use brk_error::Result;
use brk_traversable::Traversable;
use brk_types::{Height, Version};
use derive_more::{Deref, DerefMut};
use vecdb::{BinaryTransform, Database, Exit, ReadableVec, Rw, StorageMode, VecValue};

use crate::{
    indexes,
    internal::{FixedRatio, PercentPerBlock, Windows},
};

/// 4 rolling window vecs (24h, 1w, 1m, 1y), each storing fixed-point values
/// with lazy ratio and percent float views.
#[derive(Deref, DerefMut, Traversable)]
#[traversable(transparent)]
pub struct PercentRollingWindows<B: FixedRatio, M: StorageMode = Rw>(
    pub Windows<PercentPerBlock<B, M>>,
);

impl<B: FixedRatio> PercentRollingWindows<B> {
    pub(crate) fn forced_import(
        db: &Database,
        name: &str,
        version: Version,
        indexes: &indexes::Vecs,
    ) -> Result<Self> {
        Ok(Self(Windows::try_from_fn(|suffix| {
            PercentPerBlock::forced_import(db, &format!("{name}_{suffix}"), version, indexes)
        })?))
    }

    pub(crate) fn compute_binary<S1T, S2T, F, R1, R2>(
        &mut self,
        max_from: Height,
        sources1: [&R1; 4],
        sources2: [&R2; 4],
        exit: &Exit,
    ) -> Result<()>
    where
        S1T: VecValue,
        S2T: VecValue,
        R1: ReadableVec<Height, S1T>,
        R2: ReadableVec<Height, S2T>,
        F: BinaryTransform<S1T, S2T, B>,
    {
        for ((target, s1), s2) in self
            .0
            .as_mut_array()
            .into_iter()
            .zip(sources1)
            .zip(sources2)
        {
            target.compute_binary::<S1T, S2T, F>(max_from, s1, s2, exit)?;
        }
        Ok(())
    }
}
