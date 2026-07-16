use brk_error::Result;
use brk_types::Version;
use vecdb::{Database, EagerVec, ImportableVec};

use super::{CountVecs, Vecs};

impl Vecs {
    pub(crate) fn forced_import(db: &Database, version: Version) -> Result<Self> {
        Ok(Self {
            count: CountVecs {
                coinjoin: EagerVec::forced_import(db, "coinjoin_count", version)?,
                consolidation: EagerVec::forced_import(db, "consolidation_count", version)?,
                batch_payout: EagerVec::forced_import(db, "batch_payout_count", version)?,
            },
            is_coinjoin: EagerVec::forced_import(db, "is_coinjoin", version)?,
            is_consolidation: EagerVec::forced_import(db, "is_consolidation", version)?,
            is_batch_payout: EagerVec::forced_import(db, "is_batch_payout", version)?,
        })
    }
}
