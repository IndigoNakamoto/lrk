use brk_cohort::{AgeRange, CohortContext};
use brk_error::Result;
use brk_types::{Cents, Height, Version};
use vecdb::{CachedBoxedVec, Database, ReadableCloneableVec};

use super::{ActivityVecs, CohortVecs, SupplyVecs, Vecs};
use crate::{
    indexes,
    internal::{
        CachedWindowStartVec, LazyPerBlock, OddsF64, OneMinusF64, PerBlock,
        PerBlockCumulativeRolling, SpotValuePerBlock, Windows,
    },
};

const VERSION: Version = Version::ONE;

impl ActivityVecs {
    fn forced_import(
        db: &Database,
        name: &str,
        version: Version,
        indexes: &indexes::Vecs,
    ) -> Result<Self> {
        let wakefulness =
            PerBlock::forced_import(db, &format!("{name}_wakefulness"), version, indexes)?;
        let dormancy = LazyPerBlock::from_computed::<OneMinusF64>(
            &format!("{name}_dormancy"),
            version,
            wakefulness.height.read_only_boxed_clone(),
            &wakefulness,
        );
        let wakefulness_to_dormancy = LazyPerBlock::from_computed::<OddsF64>(
            &format!("{name}_wakefulness_to_dormancy"),
            version,
            wakefulness.height.read_only_boxed_clone(),
            &wakefulness,
        );

        Ok(Self {
            wakefulness,
            dormancy,
            wakefulness_to_dormancy,
        })
    }
}

impl SupplyVecs {
    fn forced_import(
        db: &Database,
        name: &str,
        version: Version,
        indexes: &indexes::Vecs,
        spot_price: &CachedBoxedVec<Height, Cents>,
    ) -> Result<Self> {
        Ok(Self {
            awake: SpotValuePerBlock::forced_import(
                db,
                &format!("{name}_awake_supply"),
                version,
                indexes,
                spot_price,
            )?,
            dormant: SpotValuePerBlock::forced_import(
                db,
                &format!("{name}_dormant_supply"),
                version,
                indexes,
                spot_price,
            )?,
        })
    }
}

impl CohortVecs {
    fn forced_import(
        db: &Database,
        name: &str,
        version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&CachedWindowStartVec>,
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
            activity: ActivityVecs::forced_import(db, name, version, indexes)?,
            supply: SupplyVecs::forced_import(db, name, version, indexes, spot_price)?,
        })
    }
}

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        parent_version: Version,
        indexes: &indexes::Vecs,
        cached_starts: &Windows<&CachedWindowStartVec>,
        spot_price: &CachedBoxedVec<Height, Cents>,
    ) -> Result<Self> {
        let version = parent_version + VERSION;

        Ok(Self(AgeRange::try_new(|_, name| {
            let name = CohortContext::Utxo.prefixed(name);
            CohortVecs::forced_import(db, &name, version, indexes, cached_starts, spot_price)
        })?))
    }
}
