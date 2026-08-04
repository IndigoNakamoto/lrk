pub mod activity;
pub mod adjusted;
pub mod age_range;
pub mod aggregate;
pub mod cap;
pub mod prices;
pub mod reserve_risk;
pub mod supply;
pub mod value;

mod compute;
mod import;

use brk_traversable::Traversable;
use vecdb::{Rw, StorageMode};

pub use activity::Vecs as ActivityVecs;
pub use adjusted::Vecs as AdjustedVecs;
pub use age_range::Vecs as AgeRangeVecs;
pub use aggregate::Vecs as AggregateVecs;
pub use cap::Vecs as CapVecs;
pub use prices::Vecs as PricesVecs;
pub use reserve_risk::Vecs as ReserveRiskVecs;
pub use supply::Vecs as SupplyVecs;
pub use value::Vecs as ValueVecs;

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    pub activity: ActivityVecs<M>,
    pub age_range: AgeRangeVecs<M>,
    #[traversable(flatten)]
    pub aggregate: AggregateVecs<M>,
    pub supply: SupplyVecs<M>,
    pub value: ValueVecs<M>,
    pub cap: CapVecs<M>,
    pub prices: PricesVecs<M>,
    pub adjusted: AdjustedVecs<M>,
    pub reserve_risk: ReserveRiskVecs<M>,
}
