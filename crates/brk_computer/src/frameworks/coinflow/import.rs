use std::path::Path;

use brk_cohort::{AgeRange, CohortContext};
use brk_error::Result;
use brk_types::Version;

use super::{CohortVecs, DB_NAME, HorizonVecs, Horizons, Split, Vecs};
use crate::{
    indexes,
    internal::{
        FiatPerBlock, PerBlock, PriceWithRatioPerBlock, ValuePerBlock,
        db_utils::{finalize_db, open_db},
    },
};

const VERSION: Version = Version::TWO;

fn import_split<T>(mut import: impl FnMut(&str) -> Result<T>) -> Result<Split<T>> {
    Ok(Split {
        mobile: import("mobile")?,
        immobile: import("immobile")?,
    })
}

impl CohortVecs {
    fn forced_import(
        db: &vecdb::Database,
        name: &str,
        version: Version,
        indexes: &indexes::Vecs,
    ) -> Result<Self> {
        Ok(Self {
            spending_rate: PerBlock::forced_import(
                db,
                &format!("{name}_spending_rate"),
                version,
                indexes,
            )?,
            spending_exposure: PerBlock::forced_import(
                db,
                &format!("{name}_spending_exposure"),
                version,
                indexes,
            )?,
            mobility: PerBlock::forced_import(db, &format!("{name}_mobility"), version, indexes)?,
            supply: import_split(|side| {
                ValuePerBlock::forced_import(db, &format!("{name}_{side}_supply"), version, indexes)
            })?,
        })
    }
}

impl Vecs {
    pub(crate) fn forced_import(
        parent_path: &Path,
        parent_version: Version,
        indexes: &indexes::Vecs,
    ) -> Result<Self> {
        let db = open_db(parent_path, DB_NAME, 250_000)?;
        let version = parent_version + VERSION;

        let this = Self {
            age_range: AgeRange::try_new(|_, name| {
                let name = CohortContext::Utxo.prefixed(name);
                CohortVecs::forced_import(&db, &name, version, indexes)
            })?,
            supply: import_split(|side| {
                ValuePerBlock::forced_import(&db, &format!("{side}_supply"), version, indexes)
            })?,
            supply_in_loss_share: PerBlock::forced_import(
                &db,
                "coinflow_supply_in_loss_share",
                version,
                indexes,
            )?,
            horizon: Horizons::try_from_fn(|horizon, _| -> Result<_> {
                Ok(HorizonVecs {
                    supply_in_loss_share: PerBlock::forced_import(
                        &db,
                        &format!("coinflow_{horizon}_supply_in_loss_share"),
                        version,
                        indexes,
                    )?,
                })
            })?,
            cap: FiatPerBlock::forced_import(&db, "coinflow_cap", version, indexes)?,
            price: PriceWithRatioPerBlock::forced_import(&db, "coinflow_price", version, indexes)?,
            db,
        };

        finalize_db(&this.db, &this)?;
        Ok(this)
    }
}
