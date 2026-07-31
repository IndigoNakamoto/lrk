use std::path::Path;

use brk_cohort::{AgeRange, CohortContext};
use brk_error::Result;
use brk_types::{Cents, Height, StoredF64, Version};
use vecdb::{CachedBoxedVec, ReadableCloneableVec, UnaryTransform};

use super::{CohortVecs, DB_NAME, HorizonVecs, Horizons, Split, Vecs, mobility};
use crate::{
    indexes,
    internal::{
        FiatPerBlock, LazyPerBlock, PerBlock, PriceWithRatioPerBlock, SpotValuePerBlock,
        db_utils::{finalize_db, open_db},
    },
};

const VERSION: Version = Version::TWO;

struct ExposureToMobility;

impl UnaryTransform<StoredF64, StoredF64> for ExposureToMobility {
    #[inline(always)]
    fn apply(exposure: StoredF64) -> StoredF64 {
        StoredF64::from(mobility(*exposure))
    }
}

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
        spot_price: &CachedBoxedVec<Height, Cents>,
    ) -> Result<Self> {
        let spending_rate =
            PerBlock::forced_import(db, &format!("{name}_spending_rate"), version, indexes)?;
        let spending_exposure =
            PerBlock::forced_import(db, &format!("{name}_spending_exposure"), version, indexes)?;
        let mobility = LazyPerBlock::from_computed::<ExposureToMobility>(
            &format!("{name}_mobility"),
            version,
            spending_exposure.height.read_only_boxed_clone(),
            &spending_exposure,
        );

        Ok(Self {
            spending_rate,
            spending_exposure,
            mobility,
            supply: import_split(|side| {
                SpotValuePerBlock::forced_import(
                    db,
                    &format!("{name}_{side}_supply"),
                    version,
                    indexes,
                    spot_price,
                )
            })?,
        })
    }
}

impl Vecs {
    pub(crate) fn forced_import(
        parent_path: &Path,
        parent_version: Version,
        indexes: &indexes::Vecs,
        prices: &crate::price::Vecs,
    ) -> Result<Self> {
        let db = open_db(parent_path, DB_NAME, 250_000)?;
        let version = parent_version + VERSION;
        let spot_price = prices.spot.cents.height.read_only_cached_boxed_clone();

        let this = Self {
            age_range: AgeRange::try_new(|_, name| {
                let name = CohortContext::Utxo.prefixed(name);
                CohortVecs::forced_import(&db, &name, version, indexes, &spot_price)
            })?,
            supply: import_split(|side| {
                SpotValuePerBlock::forced_import(
                    &db,
                    &format!("{side}_supply"),
                    version,
                    indexes,
                    &spot_price,
                )
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
            price: PriceWithRatioPerBlock::forced_import(
                &db,
                "coinflow_price",
                version,
                indexes,
                &spot_price,
            )?,
            db,
        };

        finalize_db(&this.db, &this)?;
        Ok(this)
    }
}
