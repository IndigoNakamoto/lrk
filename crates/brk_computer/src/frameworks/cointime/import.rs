use std::path::Path;

use brk_error::Result;
use brk_types::{Cents, Version};

use crate::{
    indexes,
    internal::{
        PerBlock,
        db_utils::{finalize_db, open_db},
    },
    price,
};

use super::{
    ActivityVecs, AdjustedVecs, AgeRangeVecs, CapVecs, DB_NAME, PricesVecs, ReserveRiskVecs,
    SupplyVecs, ValueVecs, Vecs,
};

use crate::internal::{WindowStartVec, Windows};

impl Vecs {
    pub(crate) fn forced_import(
        parent_path: &Path,
        parent_version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
        prices: &price::Vecs,
        subsidy_cents: &PerBlock<Cents>,
    ) -> Result<Self> {
        let db = open_db(parent_path, DB_NAME, 250_000)?;
        let version = parent_version;
        let v1 = version + Version::ONE;
        let spot_price = prices.spot.cents.height.read_only_cached_boxed_clone();
        let activity = ActivityVecs::forced_import(&db, version, indexes, cached_starts)?;
        let age_range =
            AgeRangeVecs::forced_import(&db, version, indexes, cached_starts, &spot_price)?;
        let supply = SupplyVecs::forced_import(&db, v1, indexes, &spot_price)?;
        let value = ValueVecs::forced_import(&db, v1, indexes, cached_starts)?;
        let cap = CapVecs::forced_import(&db, version + Version::TWO, indexes, subsidy_cents)?;
        let prices =
            PricesVecs::forced_import(&db, version + Version::new(3), indexes, &spot_price)?;
        let adjusted = AdjustedVecs::forced_import(&db, version, indexes)?;
        let reserve_risk = ReserveRiskVecs::forced_import(&db, v1, indexes, &spot_price)?;

        let this = Self {
            db,
            activity,
            age_range,
            supply,
            value,
            cap,
            prices,
            adjusted,
            reserve_risk,
        };
        finalize_db(&this.db, &this)?;
        Ok(this)
    }
}
