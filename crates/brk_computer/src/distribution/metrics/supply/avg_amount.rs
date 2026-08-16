use brk_cohort::ByAddrType;
use brk_error::Result;
use brk_traversable::Traversable;
use brk_types::{Cents, Height, Sats, StoredU64, Version};
use rayon::prelude::*;
use vecdb::{
    AnyStoredVec, CachedBoxedVec, Database, Exit, ReadableVec, Rw, StorageMode, WritableVec,
};

use crate::{
    distribution::AllChainCache,
    indexes,
    internal::{LazySpotValuePerBlock, SpotValuePerBlock},
};

/// Average amount held per UTXO and per funded address.
///
/// `utxo = supply / utxo_count`, `addr = supply / funded_addr_count`.
#[derive(Traversable)]
pub struct AvgAmountMetrics<M: StorageMode = Rw> {
    pub utxo: SpotValuePerBlock<M>,
    pub addr: SpotValuePerBlock<M>,
}

#[derive(Clone, Traversable)]
pub struct LazyAvgAmountMetrics {
    pub utxo: LazySpotValuePerBlock,
    pub addr: LazySpotValuePerBlock,
}

#[derive(Traversable)]
pub struct AvgAmountVecs<M: StorageMode = Rw> {
    pub all: LazyAvgAmountMetrics,
    #[traversable(flatten)]
    pub by_addr_type: ByAddrType<AvgAmountMetrics<M>>,
}

impl AvgAmountVecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexes: &indexes::Vecs,
        spot_price: &CachedBoxedVec<Height, Cents>,
        all_chain: &AllChainCache,
        utxo_count: &(impl vecdb::ReadableCloneableVec<Height, StoredU64> + 'static),
        funded_addr_count: &(impl vecdb::ReadableCloneableVec<Height, StoredU64> + 'static),
    ) -> Result<Self> {
        let avg_utxo = all_chain.with_supply(
            "avg_utxo_amount_sats_source",
            Version::ZERO,
            utxo_count,
            |_, count, supply| supply / count,
        );
        let avg_addr = all_chain.with_supply(
            "avg_addr_amount_sats_source",
            Version::ZERO,
            funded_addr_count,
            |_, count, supply| supply / count,
        );
        let all = LazyAvgAmountMetrics {
            utxo: LazySpotValuePerBlock::from_sats_source(
                "avg_utxo_amount",
                version,
                avg_utxo,
                indexes,
                spot_price,
            ),
            addr: LazySpotValuePerBlock::from_sats_source(
                "avg_addr_amount",
                version,
                avg_addr,
                indexes,
                spot_price,
            ),
        };
        let by_addr_type = ByAddrType::new_with_name(|type_name| {
            AvgAmountMetrics::forced_import(db, type_name, version, indexes, spot_price)
        })?;

        Ok(Self { all, by_addr_type })
    }

    pub(crate) fn par_iter_height_mut(
        &mut self,
    ) -> impl ParallelIterator<Item = &mut dyn AnyStoredVec> {
        self.by_addr_type
            .par_values_mut()
            .flat_map_iter(AvgAmountMetrics::collect_vecs_mut)
    }

    pub(crate) fn reset_height(&mut self) -> Result<()> {
        for metrics in self.by_addr_type.values_mut() {
            metrics.reset_height()?;
        }
        Ok(())
    }
}

impl AvgAmountMetrics {
    pub(crate) fn forced_import(
        db: &Database,
        prefix: &str,
        version: Version,
        indexes: &indexes::Vecs,
        spot_price: &CachedBoxedVec<Height, Cents>,
    ) -> Result<Self> {
        let name = |suffix: &str| {
            if prefix.is_empty() {
                suffix.to_string()
            } else {
                format!("{prefix}_{suffix}")
            }
        };
        Ok(Self {
            utxo: SpotValuePerBlock::forced_import(
                db,
                &name("avg_utxo_amount"),
                version,
                indexes,
                spot_price,
            )?,
            addr: SpotValuePerBlock::forced_import(
                db,
                &name("avg_addr_amount"),
                version,
                indexes,
                spot_price,
            )?,
        })
    }

    pub(crate) fn collect_vecs_mut(&mut self) -> Vec<&mut dyn AnyStoredVec> {
        vec![
            &mut self.utxo.sats.height as &mut dyn AnyStoredVec,
            &mut self.addr.sats.height,
        ]
    }

    pub(crate) fn reset_height(&mut self) -> Result<()> {
        self.utxo.sats.height.reset()?;
        self.addr.sats.height.reset()?;
        Ok(())
    }

    pub(crate) fn compute(
        &mut self,
        supply_sats: &impl ReadableVec<Height, Sats>,
        utxo_count: &impl ReadableVec<Height, StoredU64>,
        funded_addr_count: &impl ReadableVec<Height, StoredU64>,
        max_from: Height,
        exit: &Exit,
    ) -> Result<()> {
        self.utxo
            .sats
            .height
            .compute_divide(max_from, supply_sats, utxo_count, exit)?;

        self.addr
            .sats
            .height
            .compute_divide(max_from, supply_sats, funded_addr_count, exit)?;

        Ok(())
    }
}
