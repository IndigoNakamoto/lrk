use brk_error::Result;
use brk_types::Version;
use vecdb::Database;

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
        let import = |name| {
            PerBlockCumulativeRolling::forced_import(db, name, version, indexes, cached_starts)
        };

        Ok(Self {
            count: CountVecs {
                inscription: import("tx_count_inscription")?,
                annex: import("tx_count_annex")?,
                sighash_all: import("tx_count_sighash_all")?,
                sighash_none: import("tx_count_sighash_none")?,
                sighash_single: import("tx_count_sighash_single")?,
                sighash_default: import("tx_count_sighash_default")?,
                sighash_anyone_can_pay: import("tx_count_sighash_anyone_can_pay")?,
                dust_output: import("tx_count_dust_output")?,
            },
        })
    }
}
