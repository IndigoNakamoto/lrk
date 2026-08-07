use brk_error::Result;
use brk_types::Version;
use vecdb::{Database, EagerVec, ImportableVec};

use super::{CountVecs, Vecs};
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
            count: CountVecs {
                nonstandard: PerBlockCumulativeRolling::forced_import(
                    db,
                    "nonstandard_count",
                    version,
                    indexes,
                    cached_starts,
                )?,
            },
            is_nonstandard: EagerVec::forced_import(db, "is_nonstandard", version)?,
        })
    }
}
