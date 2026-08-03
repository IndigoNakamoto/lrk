use brk_traversable::Traversable;
use vecdb::{Database, Rw, StorageMode};

use super::{coinflow, cointime};

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    #[traversable(skip)]
    pub(crate) db: Database,

    pub cointime: cointime::Vecs<M>,
    pub coinflow: coinflow::Vecs<M>,
}
