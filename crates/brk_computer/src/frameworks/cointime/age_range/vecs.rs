use brk_cohort::AgeRange;
use brk_traversable::Traversable;
use brk_types::StoredF64;
use derive_more::{Deref, DerefMut};
use vecdb::{Rw, StorageMode};

use crate::internal::PerBlockCumulativeRolling;

use super::super::{SupplyBaseVecs, activity::DerivedVecs as ActivityDerivedVecs};

#[derive(Deref, DerefMut, Traversable)]
pub struct CohortVecs<M: StorageMode = Rw> {
    pub coindays_created: PerBlockCumulativeRolling<StoredF64, M>,
    pub coindays_consumed: PerBlockCumulativeRolling<StoredF64, M>,
    pub coindays_stored: PerBlockCumulativeRolling<StoredF64, M>,
    #[deref]
    #[deref_mut]
    #[traversable(flatten)]
    pub activity: ActivityDerivedVecs<M>,
    pub supply: SupplyBaseVecs<M>,
}

#[derive(Deref, DerefMut, Traversable)]
#[traversable(transparent)]
pub struct Vecs<M: StorageMode = Rw>(pub AgeRange<CohortVecs<M>>);
