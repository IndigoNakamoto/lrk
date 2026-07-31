use brk_error::Result;
use brk_traversable::Traversable;
use brk_types::{Cents, Height, Sats, StoredU64, Version};
use vecdb::{
    AnyStoredVec, CachedBoxedVec, Database, Exit, ReadableVec, Rw, StorageMode, WritableVec,
};

use crate::{indexes, internal::SpotValuePerBlock};

/// Average amount held per UTXO and per funded address.
///
/// `utxo = supply / utxo_count`, `addr = supply / funded_addr_count`.
#[derive(Traversable)]
pub struct AvgAmountMetrics<M: StorageMode = Rw> {
    pub utxo: SpotValuePerBlock<M>,
    pub addr: SpotValuePerBlock<M>,
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
