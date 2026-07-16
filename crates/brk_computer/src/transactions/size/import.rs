use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::{Version, Weight};
use vecdb::{Database, LazyVecFrom1, ReadableCloneableVec};

use super::Vecs;
use crate::{
    indexes,
    internal::{Identity, LazyPerTxDistribution, LazyPerTxDistributionTransformed, WeightToVSize},
};

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexer: &Indexer,
        indexes: &indexes::Vecs,
    ) -> Result<Self> {
        let tx_index_to_weight = LazyVecFrom1::transformed::<Identity<Weight>>(
            "tx_weight",
            version,
            indexer.vecs.transactions.weight.read_only_boxed_clone(),
        );

        let weight = LazyPerTxDistribution::forced_import(
            db,
            "tx_weight",
            version,
            indexes,
            tx_index_to_weight,
        )?;

        let tx_index_to_vsize = LazyVecFrom1::transformed::<WeightToVSize>(
            "tx_vsize",
            version,
            indexer.vecs.transactions.weight.read_only_boxed_clone(),
        );

        let vsize = LazyPerTxDistributionTransformed::new::<WeightToVSize>(
            "tx_vsize",
            version,
            tx_index_to_vsize,
            &weight.distribution,
        );

        Ok(Self { vsize, weight })
    }
}
