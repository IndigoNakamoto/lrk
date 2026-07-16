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

        self.total.sum.compute_count_from_indexes(
            starting_height,
            &indexer.vecs.outputs.first_txout_index,
            &indexer.vecs.outputs.value,
            exit,
        )?;
        self.total
            .compute_rest(starting_height, &window_starts, exit)?;
        Ok(())
    }
}
