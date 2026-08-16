use brk_error::Result;
use brk_types::Version;
use vecdb::Database;

use super::Vecs;
use crate::{
    indexes,
    internal::{CachedWindowStartVec, PerBlockCumulativeRolling, Windows},
};

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&CachedWindowStartVec>,
    ) -> Result<Self> {
        Ok(Self {
            total: PerBlockCumulativeRolling::forced_import(
                db,
                "total_sigop_cost",
                version,
                indexes,
                cached_starts,
            )?,
        })
    }
}
