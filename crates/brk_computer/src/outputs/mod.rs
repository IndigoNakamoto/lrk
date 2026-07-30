pub mod by_type;
pub mod count;
pub mod spent;
pub mod unspent;
pub mod value;

mod compute;
mod import;

use brk_traversable::Traversable;
use vecdb::{Database, Rw, StorageMode};

use crate::internal::LazyPerSecondWindows;

pub use by_type::Vecs as ByTypeVecs;
pub use count::Vecs as CountVecs;
pub use spent::Vecs as SpentVecs;
pub use unspent::Vecs as UnspentVecs;
pub use value::Vecs as ValueVecs;

pub const DB_NAME: &str = "outputs";

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    #[traversable(skip)]
    pub(crate) db: Database,

    pub spent: SpentVecs<M>,
    pub count: CountVecs<M>,
    pub per_sec: LazyPerSecondWindows,
    pub unspent: UnspentVecs<M>,
    pub by_type: ByTypeVecs<M>,
    pub value: ValueVecs<M>,
}
