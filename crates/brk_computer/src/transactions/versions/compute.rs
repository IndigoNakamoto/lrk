use brk_error::Result;
use brk_indexer::Indexer;
use vecdb::Exit;

use super::Vecs;

impl Vecs {
    pub(crate) fn compute(&mut self, indexer: &Indexer, exit: &Exit) -> Result<()> {
        let lengths = indexer.safe_lengths();
        let starting_height = lengths.height;
        let counts = &indexer.vecs.transaction_features.count;

        for (metrics, source) in [
            (&mut self.v1, &counts.v1),
            (&mut self.v2, &counts.v2),
            (&mut self.v3, &counts.v3),
            (&mut self.other, &counts.other_version),
        ] {
            metrics.compute(starting_height, exit, |block| {
                Ok(block.compute_transform(
                    starting_height,
                    source,
                    |(height, count, ..)| (height, count),
                    exit,
                )?)
            })?;
        }

        Ok(())
    }
}
