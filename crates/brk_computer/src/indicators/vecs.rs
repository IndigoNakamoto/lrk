use brk_traversable::Traversable;
use brk_types::{PartsPerMillion32, PartsPerMillion64, StoredF32};
use vecdb::{Database, Rw, StorageMode};

use crate::internal::{PerBlock, PercentPerBlock, RatioPerBlock};

#[derive(Traversable)]
pub struct DormancyVecs<M: StorageMode = Rw> {
    pub supply_adj: PerBlock<StoredF32, M>,
    pub flow: PerBlock<StoredF32, M>,
}

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    #[traversable(skip)]
    pub(crate) db: Database,
    pub puell_multiple: RatioPerBlock<PartsPerMillion64, M>,
    pub nvt: RatioPerBlock<PartsPerMillion64, M>,
    pub gini: PercentPerBlock<PartsPerMillion32, M>,
    pub rhodl_ratio: RatioPerBlock<PartsPerMillion64, M>,
    pub thermo_cap_multiple: RatioPerBlock<PartsPerMillion64, M>,
    pub coindays_destroyed_supply_adj: PerBlock<StoredF32, M>,
    pub coinyears_destroyed_supply_adj: PerBlock<StoredF32, M>,
    pub dormancy: DormancyVecs<M>,
    pub stock_to_flow: PerBlock<StoredF32, M>,
    pub seller_exhaustion: PerBlock<StoredF32, M>,
}
