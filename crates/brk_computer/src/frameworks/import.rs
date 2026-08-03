use std::path::Path;

use brk_error::Result;
use brk_types::{Cents, Version};

use super::{DB_NAME, Vecs, coinflow, cointime};
use crate::{
    indexes,
    internal::{
        PerBlock, WindowStartVec, Windows,
        db_utils::{finalize_db, open_db},
    },
    price,
};

impl Vecs {
    pub(crate) fn forced_import(
        parent_path: &Path,
        parent_version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
        prices: &price::Vecs,
        subsidy_cents: &PerBlock<Cents>,
    ) -> Result<Self> {
        let db = open_db(parent_path, DB_NAME, 250_000)?;
        let cointime = cointime::Vecs::forced_import(
            &db,
            parent_version,
            indexes,
            cached_starts,
            prices,
            subsidy_cents,
        )?;
        let coinflow = coinflow::Vecs::forced_import(&db, parent_version, indexes, prices)?;

        let this = Self {
            db,
            cointime,
            coinflow,
        };
        finalize_db(&this.db, &this)?;
        Ok(this)
    }
}
