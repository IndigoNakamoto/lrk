use brk_error::Result;
use brk_types::Version;
use vecdb::Database;

use super::{PegFlow, Vecs};
use crate::{
    indexes,
    internal::{
        PerBlockCumulativeRolling, ValuePerBlock, ValuePerBlockCumulative,
        CachedWindowStartVec, ValuePerBlockCumulativeRolling, Windows,
    },
};

fn import_peg_flow(
    db: &Database,
    prefix: &str,
    version: Version,
    indexes: &indexes::Vecs,
) -> Result<PegFlow> {
    Ok(PegFlow {
        outputs_value: ValuePerBlockCumulative::forced_import(
            db,
            &format!("{prefix}_outputs_value"),
            version,
            indexes,
        )?,
        inputs_value: ValuePerBlockCumulative::forced_import(
            db,
            &format!("{prefix}_inputs_value"),
            version,
            indexes,
        )?,
        balance: ValuePerBlock::forced_import(db, &format!("{prefix}_balance"), version, indexes)?,
    })
}

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&CachedWindowStartVec>,
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
            peg_pool: import_peg_flow(db, "mweb_peg_pool", version, indexes)?,
            pegin: import_peg_flow(db, "mweb_pegin", version, indexes)?,
            pegin_count: PerBlockCumulativeRolling::forced_import(
                db,
                "mweb_pegin_count",
                version,
                indexes,
                cached_starts,
            )?,
            pegout_value: ValuePerBlockCumulativeRolling::forced_import(
                db,
                "mweb_pegout_value",
                version,
                indexes,
                cached_starts,
            )?,
            pegout_count: PerBlockCumulativeRolling::forced_import(
                db,
                "mweb_pegout_count",
                version,
                indexes,
                cached_starts,
            )?,
        })
    }
}
