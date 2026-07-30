use brk_error::Result;
use brk_types::{Height, StoredU64, Version};
use vecdb::Database;

use super::Vecs;
use crate::{
    indexes,
    internal::{PerBlockFull, WindowStartVec, Windows},
};

fn tx_count(_: Height, count: StoredU64) -> StoredU64 {
    count
}

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
    ) -> Result<Self> {
        Ok(Self {
            total: PerBlockFull::forced_import(
                db,
                "tx_count",
                version,
                &indexes.height.tx_index_count,
                tx_count,
                indexes,
                cached_starts,
            )?,
        })
    }
}
