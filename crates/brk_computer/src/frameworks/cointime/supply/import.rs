use brk_error::Result;
use brk_types::{Cents, Height, Version};
use vecdb::{CachedBoxedVec, Database};

use super::{BaseVecs, Vecs};
use crate::{
    indexes,
    internal::{PerBlock, SpotValuePerBlock},
};

impl BaseVecs {
    pub(crate) fn forced_import_with_prefix(
        db: &Database,
        prefix: &str,
        version: Version,
        indexes: &indexes::Vecs,
        spot_price: &CachedBoxedVec<Height, Cents>,
    ) -> Result<Self> {
        let name = |metric: &str| {
            if prefix.is_empty() {
                metric.to_owned()
            } else {
                format!("{prefix}_{metric}")
            }
        };

        Ok(Self {
            vaulted: SpotValuePerBlock::forced_import(
                db,
                &name("vaulted_supply"),
                version,
                indexes,
                spot_price,
            )?,
            active: SpotValuePerBlock::forced_import(
                db,
                &name("active_supply"),
                version,
                indexes,
                spot_price,
            )?,
        })
    }
}

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexes: &indexes::Vecs,
        spot_price: &CachedBoxedVec<Height, Cents>,
    ) -> Result<Self> {
        Ok(Self {
            base: BaseVecs::forced_import_with_prefix(db, "", version, indexes, spot_price)?,
            active_supply_in_loss_share: PerBlock::forced_import(
                db,
                "cointime_supply_in_loss_share",
                version,
                indexes,
            )?,
        })
    }
}
