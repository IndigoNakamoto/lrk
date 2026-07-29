use brk_error::Result;
use brk_traversable::Traversable;
use brk_types::{Height, Sats, Version};
use derive_more::{Deref, DerefMut};
use vecdb::{Database, Exit, ReadableVec, Rw, StorageMode, VecIndex, VecValue};

use crate::{
    indexes,
    internal::{
        RollingDistributionValuePerBlock, ValuePerBlockCumulativeRolling, WindowStartVec,
        WindowStarts, Windows,
    },
    price,
};

#[derive(Deref, DerefMut, Traversable)]
pub struct ValuePerBlockFull<M: StorageMode = Rw> {
    #[deref]
    #[deref_mut]
    #[traversable(flatten)]
    pub inner: ValuePerBlockCumulativeRolling<M>,
    #[traversable(flatten)]
    pub distribution: RollingDistributionValuePerBlock<M>,
}

const VERSION: Version = Version::TWO;

impl ValuePerBlockFull {
    pub(crate) fn forced_import(
        db: &Database,
        name: &str,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
    ) -> Result<Self> {
        let v = version + VERSION;

        let inner =
            ValuePerBlockCumulativeRolling::forced_import(db, name, v, indexes, cached_starts)?;
        let distribution = RollingDistributionValuePerBlock::forced_import(db, name, v, indexes)?;

        Ok(Self {
            inner,
            distribution,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn compute_from_indexes<A, B>(
        &mut self,
        max_from: Height,
        windows: &WindowStarts<'_>,
        prices: &price::Vecs,
        first_indexes: &impl ReadableVec<Height, A>,
        indexes_count: &impl ReadableVec<Height, B>,
        source: &impl ReadableVec<A, Sats>,
        exit: &Exit,
    ) -> Result<()>
    where
        A: VecIndex + VecValue,
        B: VecValue,
        usize: From<B>,
    {
        self.inner.compute_from_indexes(
            max_from,
            prices,
            first_indexes,
            indexes_count,
            source,
            exit,
        )?;
        self.compute_distribution(max_from, windows, exit)
    }

    fn compute_distribution(
        &mut self,
        max_from: Height,
        windows: &WindowStarts<'_>,
        exit: &Exit,
    ) -> Result<()> {
        self.distribution.compute(
            max_from,
            windows,
            &self.inner.block.sats,
            &self.inner.block.cents,
            exit,
        )?;

        Ok(())
    }
}
