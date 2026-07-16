use brk_types::BlockHash;

use super::{txin::InputResolver, txout::BlockAddresses};

/// Reusable buffers cleared and refilled each block to avoid allocation churn.
#[derive(Default)]
pub(crate) struct BlockBuffers {
    pub(crate) inputs: InputResolver,
    pub(crate) addresses: BlockAddresses,
    tip: Option<BlockHash>,
}

impl BlockBuffers {
    pub(crate) fn continue_from(&mut self, parent: Option<BlockHash>) {
        if self.tip != parent {
            self.addresses.clear_cache();
        }
        self.tip = parent;
    }

    pub(crate) fn finish_block(&mut self, blockhash: BlockHash) {
        self.tip = Some(blockhash);
    }

    pub(crate) fn reset(&mut self) {
        self.addresses.clear_cache();
        self.tip = None;
    }
}
