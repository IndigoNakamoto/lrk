use brk_error::Result;
use brk_indexer::Indexer;
use vecdb::Exit;

use super::Vecs;

impl Vecs {
    pub(crate) fn compute(&mut self, indexer: &Indexer, exit: &Exit) -> Result<()> {
        let starting_height = indexer.safe_lengths().height;
        self.total.cumulative.height.compute_cumulative_count(
            starting_height,
            &indexer.vecs.blocks.weight,
            |_| true,
            exit,
        )?;

        Ok(())
    }
}
