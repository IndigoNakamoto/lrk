mod block;
mod buffer;
mod transaction;
mod txin;
mod txout;

pub(crate) use buffer::BlockBuffers;

use brk_types::{Block, Height};

use crate::{Lengths, Readers, Stores, Vecs};

/// Processes a single block, extracting and storing all indexed data.
pub(crate) struct BlockProcessor<'a> {
    pub(crate) block: &'a Block,
    pub(crate) height: Height,
    pub(crate) check_collisions: bool,
    pub(crate) lengths: &'a mut Lengths,
    pub(crate) vecs: &'a mut Vecs,
    pub(crate) stores: &'a mut Stores,
    pub(crate) readers: &'a Readers,
}
