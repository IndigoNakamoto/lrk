use std::path::Path;

use brk_error::{Error, Result};
use brk_types::Version;

use crate::{
    blocks, indexes,
    internal::db_utils::{finalize_db, open_db},
    price,
};

use super::{
    AthVecs, LookbackVecs, MovingAverageVecs, RangeVecs, ReturnsVecs, TechnicalVecs, Vecs,
    VolatilityVecs, lookback::ByLookbackPeriod,
};

impl Vecs {
    pub(crate) fn forced_import(
        parent_path: &Path,
        parent_version: Version,
        indexes: &indexes::Vecs,
        blocks: &blocks::Vecs,
        prices: &price::Vecs,
    ) -> Result<Self> {
        let db = open_db(parent_path, super::DB_NAME, 250_000)?;
        let version = parent_version;

        let ath = AthVecs::forced_import(&db, version, indexes)?;
        let cached_starts = ByLookbackPeriod::try_new(|_, days| {
            Ok::<_, Error>(
                blocks
                    .lookback
                    .cached_start_vec(days as usize)
                    .read_only_cached_boxed_clone(),
            )
        })?;
        let lookback = LookbackVecs::forced_import(version, indexes, &cached_starts, prices)?;
        let returns =
            ReturnsVecs::forced_import(&db, version, indexes, &cached_starts, prices)?;
        let volatility = VolatilityVecs::forced_import(version, &returns)?;
        let range = RangeVecs::forced_import(&db, version, indexes)?;
        let moving_average = MovingAverageVecs::forced_import(&db, version, indexes)?;
        let technical = TechnicalVecs::forced_import(&db, version, indexes)?;

        let this = Self {
            db,
            ath,
            lookback,
            returns,
            volatility,
            range,
            moving_average,
            technical,
        };
        finalize_db(&this.db, &this)?;
        Ok(this)
    }
}
