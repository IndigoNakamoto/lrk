use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::PartsPerMillion32;
use vecdb::Exit;

use super::Vecs;

impl Vecs {
    pub(crate) fn compute(&mut self, indexer: &Indexer, exit: &Exit) -> Result<()> {
        let starting_height = indexer.safe_lengths().height;
        self.fullness.raw.compute_transform(
            starting_height,
            &indexer.vecs.blocks.weight,
            |(h, weight, ..)| (h, PartsPerMillion32::from(weight.fullness())),
            exit,
        )?;

        Ok(())
    }
}
