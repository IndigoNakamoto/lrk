use brk_error::Result;
use brk_types::Version;
use vecdb::Database;

use super::Vecs;
use crate::{
    indexes,
    internal::{CachedWindowStartVec, PerBlockAggregated, Windows},
};

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&CachedWindowStartVec>,
    ) -> Result<Self> {
        Ok(Self(PerBlockAggregated::forced_import(
            db,
            "input_count",
            version,
            indexes.input_count_source(),
            indexes,
            cached_starts,
        )?))
    }
}
