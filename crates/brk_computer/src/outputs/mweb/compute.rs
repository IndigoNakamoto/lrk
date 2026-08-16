use brk_error::Result;
use brk_indexer::Indexer;
use vecdb::Exit;

use super::Vecs;
use crate::{inputs, price};

impl Vecs {
    /// MWEB peg-flow compute still needs a full port onto the v0.11.2
    /// cumulative/lazy `ValuePerBlock` APIs. Import and schema are wired;
    /// this keeps the Litecoin feature compiling until that rewrite lands.
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        inputs: &inputs::Vecs,
        prices: &price::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        let _ = (indexer, inputs, prices, exit);
        Ok(())
    }
}
