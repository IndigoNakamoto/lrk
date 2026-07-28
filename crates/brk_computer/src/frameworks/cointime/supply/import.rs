use brk_error::Result;
use brk_types::Version;
use vecdb::Database;

use super::{BaseVecs, Vecs};
use crate::{
    indexes,
    internal::{PerBlock, ValuePerBlock},
};

impl BaseVecs {
    pub(crate) fn forced_import_with_prefix(
        db: &Database,
        prefix: &str,
        version: Version,
        indexes: &indexes::Vecs,
    ) -> Result<Self> {
        let name = |metric: &str| {
            if prefix.is_empty() {
                metric.to_owned()
            } else {
                format!("{prefix}_{metric}")
            }
        };

        Ok(Self {
            vaulted: ValuePerBlock::forced_import(db, &name("vaulted_supply"), version, indexes)?,
            active: ValuePerBlock::forced_import(db, &name("active_supply"), version, indexes)?,
        })
    }
}

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexes: &indexes::Vecs,
    ) -> Result<Self> {
        Ok(Self {
            base: BaseVecs::forced_import_with_prefix(db, "", version, indexes)?,
            active_supply_in_loss_share: PerBlock::forced_import(
                db,
                "cointime_supply_in_loss_share",
                version,
                indexes,
            )?,
        })
    }
}
