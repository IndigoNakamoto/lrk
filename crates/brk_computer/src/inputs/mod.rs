pub mod by_type;
pub mod count;

mod compute;
mod import;

use brk_traversable::Traversable;
use vecdb::{Database, Rw, StorageMode};

use crate::internal::LazyPerSecondWindows;

pub use by_type::Vecs as ByTypeVecs;
pub use count::Vecs as CountVecs;

pub const DB_NAME: &str = "inputs";

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    #[traversable(skip)]
    pub(crate) db: Database,

    pub count: CountVecs<M>,
    pub per_sec: LazyPerSecondWindows,
    pub by_type: ByTypeVecs<M>,
}
