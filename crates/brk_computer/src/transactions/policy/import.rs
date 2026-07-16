use brk_error::Result;
use brk_types::Version;
use vecdb::{Database, EagerVec, ImportableVec};

use super::Vecs;

impl Vecs {
    pub(crate) fn forced_import(db: &Database, version: Version) -> Result<Self> {
        Ok(Self {
            count: EagerVec::forced_import(db, "nonstandard_count", version)?,
            is_nonstandard: EagerVec::forced_import(db, "is_nonstandard", version)?,
        })
    }
}
