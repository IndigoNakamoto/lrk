use brk_cohort::Filter;
use brk_error::Result;
use brk_indexer::Lengths;
use brk_traversable::Traversable;
use vecdb::{AnyStoredVec, Exit, Rw, StorageMode};

use crate::{
    distribution::metrics::{
        ActivityMinimal, AllSupplyCache, ImportConfig, OutputsBase, RealizedMinimal, SupplyCore,
        UnrealizedBasic,
    },
    price,
};

/// TypeCohortMetrics: supply(core), outputs(base), realized(minimal), unrealized(basic).
///
/// Used for type_ cohorts (p2pkh, p2sh, etc.).
#[derive(Traversable)]
pub struct TypeCohortMetrics<M: StorageMode = Rw> {
    #[traversable(skip)]
    pub filter: Filter,
    pub supply: Box<SupplyCore<M>>,
    pub outputs: Box<OutputsBase<M>>,
    pub activity: Box<ActivityMinimal<M>>,
    pub realized: Box<RealizedMinimal<M>>,
    pub unrealized: Box<UnrealizedBasic<M>>,
}

impl TypeCohortMetrics {
    pub(crate) fn forced_import(cfg: &ImportConfig, all_supply: &AllSupplyCache) -> Result<Self> {
        let realized = RealizedMinimal::forced_import(cfg)?;
        let unrealized = UnrealizedBasic::forced_import(cfg, &realized.price.ppm)?;

        Ok(Self {
            filter: cfg.filter.clone(),
            supply: Box::new(SupplyCore::forced_import(cfg, all_supply)?),
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
            .min(self.unrealized.min_stateful_len())
    }

    pub(crate) fn collect_all_vecs_mut(&mut self) -> Vec<&mut dyn AnyStoredVec> {
        let mut vecs: Vec<&mut dyn AnyStoredVec> = Vec::new();
        vecs.extend(self.supply.collect_vecs_mut());
        vecs.extend(self.outputs.collect_vecs_mut());
        vecs.extend(self.activity.collect_vecs_mut());
        vecs.extend(self.realized.collect_vecs_mut());
        vecs.extend(self.unrealized.collect_vecs_mut());
        vecs
    }

    pub(crate) fn compute_rest_part1(
        &mut self,
        prices: &price::Vecs,
        starting_lengths: &Lengths,
        exit: &Exit,
    ) -> Result<()> {
        self.activity
            .compute_rest_part1(prices, starting_lengths, exit)?;
        Ok(())
    }

    pub(crate) fn compute_rest_part2(
        &mut self,
        prices: &price::Vecs,
        starting_lengths: &Lengths,
        exit: &Exit,
    ) -> Result<()> {
        self.realized.compute_rest_part2(
            prices,
            starting_lengths,
            &self.supply.total.btc.height,
            exit,
        )?;

        Ok(())
    }
}
