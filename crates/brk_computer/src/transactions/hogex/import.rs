use brk_error::Result;
use brk_types::Version;
use vecdb::Database;

use super::Vecs;
use crate::{
    indexes,
    internal::{PerBlockCumulativeRolling, ValuePerBlockCumulativeRolling, WindowStartVec, Windows},
};

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
    ) -> Result<Self> {
        Ok(Self {
            tx_count: PerBlockCumulativeRolling::forced_import(
                db,
                "hogex_tx_count",
                version,
                indexes,
                cached_starts,
            )?,
            raw_input_volume: ValuePerBlockCumulativeRolling::forced_import(
                db,
                "raw_input_volume",
                version,
                indexes,
                cached_starts,
            )?,
        })
    }
}
