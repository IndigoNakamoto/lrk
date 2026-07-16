use brk_error::Result;
use brk_indexer::{Indexer, Lengths};
use brk_traversable::Traversable;
use brk_types::TxIndex;
use schemars::JsonSchema;
use vecdb::{Database, Exit, LazyVecFrom1, ReadableVec, Rw, StorageMode, Version};

use crate::{
    indexes,
    internal::{ComputedVecValue, NumericValue, TxDerivedDistribution},
};

#[derive(Traversable)]
pub struct LazyPerTxDistribution<T, S, M: StorageMode = Rw>
where
    T: ComputedVecValue + PartialOrd + JsonSchema,
    S: ComputedVecValue,
{
    pub tx_index: LazyVecFrom1<TxIndex, T, TxIndex, S>,
    #[traversable(flatten)]
    pub distribution: TxDerivedDistribution<T, M>,
}

impl<T, S> LazyPerTxDistribution<T, S>
where
    T: NumericValue + JsonSchema,
    S: ComputedVecValue + JsonSchema,
{
    pub(crate) fn forced_import(
        db: &Database,
        name: &str,
        version: Version,
        indexes: &indexes::Vecs,
        tx_index: LazyVecFrom1<TxIndex, T, TxIndex, S>,
    ) -> Result<Self> {
        let distribution = TxDerivedDistribution::forced_import(db, name, version, indexes)?;
        Ok(Self {
            tx_index,
            distribution,
        })
    }

    pub(crate) fn derive_from(
        &mut self,
        indexer: &Indexer,
        indexes: &indexes::Vecs,
        starting_lengths: &Lengths,
        exit: &Exit,
    ) -> Result<()>
    where
        T: Copy + Ord + From<f64> + Default,
        f64: From<T>,
        LazyVecFrom1<TxIndex, T, TxIndex, S>: ReadableVec<TxIndex, T>,
    {
        self.distribution
            .derive_from(indexer, indexes, starting_lengths, &self.tx_index, exit)
    }
}
