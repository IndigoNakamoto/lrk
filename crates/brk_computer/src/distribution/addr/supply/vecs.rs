use brk_error::Result;
use brk_traversable::Traversable;
use brk_types::{Cents, Height, Version};
use derive_more::{Deref, DerefMut};
use vecdb::{CachedBoxedVec, Database, Rw, StorageMode};

use crate::{
    indexes,
    internal::{SpotValuePerBlock, WithAddrTypes},
};

use super::AddrTypeToSupply;

/// Per-addr-type running supply (sats/btc/cents/usd) with an aggregated `all`.
/// Shared across predicate-based supply categories (exposed, reused, respent).
/// Sats are pushed stateful per block; cents/usd are derived post-hoc from
/// sats × spot price.
#[derive(Deref, DerefMut, Traversable)]
pub struct AddrSupplyVecs<M: StorageMode = Rw>(
    #[traversable(flatten)] pub WithAddrTypes<SpotValuePerBlock<M>>,
);

impl AddrSupplyVecs {
    pub(crate) fn forced_import(
        db: &Database,
        name: &str,
        version: Version,
        indexes: &indexes::Vecs,
        spot_price: &CachedBoxedVec<Height, Cents>,
    ) -> Result<Self> {
        Ok(Self(WithAddrTypes::<SpotValuePerBlock>::forced_import(
            db,
            &format!("{name}_addr_supply"),
            version,
            indexes,
            spot_price,
        )?))
    }

    #[inline(always)]
    pub(crate) fn push_supply(&mut self, supply: &AddrTypeToSupply) {
        self.push_height(supply.sum(), supply.values().copied());
    }
}
