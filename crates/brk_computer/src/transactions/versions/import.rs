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
            v1: PerBlockCumulativeRolling::forced_import(
                db,
                "tx_v1",
                version,
                indexes,
                cached_starts,
            )?,
            v2: PerBlockCumulativeRolling::forced_import(
                db,
                "tx_v2",
                version,
                indexes,
                cached_starts,
            )?,
            v3: PerBlockCumulativeRolling::forced_import(
                db,
                "tx_v3",
                version,
                indexes,
                cached_starts,
            )?,
            other: PerBlockCumulativeRolling::forced_import(
                db,
                "tx_other_version",
                version,
                indexes,
                cached_starts,
            )?,
        })
    }
}
