use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::StoredU64;
use vecdb::Exit;

use super::Vecs;
use crate::indexes;

impl Vecs {
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        indexes: &indexes::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        self.total.compute_cumulative_sum_from_indexes(
            indexer.safe_lengths().height,
            &indexer.vecs.transactions.first_tx_index,
            &indexes.height.tx_index_count,
            &indexer.vecs.transactions.total_sigop_cost,
            |value| StoredU64::from(u64::from(u32::from(value))),
            exit,
        )
    }
}
