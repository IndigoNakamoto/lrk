use brk_traversable::Traversable;
use brk_types::StoredU64;
use vecdb::{Rw, StorageMode};

use crate::internal::PerBlockFullFromCumulative;

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    pub total: PerBlockFullFromCumulative<StoredU64, M>,
}
