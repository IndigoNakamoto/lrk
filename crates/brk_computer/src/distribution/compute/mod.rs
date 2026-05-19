mod block_loop;
mod context;
mod readers;
mod recover;
mod write;

pub(crate) use block_loop::process_blocks;
pub(crate) use context::{ComputeContext, PriceRangeMax};
pub(crate) use readers::{IndexToTxIndexBuf, TxInReaders, TxOutData, TxOutReaders, VecsReaders};
pub(crate) use recover::{StartMode, determine_start_mode, recover_state, reset_state};

/// Flush checkpoint interval (every N blocks).
pub const FLUSH_INTERVAL: usize = 10_000;

