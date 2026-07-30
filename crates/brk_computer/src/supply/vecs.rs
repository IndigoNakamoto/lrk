use brk_traversable::Traversable;
use brk_types::{Cents, CentsSigned, PartsPerMillionSigned64};
use vecdb::{Database, Rw, StorageMode};

use super::{burned, velocity};
use crate::internal::{
    LazyFiatPerBlock, LazyPercentPerBlock, LazyRollingDeltasFiatFromHeight, LazyValuePerBlock,
    RollingWindows,
};

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    #[traversable(skip)]
    pub(crate) db: Database,

    pub circulating: LazyValuePerBlock,
    pub burned: burned::Vecs<M>,
    pub inflation_rate: LazyPercentPerBlock<PartsPerMillionSigned64>,
    pub velocity: velocity::Vecs<M>,
    pub market_cap: LazyFiatPerBlock<Cents>,
    #[traversable(wrap = "market_cap", rename = "delta")]
    pub market_cap_delta:
        LazyRollingDeltasFiatFromHeight<Cents, CentsSigned, PartsPerMillionSigned64>,
    pub market_minus_realized_cap_growth_rate: RollingWindows<PartsPerMillionSigned64, M>,
    pub hodled_or_lost: LazyValuePerBlock,
}
