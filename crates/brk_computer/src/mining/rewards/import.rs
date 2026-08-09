use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::{PartsPerMillion32, PartsPerMillion64, Sats, Version};
use vecdb::{AnyVec, Database, EagerVec, ImportableVec};

use super::Vecs;
use crate::{
    indexes,
    internal::{
        CachedValuePerBlockFull, CachedWindowStartVec, LazyPercentCumulativeRolling,
        LazyPercentRollingWindows, OneMinusPpm, RatioSats, ValuePerBlockCumulative,
        ValuePerBlockCumulativeRolling, Windows,
    },
};

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexer: &Indexer,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&CachedWindowStartVec>,
    ) -> Result<Self> {
        let coinbase_version = version
            + indexer.vecs().transactions.first_txout_index.version()
            + indexes.tx_index.output_count.version()
            + indexer.vecs().outputs.value.version();

        let coinbase = ValuePerBlockCumulativeRolling::forced_import(
            db,
            "coinbase",
            coinbase_version,
            indexes,
            cached_starts,
        )?;
        let subsidy = ValuePerBlockCumulativeRolling::forced_import(
            db,
            "subsidy",
            version,
            indexes,
            cached_starts,
        )?;
        let fees =
            CachedValuePerBlockFull::forced_import(db, "fees", version, indexes, cached_starts)?;
        let cached_fees = fees.cached_cumulative_sats();

        let fee_dominance =
            LazyPercentCumulativeRolling::from_cumulative_ratio_with_cached_numerator::<
                Sats,
                Sats,
                RatioSats<PartsPerMillion32>,
            >(
                "fee_dominance",
                version,
                cached_fees.clone(),
                &coinbase.cumulative.sats.height,
                cached_starts,
                indexes,
            );
        let subsidy_dominance = LazyPercentCumulativeRolling::from_lazy_source::<OneMinusPpm>(
            "subsidy_dominance",
            version,
            &fee_dominance,
        );
        let fee_to_subsidy = LazyPercentRollingWindows::from_cumulative_ratio_with_cached_numerator::<
            Sats,
            Sats,
            RatioSats<PartsPerMillion64>,
        >(
            "fee_to_subsidy",
            version + Version::ONE,
            cached_fees,
            &subsidy.cumulative.sats.height,
            cached_starts,
            indexes,
        );

        Ok(Self {
            coinbase,
            subsidy,
            fees,
            output_volume: EagerVec::forced_import(db, "output_volume", version)?,
            unclaimed: ValuePerBlockCumulative::forced_import(
                db,
                "unclaimed_rewards",
                version,
                indexes,
            )?,
            fee_dominance,
            subsidy_dominance,
            fee_to_subsidy,
        })
    }
}
