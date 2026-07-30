use brk_traversable::Traversable;
use brk_types::{StoredU64, Weight};
use vecdb::{Rw, StorageMode};

use crate::internal::{PerBlockFull, PerBlockRolling};

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    pub vbytes: PerBlockFull<StoredU64, Weight, M>,
    pub size: PerBlockRolling<StoredU64, M>,
}
