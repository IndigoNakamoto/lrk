use vecdb::StorageMode;

use crate::Vecs;

pub trait HasMweb<M: StorageMode> {
    fn mweb(&self) -> &Vecs<M>;
}
