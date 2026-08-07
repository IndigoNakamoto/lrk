use brk_error::Result;
use brk_indexer::Indexer;
use vecdb::Exit;

use super::Vecs;

impl Vecs {
    pub(crate) fn compute(&mut self, indexer: &Indexer, exit: &Exit) -> Result<()> {
        let starting_height = indexer.safe_lengths().height;
        let source = &indexer.vecs.transaction_features.count;

        for (metrics, source) in [
            (&mut self.count.inscription, &source.inscription),
            (&mut self.count.annex, &source.annex),
            (&mut self.count.sighash_all, &source.sighash_all),
            (&mut self.count.sighash_none, &source.sighash_none),
            (&mut self.count.sighash_single, &source.sighash_single),
            (&mut self.count.sighash_default, &source.sighash_default),
            (
                &mut self.count.sighash_anyone_can_pay,
                &source.sighash_anyone_can_pay,
            ),
            (&mut self.count.dust_output, &source.dust_output),
        ] {
            metrics.compute_cumulative(starting_height, source, exit)?;
        }

        Ok(())
    }
}
