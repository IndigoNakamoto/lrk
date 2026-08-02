use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::Version;
use vecdb::{AnyVec, Database, EagerVec, ImportableVec};

use super::Vecs;
use crate::{
    indexes,
    internal::{
        LazyPercentCumulativeRolling, OneMinusPpm, PercentCumulativeRolling, PercentRollingWindows,
        ValuePerBlockCumulative, ValuePerBlockCumulativeRolling, ValuePerBlockFull, WindowStartVec,
        Windows,
    },
};

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexer: &Indexer,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
    ) -> Result<Self> {
        let coinbase_version = version
            + indexer.vecs.transactions.first_txout_index.version()
            + indexes.tx_index.output_count.version()
            + indexer.vecs.outputs.value.version();

        let fee_dominance =
            PercentCumulativeRolling::forced_import(db, "fee_dominance", version, indexes)?;

        let subsidy_dominance = LazyPercentCumulativeRolling::from_source::<OneMinusPpm>(
            "subsidy_dominance",
            version,
            &fee_dominance,
        );

        Ok(Self {
            coinbase: ValuePerBlockCumulativeRolling::forced_import(
                db,
                "coinbase",
                coinbase_version,
                indexes,
                cached_starts,
            )?,
            subsidy: ValuePerBlockCumulativeRolling::forced_import(
                db,
                "subsidy",
                version,
                indexes,
                cached_starts,
            )?,
            fees: ValuePerBlockFull::forced_import(db, "fees", version, indexes, cached_starts)?,
            output_volume: EagerVec::forced_import(db, "output_volume", version)?,
            unclaimed: ValuePerBlockCumulative::forced_import(
                db,
                "unclaimed_rewards",
                version,
                indexes,
            )?,
            fee_dominance,
            subsidy_dominance,
            fee_to_subsidy: PercentRollingWindows::forced_import(
                db,
                "fee_to_subsidy",
                version + Version::ONE,
                indexes,
            )?,
        })
    }
}
