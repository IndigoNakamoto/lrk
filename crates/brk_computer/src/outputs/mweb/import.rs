use brk_error::Result;
use brk_types::Version;
use vecdb::Database;

use super::Vecs;
use crate::{
    indexes,
    internal::{ValuePerBlock, ValuePerBlockCumulative},
};

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexes: &indexes::Vecs,
    ) -> Result<Self> {
        Ok(Self {
            outputs_value: ValuePerBlockCumulative::forced_import(
                db,
                "mweb_outputs_value",
                version,
                indexes,
            )?,
            inputs_value: ValuePerBlockCumulative::forced_import(
                db,
                "mweb_inputs_value",
                version,
                indexes,
            )?,
            balance: ValuePerBlock::forced_import(db, "mweb_balance", version, indexes)?,
        })
    }
}
