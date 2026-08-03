use std::path::Path;

use brk_error::Result;
use brk_types::Version;

use super::{DB_NAME, Vecs, bedrock, capital_sentiment, rarity_meter::RarityMeter};
use crate::{
    indexes,
    internal::db_utils::{finalize_db, open_db},
};

impl Vecs {
    pub(crate) fn forced_import(
        parent_path: &Path,
        parent_version: Version,
        indexes: &indexes::Vecs,
    ) -> Result<Self> {
        let db = open_db(parent_path, DB_NAME, 100_000)?;
        let bedrock = bedrock::Vecs::forced_import(&db, parent_version, indexes)?;
        let capital_sentiment =
            capital_sentiment::Vecs::forced_import(&db, parent_version, indexes)?;
        let rarity_meter = RarityMeter::forced_import(&db, parent_version, indexes)?;

        let this = Self {
            db,
            bedrock,
            capital_sentiment,
            rarity_meter,
        };
        finalize_db(&this.db, &this)?;
        Ok(this)
    }
}
