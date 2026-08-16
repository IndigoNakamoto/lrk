use brk_error::Result;
use brk_types::Version;
use vecdb::{Database, EagerVec, ImportableVec};

use super::{CountVecs, Vecs};
use crate::{
    indexes,
    internal::{CachedWindowStartVec, PerBlockCumulativeRolling, PerTxDistribution, Windows},
};

/// Bump this when fee/feerate aggregation logic changes (e.g., skip coinbase, skip zero-fee).
const VERSION: Version = Version::new(3);

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&CachedWindowStartVec>,
    ) -> Result<Self> {
        let v = version + VERSION;
        Ok(Self {
            count: CountVecs {
                cpfp_parent: PerBlockCumulativeRolling::forced_import(
                    db,
                    "cpfp_parent_count",
                    version,
                    indexes,
                    cached_starts,
                )?,
                cpfp_child: PerBlockCumulativeRolling::forced_import(
                    db,
                    "cpfp_child_count",
                    version,
                    indexes,
                    cached_starts,
                )?,
            },
            input_value: EagerVec::forced_import(db, "input_value", version)?,
            transfer_input_value: EagerVec::forced_import(db, "transfer_input_value", version)?,
            output_value: EagerVec::forced_import(db, "output_value", version)?,
            fee: PerTxDistribution::forced_import(db, "fee", v, indexes)?,
            fee_rate: EagerVec::forced_import(db, "fee_rate", v)?,
            effective_fee_rate: PerTxDistribution::forced_import(
                db,
                "effective_fee_rate",
                v,
                indexes,
            )?,
            is_cpfp_parent: EagerVec::forced_import(db, "is_cpfp_parent", version)?,
            is_cpfp_child: EagerVec::forced_import(db, "is_cpfp_child", version)?,
        })
    }
}
