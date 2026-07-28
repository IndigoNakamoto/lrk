pub mod activity;
pub mod adjusted;
pub mod age_range;
pub mod cap;
pub mod prices;
pub mod reserve_risk;
pub mod supply;
pub mod value;

mod compute;
mod import;

use brk_traversable::Traversable;
use vecdb::{Database, Rw, StorageMode};

pub use activity::Vecs as ActivityVecs;
pub use adjusted::Vecs as AdjustedVecs;
pub use age_range::Vecs as AgeRangeVecs;
pub use cap::Vecs as CapVecs;
pub use prices::Vecs as PricesVecs;
pub use reserve_risk::Vecs as ReserveRiskVecs;
pub use supply::{BaseVecs as SupplyBaseVecs, Vecs as SupplyVecs};
pub use value::Vecs as ValueVecs;

pub const DB_NAME: &str = "cointime";

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    #[traversable(skip)]
    pub(crate) db: Database,

    pub activity: ActivityVecs<M>,
    pub age_range: AgeRangeVecs<M>,
    pub supply: SupplyVecs<M>,
    pub value: ValueVecs<M>,
    pub cap: CapVecs<M>,
    pub prices: PricesVecs<M>,
    pub adjusted: AdjustedVecs<M>,
    pub reserve_risk: ReserveRiskVecs<M>,
}
