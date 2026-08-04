use brk_traversable::Traversable;
use brk_types::{Cents, StoredF64};
use vecdb::{Rw, StorageMode};

use crate::internal::{
    FiatPerBlock, LazyPerBlock, PerBlock, PriceWithRatioPerBlock, SpotValuePerBlock,
};

#[derive(Traversable)]
pub struct AllAwakeVecs<M: StorageMode = Rw> {
    pub supply: SpotValuePerBlock<M>,
    #[traversable(wrap = "supply/in_loss", rename = "share")]
    pub supply_in_loss_share: LazyPerBlock<StoredF64>,
    pub cap: FiatPerBlock<Cents, M>,
    pub price: PriceWithRatioPerBlock<M>,
}

#[derive(Traversable)]
pub struct StoredAwakeVecs<M: StorageMode = Rw> {
    pub supply: SpotValuePerBlock<M>,
    #[traversable(wrap = "supply/in_loss", rename = "share")]
    pub supply_in_loss_share: PerBlock<StoredF64, M>,
    pub cap: FiatPerBlock<Cents, M>,
    pub price: PriceWithRatioPerBlock<M>,
}

#[derive(Traversable)]
pub struct DormantVecs<M: StorageMode = Rw> {
    pub supply: SpotValuePerBlock<M>,
}

#[derive(Traversable)]
pub struct AllCohortVecs<M: StorageMode = Rw> {
    pub awake: AllAwakeVecs<M>,
    pub dormant: DormantVecs<M>,
}

#[derive(Traversable)]
pub struct StoredCohortVecs<M: StorageMode = Rw> {
    pub awake: StoredAwakeVecs<M>,
    pub dormant: DormantVecs<M>,
}

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    #[traversable(flatten)]
    pub all: AllCohortVecs<M>,
    pub sth: StoredCohortVecs<M>,
    pub lth: StoredCohortVecs<M>,
}
