use brk_traversable::Traversable;
use brk_types::{StoredU64, Weight};
use vecdb::{Rw, StorageMode};

use crate::internal::{CachedPerBlockRolling, PerBlockFull};

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    pub vbytes: PerBlockFull<StoredU64, Weight, M>,
    pub size: CachedPerBlockRolling<StoredU64, M>,
}
