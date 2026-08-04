use brk_cohort::TERM_NAMES;
use brk_error::Result;
use brk_types::{Cents, Height, StoredF64, Version};
use vecdb::{CachedBoxedVec, Database, ReadableCloneableVec};

use super::{AllAwakeVecs, AllCohortVecs, DormantVecs, StoredAwakeVecs, StoredCohortVecs, Vecs};
use crate::{
    indexes,
    internal::{
        FiatPerBlock, Identity, LazyPerBlock, PerBlock, PriceWithRatioPerBlock, SpotValuePerBlock,
    },
};

struct CommonVecs<M: vecdb::StorageMode = vecdb::Rw> {
    supply: SpotValuePerBlock<M>,
    dormant_supply: SpotValuePerBlock<M>,
    cap: FiatPerBlock<Cents, M>,
    price: PriceWithRatioPerBlock<M>,
}

impl CommonVecs {
    fn forced_import(
        db: &Database,
        name: &str,
        version: Version,
        indexes: &indexes::Vecs,
        spot_price: &CachedBoxedVec<Height, Cents>,
    ) -> Result<Self> {
        let prefix = if name.is_empty() {
            String::new()
        } else {
            format!("{name}_")
        };

        Ok(CommonVecs {
            supply: SpotValuePerBlock::forced_import(
                db,
                &format!("{prefix}awake_supply"),
                version,
                indexes,
                spot_price,
            )?,
            dormant_supply: SpotValuePerBlock::forced_import(
                db,
                &format!("{prefix}dormant_supply"),
                version,
                indexes,
                spot_price,
            )?,
            cap: FiatPerBlock::forced_import(db, &format!("{prefix}awake_cap"), version, indexes)?,
            price: PriceWithRatioPerBlock::forced_import(
                db,
                &format!("{prefix}awake_price"),
                version,
                indexes,
                spot_price,
            )?,
        })
    }
}

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexes: &indexes::Vecs,
        spot_price: &CachedBoxedVec<Height, Cents>,
        all_supply_in_loss_share: &PerBlock<StoredF64>,
    ) -> Result<Self> {
        let all_supply_in_loss_share = LazyPerBlock::from_computed::<Identity<StoredF64>>(
            "awake_supply_in_loss_share",
            version,
            all_supply_in_loss_share.height.read_only_boxed_clone(),
            all_supply_in_loss_share,
        );
        let stored = |name: &str| {
            PerBlock::forced_import(
                db,
                &format!("{name}_awake_supply_in_loss_share"),
                version,
                indexes,
            )
        };
        let all_common = CommonVecs::forced_import(db, "", version, indexes, spot_price)?;
        let sth_common =
            CommonVecs::forced_import(db, TERM_NAMES.short.id, version, indexes, spot_price)?;
        let lth_common =
            CommonVecs::forced_import(db, TERM_NAMES.long.id, version, indexes, spot_price)?;

        Ok(Self {
            all: AllCohortVecs {
                awake: AllAwakeVecs {
                    supply: all_common.supply,
                    supply_in_loss_share: all_supply_in_loss_share,
                    cap: all_common.cap,
                    price: all_common.price,
                },
                dormant: DormantVecs {
                    supply: all_common.dormant_supply,
                },
            },
            sth: StoredCohortVecs {
                awake: StoredAwakeVecs {
                    supply: sth_common.supply,
                    supply_in_loss_share: stored(TERM_NAMES.short.id)?,
                    cap: sth_common.cap,
                    price: sth_common.price,
                },
                dormant: DormantVecs {
                    supply: sth_common.dormant_supply,
                },
            },
            lth: StoredCohortVecs {
                awake: StoredAwakeVecs {
                    supply: lth_common.supply,
                    supply_in_loss_share: stored(TERM_NAMES.long.id)?,
                    cap: lth_common.cap,
                    price: lth_common.price,
                },
                dormant: DormantVecs {
                    supply: lth_common.dormant_supply,
                },
            },
        })
    }
}
