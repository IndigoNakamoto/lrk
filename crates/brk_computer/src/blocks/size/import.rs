use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::{Height, StoredU64, Version, Weight};
use vecdb::Database;

use super::Vecs;
use crate::{
    indexes,
    internal::{CachedPerBlockRolling, CachedWindowStartVec, PerBlockFull, Windows},
};

fn block_vbytes(_: Height, weight: Weight) -> StoredU64 {
    StoredU64::from(weight.to_vbytes_floor())
}

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexer: &Indexer,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&CachedWindowStartVec>,
    ) -> Result<Self> {
        Ok(Self {
            vbytes: PerBlockFull::forced_import(
                db,
                "block_vbytes",
                version,
                &indexer.vecs().blocks.weight,
                block_vbytes,
                indexes,
                cached_starts,
            )?,
            size: CachedPerBlockRolling::forced_import(
                db,
                "block_size",
                version,
                indexes,
                cached_starts,
            )?,
        })
    }
}
