use std::path::Path;

use brk_error::Result;
use brk_types::{Height, Sats, StoredU64, Version};
use vecdb::CachedBoxedVec;

use super::{ByKind, Policy, Total, Vecs, vecs::BreakdownImporter};
use crate::{
    indexes,
    internal::{
        CachedWindowStartVec, Windows,
        db_utils::{finalize_db, open_db},
    },
};

impl Vecs {
    pub(crate) fn forced_import(
        parent_path: &Path,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&CachedWindowStartVec>,
        block_size: CachedBoxedVec<Height, StoredU64>,
        chain_fees: CachedBoxedVec<Height, Sats>,
    ) -> Result<Self> {
        let db = open_db(parent_path, super::DB_NAME, 1_000_000)?;
        let total = Total::forced_import(
            &db,
            "op_return",
            version,
            indexes,
            cached_starts,
            &block_size,
            &chain_fees,
        )?;
        let total_data = total.cached_data_bytes();
        let breakdowns = BreakdownImporter::new(
            &db,
            version,
            indexes,
            cached_starts,
            &total_data,
            &block_size,
            &chain_fees,
        );
        let by_kind = ByKind::try_new(|_, name| breakdowns.import(&format!("op_return_{name}")))?;
        let policy = Policy::forced_import(&breakdowns)?;

        let this = Self {
            db,
            total,
            by_kind,
            policy,
        };
        finalize_db(&this.db, &this)?;
        Ok(this)
    }
}
