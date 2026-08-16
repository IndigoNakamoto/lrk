pub mod by_type;
pub mod count;

mod compute;
mod import;
mod value;

use brk_traversable::Traversable;
use brk_types::{Sats, TxInIndex};
use vecdb::{Database, PcoVec, Rw, StorageMode};

use crate::internal::LazyPerSecondWindows;

pub use by_type::Vecs as ByTypeVecs;
pub use count::Vecs as CountVecs;

pub const DB_NAME: &str = "inputs";

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    #[traversable(skip)]
    pub(crate) db: Database,

    pub value: M::Stored<PcoVec<TxInIndex, Sats>>,
    pub count: CountVecs<M>,
    pub per_sec: LazyPerSecondWindows,
    pub by_type: ByTypeVecs<M>,
}
