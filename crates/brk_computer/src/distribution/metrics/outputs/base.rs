use brk_error::Result;
use brk_indexer::Lengths;
use brk_traversable::Traversable;
use brk_types::{PartsPerMillionSigned64, StoredI64, StoredU64, Version};
use derive_more::{Deref, DerefMut};
use vecdb::{AnyStoredVec, AnyVec, Exit, Rw, StorageMode, WritableVec};

use crate::{
    distribution::{
        metrics::ImportConfig,
        state::{CohortState, CostBasisOps, RealizedOps},
    },
    internal::{PerBlockCumulativeRolling, PerBlockWithDeltas},
};

/// Unspent output metrics.
#[derive(Traversable)]
pub struct OutputsUnspent<M: StorageMode = Rw> {
    pub unspent_count: PerBlockWithDeltas<StoredU64, StoredI64, PartsPerMillionSigned64, M>,
}

impl OutputsUnspent {
    pub(crate) fn forced_import(cfg: &ImportConfig) -> Result<Self> {
        Ok(Self {
            unspent_count: PerBlockWithDeltas::forced_import(
                cfg.db,
                &cfg.name("utxo_count"),
                cfg.version,
                Version::TWO,
                cfg.indexes,
                cfg.cached_starts,
            )?,
        })
    }

    pub(crate) fn min_len(&self) -> usize {
        self.unspent_count.height.len()
    }

    #[inline(always)]
    pub(crate) fn push_state(&mut self, state: &CohortState<impl RealizedOps, impl CostBasisOps>) {
        self.unspent_count
            .height
            .push(StoredU64::from(state.supply.utxo_count));
    }

    pub(crate) fn collect_vecs_mut(&mut self) -> Vec<&mut dyn AnyStoredVec> {
        vec![&mut self.unspent_count.height as &mut dyn AnyStoredVec]
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
        Ok(())
    }
}

/// Base output metrics: unspent and spent output counts.
#[derive(Deref, DerefMut, Traversable)]
pub struct OutputsBase<M: StorageMode = Rw> {
    #[deref]
    #[deref_mut]
    #[traversable(flatten)]
    pub unspent: OutputsUnspent<M>,
    pub spent_count: PerBlockCumulativeRolling<StoredU64, M>,
}

impl OutputsBase {
    pub(crate) fn forced_import(cfg: &ImportConfig) -> Result<Self> {
        let v1 = Version::ONE;
        Ok(Self {
            unspent: OutputsUnspent::forced_import(cfg)?,
            spent_count: cfg.import("spent_utxo_count", v1)?,
        })
    }

    pub(crate) fn min_len(&self) -> usize {
        self.unspent.min_len().min(self.spent_count.block.len())
    }

    #[inline(always)]
    pub(crate) fn push_state(&mut self, state: &CohortState<impl RealizedOps, impl CostBasisOps>) {
        self.unspent.push_state(state);
        self.spent_count
            .push_block(StoredU64::from(state.spent_utxo_count));
    }

    pub(crate) fn collect_vecs_mut(&mut self) -> Vec<&mut dyn AnyStoredVec> {
        let mut vecs = self.unspent.collect_vecs_mut();
        vecs.push(self.spent_count.stored_mut());
        vecs
    }

    pub(crate) fn compute_from_stateful(
        &mut self,
        starting_lengths: &Lengths,
        others: &[&Self],
        exit: &Exit,
    ) -> Result<()> {
        let unspent: Vec<_> = others.iter().map(|v| &v.unspent).collect();
        self.unspent
            .compute_from_stateful(starting_lengths, &unspent, exit)?;
        sum_others!(self, starting_lengths, others, exit; spent_count.cumulative.height);
        Ok(())
    }
}
