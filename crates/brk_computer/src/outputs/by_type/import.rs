use brk_error::Result;
use brk_types::Version;
use vecdb::Database;

use super::{CachedSpendableOutputCount, Vecs, WithOutputTypes, with_output_types::identity};
use crate::{
    indexes,
    internal::{CachedWindowStartVec, Windows},
};

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&CachedWindowStartVec>,
    ) -> Result<Self> {
        let output_count_source = indexes.output_count_source();
        let output_count = WithOutputTypes::forced_import_counts(
            db,
            "output_count_bis",
            |t| format!("{t}_output_count"),
            version,
            (output_count_source, identity),
            indexes,
            cached_starts,
        )?;
        let output_share = output_count.lazy_shares(
            version,
            |name| format!("{name}_output_share"),
            cached_starts,
            indexes,
        );
        let transaction_count_source = indexes.transaction_count_source();
        let tx_count = WithOutputTypes::forced_import(
            db,
            "tx_count_bis",
            |t| format!("tx_count_with_{t}_output"),
            version,
            (transaction_count_source, identity),
            indexes,
            cached_starts,
        )?;
        let tx_share = tx_count.lazy_shares(
            version,
            |name| format!("tx_share_with_{name}_output"),
            cached_starts,
            indexes,
        );

        let op_return_count = output_count
            .by_type
            .unspendable
            .op_return
            .cached_cumulative();
        let spendable_output_count =
            CachedSpendableOutputCount::new(version, &op_return_count, indexes, cached_starts);

        Ok(Self {
            output_count,
            spendable_output_count,
            output_share,
            tx_count,
            tx_share,
        })
    }
}
