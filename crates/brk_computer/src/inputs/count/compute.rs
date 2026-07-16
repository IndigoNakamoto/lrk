use brk_error::Result;
use brk_indexer::Indexer;
use vecdb::Exit;

use super::Vecs;
use crate::blocks;

impl Vecs {
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        blocks: &blocks::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        let starting_height = indexer.safe_lengths().height;
        let window_starts = blocks.lookback.window_starts();

        self.sum.compute_count_from_indexes(
            starting_height,
            &indexer.vecs.inputs.first_txin_index,
            &indexer.vecs.inputs.outpoint,
            exit,
        )?;
        self.compute_rest(starting_height, &window_starts, exit)?;

        Ok(())
    }
}
