use std::path::Path;

use brk_error::Result;
use brk_types::{Dollars, StoredF64, Version};
use vecdb::Database;

use super::{
    DB_NAME,
    urpd_metric::{UrpdMappings, UrpdMetric},
    vecs::{Levels, ModeVecs, Modes, Percentiles, Vecs},
};
use crate::{
    indexes,
    internal::db_utils::{finalize_db, open_db},
};

const VERSION: Version = Version::new(3);

fn import_percentiles<T>(mut import: impl FnMut(&str) -> Result<T>) -> Result<Percentiles<T>> {
    Ok(Percentiles {
        pct95: import("pct95")?,
        pct98: import("pct98")?,
        pct99: import("pct99")?,
        pct99_5: import("pct99_5")?,
        pct99_9: import("pct99_9")?,
    })
}

fn import_levels<T>(mut import: impl FnMut(&str) -> Result<T>) -> Result<Levels<T>> {
    Ok(Levels {
        pct10: import("pct10")?,
        pct20: import("pct20")?,
        pct30: import("pct30")?,
        pct40: import("pct40")?,
        pct50: import("pct50")?,
        pct60: import("pct60")?,
        pct70: import("pct70")?,
        pct80: import("pct80")?,
        pct90: import("pct90")?,
    })
}

fn import_ratio(
    db: &Database,
    name: &str,
    version: Version,
    mappings: &UrpdMappings,
) -> Result<UrpdMetric<StoredF64>> {
    UrpdMetric::forced_import(db, name, version, mappings)
}

fn import_price(
    db: &Database,
    name: &str,
    version: Version,
    mappings: &UrpdMappings,
) -> Result<UrpdMetric<Dollars>> {
    UrpdMetric::forced_import(db, name, version, mappings)
}

fn import_mode(
    db: &Database,
    name: &str,
    version: Version,
    mappings: &UrpdMappings,
) -> Result<ModeVecs> {
    Ok(ModeVecs {
        loss_threshold: import_percentiles(|percentile| {
            import_ratio(
                db,
                &format!("{name}_loss_threshold_{percentile}"),
                version,
                mappings,
            )
        })?,
        floor: import_percentiles(|percentile| {
            import_price(db, &format!("{name}_floor_{percentile}"), version, mappings)
        })?,
        level: import_levels(|percentile| {
            import_price(db, &format!("{name}_level_{percentile}"), version, mappings)
        })?,
    })
}

impl Vecs {
    pub(crate) fn forced_import(
        parent_path: &Path,
        parent_version: Version,
        indexes: &indexes::Vecs,
    ) -> Result<Self> {
        let db = open_db(parent_path, DB_NAME, 50_000)?;
        let version = parent_version + VERSION;
        let mappings = UrpdMappings::new(indexes);

        let this = Self {
            modes: Modes::try_from_fn(|name| {
                import_mode(&db, &format!("bedrock_{name}"), version, &mappings)
            })?,
            db,
        };

        finalize_db(&this.db, &this)?;
        Ok(this)
    }
}
