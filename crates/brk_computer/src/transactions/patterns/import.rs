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
                coinjoin: PerBlockCumulativeRolling::forced_import(
                    db,
                    "coinjoin_count",
                    version,
                    indexes,
                    cached_starts,
                )?,
                consolidation: PerBlockCumulativeRolling::forced_import(
                    db,
                    "consolidation_count",
                    version,
                    indexes,
                    cached_starts,
                )?,
                batch_payout: PerBlockCumulativeRolling::forced_import(
                    db,
                    "batch_payout_count",
                    version,
                    indexes,
                    cached_starts,
                )?,
            },
            is_coinjoin: EagerVec::forced_import(db, "is_coinjoin", version)?,
            is_consolidation: EagerVec::forced_import(db, "is_consolidation", version)?,
            is_batch_payout: EagerVec::forced_import(db, "is_batch_payout", version)?,
        })
    }
}
