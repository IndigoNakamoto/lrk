use std::path::Path;

use brk_error::Result;
use brk_types::Version;

use super::{ByKind, Metrics, Policy, TotalMetrics, Vecs};
use crate::{
    indexes,
    internal::{
        WindowStartVec, Windows,
        db_utils::{finalize_db, open_db},
    },
};

impl Vecs {
    pub(crate) fn forced_import(
        parent_path: &Path,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
    ) -> Result<Self> {
        let db = open_db(parent_path, super::DB_NAME, 1_000_000)?;
        let total = TotalMetrics::forced_import(&db, "op_return", version, indexes, cached_starts)?;
        let by_kind = ByKind::try_new(|_, name| {
            Metrics::forced_import(
                &db,
                &format!("op_return_{name}"),
                version,
                indexes,
                cached_starts,
            )
        })?;
        let policy = Policy::forced_import(&db, version, indexes, cached_starts)?;

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
