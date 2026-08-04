use brk_error::Result;
use brk_indexer::Lengths;
use brk_traversable::Traversable;
use brk_types::{Height, PartsPerMillion32, PartsPerMillionSigned64, Sats, SatsSigned, Version};
use vecdb::{
    AnyStoredVec, AnyVec, BinaryTransform, Exit, ReadableCloneableVec, Rw, StorageMode, WritableVec,
};

use crate::distribution::state::{CohortState, CostBasisOps, RealizedOps};

use crate::internal::{
    LazyIndexedVec, LazyPercentPerBlock, LazyRollingDeltasAmountFromHeight, RatioSats,
    SpotValuePerBlock,
};

use crate::distribution::metrics::{AllSupplyCache, ImportConfig};

/// Base supply metrics: total supply + dominance (share of circulating).
#[derive(Traversable)]
pub struct SupplyBase<M: StorageMode = Rw> {
    pub total: SpotValuePerBlock<M>,
    pub delta: LazyRollingDeltasAmountFromHeight<Sats, SatsSigned, PartsPerMillionSigned64>,
    #[traversable(rename = "dominance")]
    pub dominance: LazyPercentPerBlock<PartsPerMillion32>,
}

impl SupplyBase {
    pub(crate) fn forced_import(cfg: &ImportConfig, all_supply: &AllSupplyCache) -> Result<Self> {
        let supply: SpotValuePerBlock = cfg.import("supply", Version::ZERO)?;
        let name = cfg.name("supply_dominance");
        let source = LazyIndexedVec::new(
            &format!("{name}_ppm_source"),
            cfg.version,
            supply.sats.height.read_only_boxed_clone(),
            all_supply.cached_boxed_clone(),
            |_, supply, all_supply| RatioSats::<PartsPerMillion32>::apply(supply, all_supply),
        );
        let dominance =
            LazyPercentPerBlock::from_height_source(&name, cfg.version, source, cfg.indexes);

        Ok(Self::new(cfg, supply, dominance))
    }

    pub(crate) fn forced_import_all(cfg: &ImportConfig) -> Result<Self> {
        let supply: SpotValuePerBlock = cfg.import("supply", Version::ZERO)?;
        let dominance = LazyPercentPerBlock::from_indexed_source(
            &cfg.name("supply_dominance"),
            cfg.version,
            &supply.sats.height,
            Self::all_dominance,
            cfg.indexes,
        );

        Ok(Self::new(cfg, supply, dominance))
    }

    fn all_dominance(_height: Height, supply: Sats) -> PartsPerMillion32 {
        RatioSats::<PartsPerMillion32>::apply(supply, supply)
    }

    fn new(
        cfg: &ImportConfig,
        supply: SpotValuePerBlock,
        dominance: LazyPercentPerBlock<PartsPerMillion32>,
    ) -> Self {
        let delta = LazyRollingDeltasAmountFromHeight::new(
            &cfg.name("supply_delta"),
            cfg.version + Version::TWO,
            &supply.sats.height,
            cfg.cached_starts,
            cfg.indexes,
        );

        Self {
            total: supply,
            delta,
            dominance,
        }
    }

    pub(crate) fn min_len(&self) -> usize {
        self.total.sats.height.len()
    }

    #[inline(always)]
    pub(crate) fn push_state(&mut self, state: &CohortState<impl RealizedOps, impl CostBasisOps>) {
        self.total.sats.height.push(state.supply.value);
    }

    pub(crate) fn collect_vecs_mut(&mut self) -> Vec<&mut dyn AnyStoredVec> {
        vec![&mut self.total.sats.height as &mut dyn AnyStoredVec]
    }

    pub(crate) fn compute_from_stateful(
        &mut self,
        starting_lengths: &Lengths,
        others: &[&Self],
        exit: &Exit,
    ) -> Result<()> {
        self.total.sats.height.compute_sum_of_others(
            starting_lengths.height,
            &others
                .iter()
                .map(|v| &v.total.sats.height)
                .collect::<Vec<_>>(),
            exit,
        )?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_dominance_preserves_zero_supply() {
        assert_eq!(
            SupplyBase::all_dominance(Height::ZERO, Sats::ZERO),
            PartsPerMillion32::ZERO
        );
        assert_eq!(
            SupplyBase::all_dominance(Height::ZERO, Sats::new(1)),
            PartsPerMillion32::ONE
        );
    }
}
