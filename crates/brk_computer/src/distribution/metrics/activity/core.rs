use brk_error::Result;
use brk_indexer::Lengths;
use brk_traversable::Traversable;
use brk_types::{Bitcoin, StoredF64, Version};
use derive_more::{Deref, DerefMut};
use vecdb::{AnyStoredVec, AnyVec, Exit, Rw, StorageMode};

use crate::{
    distribution::{
        metrics::ImportConfig,
        state::{CohortState, CostBasisOps, RealizedOps},
    },
    internal::{PerBlockCumulativeRolling, ValuePerBlockCumulativeRolling},
    price,
};

use super::ActivityMinimal;

#[derive(Deref, DerefMut, Traversable)]
pub struct ActivityCore<M: StorageMode = Rw> {
    #[deref]
    #[deref_mut]
    #[traversable(flatten)]
    pub minimal: ActivityMinimal<M>,

    pub coindays_destroyed: PerBlockCumulativeRolling<StoredF64, M>,
    #[traversable(wrap = "transfer_volume", rename = "in_profit")]
    pub transfer_volume_in_profit: ValuePerBlockCumulativeRolling<M>,
    #[traversable(wrap = "transfer_volume", rename = "in_loss")]
    pub transfer_volume_in_loss: ValuePerBlockCumulativeRolling<M>,
}

impl ActivityCore {
    pub(crate) fn forced_import(cfg: &ImportConfig) -> Result<Self> {
        let v1 = Version::ONE;
        Ok(Self {
            minimal: ActivityMinimal::forced_import(cfg)?,
            coindays_destroyed: cfg.import("coindays_destroyed", v1)?,
            transfer_volume_in_profit: cfg.import("transfer_volume_in_profit", v1)?,
            transfer_volume_in_loss: cfg.import("transfer_volume_in_loss", v1)?,
        })
    }

    pub(crate) fn min_len(&self) -> usize {
        self.minimal
            .min_len()
            .min(self.coindays_destroyed.block.len())
            .min(self.transfer_volume_in_profit.cumulative.sats.height.len())
            .min(self.transfer_volume_in_loss.cumulative.sats.height.len())
    }

    #[inline(always)]
    pub(crate) fn push_state(&mut self, state: &CohortState<impl RealizedOps, impl CostBasisOps>) {
        self.minimal.push_state(state);
        self.coindays_destroyed
            .push_block(StoredF64::from(Bitcoin::from(state.satdays_destroyed)));
        self.transfer_volume_in_profit
            .push_block_sats(state.realized.sent_in_profit());
        self.transfer_volume_in_loss
            .push_block_sats(state.realized.sent_in_loss());
    }

    pub(crate) fn collect_vecs_mut(&mut self) -> Vec<&mut dyn AnyStoredVec> {
        let mut vecs = self.minimal.collect_vecs_mut();
        vecs.push(self.coindays_destroyed.stored_mut());
        vecs.push(&mut self.transfer_volume_in_profit.inner.cumulative.sats.height);
        vecs.push(&mut self.transfer_volume_in_profit.inner.cumulative.cents.height);
        vecs.push(&mut self.transfer_volume_in_loss.inner.cumulative.sats.height);
        vecs.push(&mut self.transfer_volume_in_loss.inner.cumulative.cents.height);
        vecs
    }

    pub(crate) fn validate_computed_versions(&mut self, _base_version: Version) -> Result<()> {
        Ok(())
    }

    pub(crate) fn compute_from_stateful(
        &mut self,
        starting_lengths: &Lengths,
        others: &[&Self],
        exit: &Exit,
    ) -> Result<()> {
        let minimal_refs: Vec<&ActivityMinimal> = others.iter().map(|o| &o.minimal).collect();
        self.minimal
            .compute_from_stateful(starting_lengths, &minimal_refs, exit)?;

        sum_others!(self, starting_lengths, others, exit; coindays_destroyed.cumulative.height);
        sum_others!(self, starting_lengths, others, exit; transfer_volume_in_profit.cumulative.sats.height);
        sum_others!(self, starting_lengths, others, exit; transfer_volume_in_loss.cumulative.sats.height);

        Ok(())
    }

    pub(crate) fn compute_rest_part1(
        &mut self,
        prices: &price::Vecs,
        starting_lengths: &Lengths,
        exit: &Exit,
    ) -> Result<()> {
        self.minimal
            .compute_rest_part1(prices, starting_lengths, exit)?;
        self.transfer_volume_in_profit
            .compute_rest(starting_lengths.height, prices, exit)?;
        self.transfer_volume_in_loss
            .compute_rest(starting_lengths.height, prices, exit)?;
        Ok(())
    }
}
