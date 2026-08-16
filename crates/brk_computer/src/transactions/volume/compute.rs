use brk_error::Result;
use brk_indexer::Indexer;
use vecdb::Exit;

use super::Vecs;
use crate::transactions::fees;
use crate::{indexes, price};

impl Vecs {
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        indexes: &indexes::Vecs,
        prices: &price::Vecs,
        fees_vecs: &fees::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        let starting_height = indexer.safe_lengths().height;

        self.transfer_volume.compute_filtered_from_indexes(
            starting_height,
            prices,
            &indexer.vecs().transactions.first_tx_index,
            &indexes.height.tx_index_count,
            &fees_vecs.input_value,
            |sats| !sats.is_max(),
            exit,
        )?;

        Ok(())
    }
}
