//! Bedrock-local price wrapper for metrics whose stored source is indexed by day.
//!
//! This mirrors the per-block `Price` conversion chain while preserving Bedrock's
//! custom daily repeat/last-day views: cents are stored, USD is derived from cents,
//! and sats are derived from USD.

use brk_traversable::Traversable;
use brk_types::{Cents, Dollars, SatsFract, Version};
use vecdb::{ReadableCloneableVec, Rw, StorageMode};

use crate::internal::{
    CentsUnsignedToDollars, DailyMappings, DailyMetric, DollarsToSatsFract, LazyDailyMetric,
};

#[derive(Traversable)]
pub struct Price<M: StorageMode = Rw> {
    pub usd: LazyDailyMetric<Dollars, Cents>,
    pub cents: DailyMetric<Cents, M>,
    pub sats: LazyDailyMetric<SatsFract, Dollars>,
}

impl Price {
    pub(crate) fn forced_import(
        db: &vecdb::Database,
        name: &str,
        version: Version,
        mappings: &DailyMappings,
    ) -> brk_error::Result<Self> {
        let cents = DailyMetric::forced_import(db, &format!("{name}_cents"), version, mappings)?;
        let usd = LazyDailyMetric::from_source::<CentsUnsignedToDollars>(
            name,
            version,
            cents.day1.read_only_boxed_clone(),
            mappings,
        );
        let sats = LazyDailyMetric::from_source::<DollarsToSatsFract>(
            &format!("{name}_sats"),
            version,
            usd.day1.read_only_boxed_clone(),
            mappings,
        );

        Ok(Self { usd, cents, sats })
    }
}
