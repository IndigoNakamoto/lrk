use brk_cohort::ByAddrType;
use brk_error::Result;
use brk_traversable::Traversable;
use brk_types::{Height, PartsPerMillion32, Sats, Version};
use vecdb::{Database, Exit, ReadableVec, Rw, StorageMode, WritableVec};

use crate::{
    distribution::metrics::AllSupplyCache,
    indexes,
    internal::{LazyPercentPerBlock, PercentPerBlock, RatioSats},
};

use super::vecs::AddrSupplyVecs;

/// Share of a predicate-based supply category relative to total supply.
///
/// - `all`: category supply / circulating supply
/// - Per-type: type's category supply / type's total supply
#[derive(Traversable)]
pub struct AddrSupplyShareVecs<M: StorageMode = Rw> {
    pub all: LazyPercentPerBlock<PartsPerMillion32>,
    #[traversable(flatten)]
    pub by_addr_type: ByAddrType<PercentPerBlock<PartsPerMillion32, M>>,
}

impl AddrSupplyShareVecs {
    pub(crate) fn forced_import(
        db: &Database,
        name: &str,
        version: Version,
        indexes: &indexes::Vecs,
        supply: &AddrSupplyVecs,
        all_supply: &AllSupplyCache,
    ) -> Result<Self> {
        let name = format!("{name}_addr_supply_share");
        let all = LazyPercentPerBlock::from_cached_ratio::<Sats, Sats, RatioSats<PartsPerMillion32>>(
            &name,
            version,
            &supply.all.sats.height,
            all_supply.cached_boxed_clone(),
            indexes,
        );
        let by_addr_type = ByAddrType::new_with_name(|type_name| {
            PercentPerBlock::forced_import(db, &format!("{type_name}_{name}"), version, indexes)
        })?;

        Ok(Self { all, by_addr_type })
    }

    pub(crate) fn reset_height(&mut self) -> Result<()> {
        for share in self.by_addr_type.values_mut() {
            share.ppm.height.reset()?;
        }
        Ok(())
    }

    pub(crate) fn compute_rest(
        &mut self,
        max_from: Height,
        supply: &AddrSupplyVecs,
        type_supply_sats: &ByAddrType<&impl ReadableVec<Height, Sats>>,
        exit: &Exit,
    ) -> Result<()> {
        for ((_, share), ((_, cat), (_, denom))) in self
            .by_addr_type
            .iter_mut()
            .zip(supply.by_addr_type.iter().zip(type_supply_sats.iter()))
        {
            share.compute_binary::<Sats, Sats, RatioSats<PartsPerMillion32>>(
                max_from,
                &cat.sats.height,
                *denom,
                exit,
            )?;
        }
        Ok(())
    }
}
