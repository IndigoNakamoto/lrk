use brk_cohort::{AgeRange, CohortContext};
use brk_error::Result;
use brk_types::{Cents, Height, Version};
use vecdb::{CachedBoxedVec, Database};

use super::{CohortVecs, Vecs};
use crate::{
    indexes,
    internal::{PerBlockCumulativeRolling, WindowStartVec, Windows},
};

use super::super::{SupplyBaseVecs, activity::DerivedVecs as ActivityDerivedVecs};

const VERSION: Version = Version::ONE;

impl CohortVecs {
    fn forced_import(
        db: &Database,
        name: &str,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
        spot_price: &CachedBoxedVec<Height, Cents>,
    ) -> Result<Self> {
        Ok(Self {
            coindays_created: PerBlockCumulativeRolling::forced_import(
                db,
                &format!("{name}_coindays_created"),
                version,
                indexes,
                cached_starts,
            )?,
            coindays_consumed: PerBlockCumulativeRolling::forced_import(
                db,
                &format!("{name}_coindays_consumed"),
                version,
                indexes,
                cached_starts,
            )?,
            coindays_stored: PerBlockCumulativeRolling::forced_import(
                db,
                &format!("{name}_coindays_stored"),
                version,
                indexes,
                cached_starts,
            )?,
            activity: ActivityDerivedVecs::forced_import_with_prefix(db, name, version, indexes)?,
            supply: SupplyBaseVecs::forced_import_with_prefix(
                db, name, version, indexes, spot_price,
            )?,
        })
    }
}

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        parent_version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&WindowStartVec>,
        spot_price: &CachedBoxedVec<Height, Cents>,
    ) -> Result<Self> {
        let version = parent_version + VERSION;

        Ok(Self(AgeRange::try_new(|_, name| {
            let name = CohortContext::Utxo.prefixed(name);
            CohortVecs::forced_import(db, &name, version, indexes, cached_starts, spot_price)
        })?))
    }
}
