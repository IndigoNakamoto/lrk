use brk_traversable::Traversable;
use brk_types::StoredF64;
use derive_more::{Deref, DerefMut};
use vecdb::{Rw, StorageMode};

use crate::internal::{PerBlock, SpotValuePerBlock};

#[derive(Traversable)]
pub struct BaseVecs<M: StorageMode = Rw> {
    pub vaulted: SpotValuePerBlock<M>,
    pub active: SpotValuePerBlock<M>,
}

#[derive(Deref, DerefMut, Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    #[deref]
    #[deref_mut]
    #[traversable(flatten)]
    pub base: BaseVecs<M>,
    #[traversable(wrap = "active/in_loss", rename = "share")]
    pub active_supply_in_loss_share: PerBlock<StoredF64, M>,
}
