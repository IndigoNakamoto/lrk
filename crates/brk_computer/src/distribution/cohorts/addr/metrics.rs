use brk_cohort::Filter;
use brk_error::Result;
use brk_indexer::Lengths;
use brk_traversable::Traversable;
use brk_types::{Height, Sats};
use vecdb::{AnyStoredVec, Exit, ReadableVec, Rw, StorageMode};

use crate::{
    distribution::metrics::{
        ActivityMinimal, ImportConfig, OutputsUnspent, RealizedBase, SupplyBase,
    },
    price,
};

/// Address-balance metrics: holdings plus economically meaningful flows.
///
/// Address cohorts intentionally omit spent-output counts, realized price,
/// MVRV, and NUPL.
#[derive(Traversable)]
pub struct AddrCohortMetrics<M: StorageMode = Rw> {
    #[traversable(skip)]
    pub filter: Filter,
    pub supply: Box<SupplyBase<M>>,
    pub outputs: Box<OutputsUnspent<M>>,
    pub activity: Box<ActivityMinimal<M>>,
    pub realized: Box<RealizedBase<M>>,
}

impl AddrCohortMetrics {
    pub(super) fn forced_import(cfg: &ImportConfig) -> Result<Self> {
        Ok(Self {
            filter: cfg.filter.clone(),
            supply: Box::new(SupplyBase::forced_import(cfg)?),
            outputs: Box::new(OutputsUnspent::forced_import(cfg)?),
            activity: Box::new(ActivityMinimal::forced_import(cfg)?),
            realized: Box::new(RealizedBase::forced_import(cfg)?),
        })
    }

    pub(super) fn min_stateful_len(&self) -> usize {
        self.supply
            .min_len()
            .min(self.outputs.min_len())
            .min(self.activity.min_len())
            .min(self.realized.min_stateful_len())
    }

    pub(super) fn collect_all_vecs_mut(&mut self) -> Vec<&mut dyn AnyStoredVec> {
        let mut vecs = Vec::new();
        vecs.extend(self.supply.collect_vecs_mut());
        vecs.extend(self.outputs.collect_vecs_mut());
        vecs.extend(self.activity.collect_vecs_mut());
        vecs.extend(self.realized.collect_vecs_mut());
        vecs
    }

    pub(super) fn compute_from_sources(
        &mut self,
        starting_lengths: &Lengths,
        others: &[&Self],
        exit: &Exit,
    ) -> Result<()> {
        self.supply.compute_from_stateful(
            starting_lengths,
            &others.iter().map(|v| v.supply.as_ref()).collect::<Vec<_>>(),
            exit,
        )?;
        self.outputs.compute_from_stateful(
            starting_lengths,
            &others
                .iter()
                .map(|v| v.outputs.as_ref())
                .collect::<Vec<_>>(),
            exit,
        )?;
        self.activity.compute_from_stateful(
            starting_lengths,
            &others
                .iter()
                .map(|v| v.activity.as_ref())
                .collect::<Vec<_>>(),
            exit,
        )?;
        self.realized.compute_from_stateful(
            starting_lengths,
            &others
                .iter()
                .map(|v| v.realized.as_ref())
                .collect::<Vec<_>>(),
            exit,
        )?;
        Ok(())
    }

    pub(super) fn compute_rest_part1(
        &mut self,
        prices: &price::Vecs,
        starting_lengths: &Lengths,
        exit: &Exit,
    ) -> Result<()> {
        self.activity
            .compute_rest_part1(prices, starting_lengths, exit)
    }

    pub(super) fn compute_rest_part2(
        &mut self,
        starting_lengths: &Lengths,
        all_supply_sats: &impl ReadableVec<Height, Sats>,
        exit: &Exit,
    ) -> Result<()> {
        self.supply
            .compute_dominance(starting_lengths.height, all_supply_sats, exit)
    }
}
