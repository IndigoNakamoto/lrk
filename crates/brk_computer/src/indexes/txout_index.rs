use brk_indexer::Indexer;
use brk_traversable::Traversable;
use brk_types::{Sats, TxOutIndex, Version};
use vecdb::{LazyVec, ReadableCloneableVec};

#[derive(Clone, Traversable)]
pub struct Vecs {
    pub identity: LazyVec<TxOutIndex, TxOutIndex, TxOutIndex, Sats>,
}

impl Vecs {
    pub(crate) fn forced_import(version: Version, indexer: &Indexer) -> Self {
        Self {
            identity: LazyVec::init(
                "txout_index",
                version,
                indexer.vecs.outputs.value.read_only_boxed_clone(),
                |index, _| index,
            ),
        }
    }
}
