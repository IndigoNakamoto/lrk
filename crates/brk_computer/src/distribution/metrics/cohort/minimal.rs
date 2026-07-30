use brk_cohort::Filter;
use brk_error::Result;
use brk_indexer::Lengths;
use brk_traversable::Traversable;
use brk_types::{Height, Sats};
use vecdb::{AnyStoredVec, Exit, ReadableVec, Rw, StorageMode};

use crate::{
    distribution::metrics::{
        ActivityMinimal, ImportConfig, OutputsBase, RealizedMinimal, SupplyBase, UnrealizedMinimal,
    },
    price,
};

/// MinimalCohortMetrics: supply, outputs, realized cap/price/mvrv/profit/loss + value_created/destroyed.
///
/// Used for amount_range cohorts.
/// Does NOT implement CohortMetricsBase — standalone, not aggregatable via trait.
#[derive(Traversable)]
pub struct MinimalCohortMetrics<M: StorageMode = Rw> {
    #[traversable(skip)]
    pub filter: Filter,
    pub supply: Box<SupplyBase<M>>,
    pub outputs: Box<OutputsBase<M>>,
    pub activity: Box<ActivityMinimal<M>>,
    pub realized: Box<RealizedMinimal<M>>,
    pub unrealized: Box<UnrealizedMinimal>,
}

impl MinimalCohortMetrics {
    pub(crate) fn forced_import(cfg: &ImportConfig) -> Result<Self> {
        let realized = RealizedMinimal::forced_import(cfg)?;
        let unrealized = UnrealizedMinimal::new(cfg, &realized.price.ppm);

        Ok(Self {
            filter: cfg.filter.clone(),
            supply: Box::new(SupplyBase::forced_import(cfg)?),
            outputs: Box::new(OutputsBase::forced_import(cfg)?),
            activity: Box::new(ActivityMinimal::forced_import(cfg)?),
            realized: Box::new(realized),
            unrealized: Box::new(unrealized),
        })
    }

    pub(crate) fn min_stateful_len(&self) -> usize {
        self.supply
            .min_len()
            .min(self.outputs.min_len())
            .min(self.activity.min_len())
            .min(self.realized.min_stateful_len())
    }

    pub(crate) fn collect_all_vecs_mut(&mut self) -> Vec<&mut dyn AnyStoredVec> {
        let mut vecs: Vec<&mut dyn AnyStoredVec> = Vec::new();
        vecs.extend(self.supply.collect_vecs_mut());
        vecs.extend(self.outputs.collect_vecs_mut());
        vecs.extend(self.activity.collect_vecs_mut());
        vecs.extend(self.realized.collect_vecs_mut());
        vecs
    }

    /// Aggregate Minimal-tier metrics from other MinimalCohortMetrics sources.
    pub(crate) fn compute_from_sources(
        &mut self,
        starting_lengths: &Lengths,
        others: &[&MinimalCohortMetrics],
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

    pub(crate) fn compute_rest_part1(
        &mut self,
        prices: &price::Vecs,
        starting_lengths: &Lengths,
        exit: &Exit,
    ) -> Result<()> {
        self.supply.compute(prices, starting_lengths.height, exit)?;
        self.activity
            .compute_rest_part1(prices, starting_lengths, exit)?;
        Ok(())
    }

    pub(crate) fn compute_rest_part2(
        &mut self,
        prices: &price::Vecs,
        starting_lengths: &Lengths,
        all_supply_sats: &impl ReadableVec<Height, Sats>,
        exit: &Exit,
    ) -> Result<()> {
        self.realized.compute_rest_part2(
            prices,
            starting_lengths,
            &self.supply.total.btc.height,
            exit,
        )?;

        self.supply
            .compute_dominance(starting_lengths.height, all_supply_sats, exit)?;

        Ok(())
    }
}
