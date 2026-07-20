use brk_traversable::Traversable;
use brk_types::{PartsPerMillionSigned64, StoredF64};
use vecdb::{Rw, StorageMode};

use crate::internal::{PerBlock, PercentPerBlock};

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    pub inflation_rate: PercentPerBlock<PartsPerMillionSigned64, M>,
    pub tx_velocity_native: PerBlock<StoredF64, M>,
    pub tx_velocity_fiat: PerBlock<StoredF64, M>,
}
