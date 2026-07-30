use brk_error::Result;
use brk_types::{StoredU64, Version};
use vecdb::Database;

use super::Vecs;
use crate::{
    indexes,
    internal::{
        LazyPerSecondWindows, LazyRollingSumsFromHeight, ValuePerBlockCumulativeRolling,
        WindowStartVec, Windows,
    },
};

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
        tx_count_sums: &LazyRollingSumsFromHeight<StoredU64>,
    ) -> Result<Self> {
        let v = version + Version::TWO;
        Ok(Self {
            transfer_volume: ValuePerBlockCumulativeRolling::forced_import(
                db,
                "transfer_volume_bis",
                version,
                indexes,
                cached_starts,
            )?,
            tx_per_sec: LazyPerSecondWindows::new("tx_per_sec", v, tx_count_sums),
        })
    }
}
