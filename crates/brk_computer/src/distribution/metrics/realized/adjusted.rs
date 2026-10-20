use brk_error::Result;
use brk_indexer::Lengths;
use brk_traversable::Traversable;
use brk_types::{Cents, Height, StoredF64, Version};
use vecdb::{Exit, ReadableVec, Rw, StorageMode};

use crate::{
    distribution::metrics::ImportConfig,
    internal::{PerBlockCumulativeRolling, RatioCents64, RollingWindows},
};

#[derive(Traversable)]
pub struct AdjustedSopr<M: StorageMode = Rw> {
    pub ratio: RollingWindows<StoredF64, M>,
    pub transfer_volume: PerBlockCumulativeRolling<Cents, M>,
    pub value_destroyed: PerBlockCumulativeRolling<Cents, M>,
}

impl AdjustedSopr {
    pub(crate) fn forced_import(cfg: &ImportConfig) -> Result<Self> {
        Ok(Self {
            ratio: cfg.import("asopr", Version::ONE)?,
            transfer_volume: cfg.import("adj_value_created", Version::ONE)?,
            value_destroyed: cfg.import("adj_value_destroyed", Version::ONE)?,
        })
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn compute_rest_part2(
        &mut self,
        starting_lengths: &Lengths,
        base_transfer_volume: &impl ReadableVec<Height, Cents>,
        base_value_destroyed: &impl ReadableVec<Height, Cents>,
        under_1h_transfer_volume: &impl ReadableVec<Height, Cents>,
        under_1h_value_destroyed: &impl ReadableVec<Height, Cents>,
        exit: &Exit,
    ) -> Result<()> {
        // `all - under_1h` is non-negative on a clean chain. On Litecoin the
        // `all` cumulative can sit at `Cents::MAX_FINITE` after a saturated
        // add while `under_1h` keeps growing, so saturate to zero instead of
        // aborting the whole compute pass (which would pin the served tip).
        self.transfer_volume.cumulative.height.compute_transform2(
            starting_lengths.height,
            base_transfer_volume,
            under_1h_transfer_volume,
            |(i, base, under_1h, ..)| (i, base.saturating_sub(under_1h)),
            exit,
        )?;
        self.value_destroyed.cumulative.height.compute_transform2(
            starting_lengths.height,
            base_value_destroyed,
            under_1h_value_destroyed,
            |(i, base, under_1h, ..)| (i, base.saturating_sub(under_1h)),
            exit,
        )?;

        for ((sopr, tv), vd) in self
            .ratio
            .as_mut_array()
            .into_iter()
            .zip(self.transfer_volume.sum.as_array())
            .zip(self.value_destroyed.sum.as_array())
        {
            sopr.compute_binary::<Cents, Cents, RatioCents64>(
                starting_lengths.height,
                &tv.height,
                &vd.height,
                exit,
            )?;
        }

        Ok(())
    }
}
