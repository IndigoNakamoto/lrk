use brk_error::Result;
use brk_types::{Height, StoredU64, Version};
use vecdb::Database;

use super::{Vecs, WithInputTypes};
use crate::{
    indexes,
    internal::{CachedWindowStartVec, Windows},
};

fn identity(_: Height, value: StoredU64) -> StoredU64 {
    value
}

fn without_coinbase(height: Height, total: StoredU64) -> StoredU64 {
    total - StoredU64::from(height.incremented())
}

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&CachedWindowStartVec>,
    ) -> Result<Self> {
        let input_count_source = indexes.input_count_source();
        let input_count = WithInputTypes::forced_import_counts(
            db,
            "input_count_bis",
            |t| format!("{t}_prevout_count"),
            version,
            (input_count_source, identity),
            indexes,
            cached_starts,
        )?;
        let input_share = input_count.lazy_shares(
            version,
            |name| format!("{name}_prevout_share"),
            cached_starts,
            indexes,
        );
        let transaction_count_source = indexes.transaction_count_source();
        let tx_count = WithInputTypes::forced_import(
            db,
            "non_coinbase_tx_count",
            |t| format!("tx_count_with_{t}_prevout"),
            version,
            (transaction_count_source, without_coinbase),
            indexes,
            cached_starts,
        )?;
        let tx_share = tx_count.lazy_shares(
            version,
            |name| format!("tx_share_with_{name}_prevout"),
            cached_starts,
            indexes,
        );
        Ok(Self {
            input_count,
            input_share,
            tx_count,
            tx_share,
        })
    }
}
