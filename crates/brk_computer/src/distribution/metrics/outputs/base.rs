use brk_error::Result;
use brk_indexer::Lengths;
use brk_traversable::Traversable;
use brk_types::{PartsPerMillionSigned64, StoredI64, StoredU64, Version};
use vecdb::{AnyStoredVec, AnyVec, Exit, Rw, StorageMode, WritableVec};

use crate::{
    distribution::{
        metrics::ImportConfig,
        state::{CohortState, CostBasisOps, RealizedOps},
    },
    internal::{PerBlockCumulativeRolling, PerBlockWithDeltas},
};

/// Base output metrics: utxo_count + delta.
#[derive(Traversable)]
pub struct OutputsBase<M: StorageMode = Rw> {
    pub unspent_count: PerBlockWithDeltas<StoredU64, StoredI64, PartsPerMillionSigned64, M>,
    pub spent_count: PerBlockCumulativeRolling<StoredU64, M>,
}

impl OutputsBase {
    pub(crate) fn forced_import(cfg: &ImportConfig) -> Result<Self> {
        let v1 = Version::ONE;
        Ok(Self {
            unspent_count: PerBlockWithDeltas::forced_import(
                cfg.db,
                &cfg.name("utxo_count"),
                cfg.version,
                Version::TWO,
                cfg.indexes,
                cfg.cached_starts,
            )?,
            spent_count: cfg.import("spent_utxo_count", v1)?,
        })
    }

    pub(crate) fn min_len(&self) -> usize {
        self.unspent_count
            .height
            .len()
            .min(self.spent_count.block.len())
    }

    #[inline(always)]
    pub(crate) fn push_state(&mut self, state: &CohortState<impl RealizedOps, impl CostBasisOps>) {
        self.unspent_count
            .height
            .push(StoredU64::from(state.supply.utxo_count));
        self.spent_count
            .push_block(StoredU64::from(state.spent_utxo_count));
    }

    pub(crate) fn collect_vecs_mut(&mut self) -> Vec<&mut dyn AnyStoredVec> {
        vec![
            &mut self.unspent_count.height as &mut dyn AnyStoredVec,
            self.spent_count.stored_mut(),
        ]
    }

    pub(crate) fn compute_from_stateful(
        &mut self,
        starting_lengths: &Lengths,
        others: &[&Self],
        exit: &Exit,
    ) -> Result<()> {
        self.unspent_count.height.compute_sum_of_others(
            starting_lengths.height,
            &others
                .iter()
                .map(|v| &v.unspent_count.height)
                .collect::<Vec<_>>(),
            exit,
        )?;
        sum_others!(self, starting_lengths, others, exit; spent_count.cumulative.height);
        Ok(())
    }
}
