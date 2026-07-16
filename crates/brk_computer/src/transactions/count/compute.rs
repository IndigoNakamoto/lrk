use brk_error::Result;
use brk_indexer::Indexer;
use vecdb::Exit;

use super::Vecs;
use crate::{blocks, indexes};

impl Vecs {
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        indexes: &indexes::Vecs,
        lookback: &blocks::LookbackVecs,
        exit: &Exit,
    ) -> Result<()> {
        let starting_height = indexer.safe_lengths().height;

        let window_starts = lookback.window_starts();
        self.total
            .compute(starting_height, &window_starts, exit, |height| {
                Ok(height.compute_transform(
                    starting_height,
                    &indexes.height.tx_index_count,
                    |(height, count, ..)| (height, count),
                    exit,
                )?)
            })?;

        Ok(())
    }
}
