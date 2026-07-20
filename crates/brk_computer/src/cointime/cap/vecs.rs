use brk_traversable::Traversable;
use brk_types::{Cents, PartsPerMillion32};
use vecdb::{Rw, StorageMode};

use crate::internal::{FiatPerBlock, RatioPerBlock};

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    pub thermo: FiatPerBlock<Cents, M>,
    pub investor: FiatPerBlock<Cents, M>,
    pub vaulted: FiatPerBlock<Cents, M>,
    pub active: FiatPerBlock<Cents, M>,
    pub cointime: FiatPerBlock<Cents, M>,
    pub aviv: RatioPerBlock<PartsPerMillion32, M>,
}
