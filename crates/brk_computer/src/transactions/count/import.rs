use brk_error::Result;
use brk_types::Version;
use vecdb::Database;

use super::Vecs;
use crate::{
    indexes,
    internal::{CachedWindowStartVec, PerBlockFullFromCumulative, Windows},
};

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&CachedWindowStartVec>,
    ) -> Result<Self> {
        Ok(Self {
            total: PerBlockFullFromCumulative::forced_import(
                db,
                "tx_count",
                version,
                indexes.transaction_count_source(),
                indexes,
                cached_starts,
            )?,
        })
    }
}
