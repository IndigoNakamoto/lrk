use brk_error::Result;
use brk_types::Version;
use vecdb::{Database, ReadableCloneableVec};

use super::{
    Vecs,
    vecs::{HashPriceValueVecs, HashRateSmaVecs, RateVecs},
};
use crate::{
    indexes,
    internal::{LazyPerBlock, PerBlock, PercentPerBlock, ThsToPhsF32},
};

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexes: &indexes::Vecs,
    ) -> Result<Self> {
        // v6: Litecoin hashrate scale (powLimit difficulty × 2^32/150/4096, 576 blocks/day).
        let v6 = Version::new(6);

        let price_ths = PerBlock::forced_import(db, "hash_price_ths", version + v6, indexes)?;
        let price_ths_min =
            PerBlock::forced_import(db, "hash_price_ths_min", version + v6, indexes)?;
        let price_phs = LazyPerBlock::from_computed::<ThsToPhsF32>(
            "hash_price_phs",
            version + v6,
            price_ths.height.read_only_boxed_clone(),
            &price_ths,
        );
        let price_phs_min = LazyPerBlock::from_computed::<ThsToPhsF32>(
            "hash_price_phs_min",
            version + v6,
            price_ths_min.height.read_only_boxed_clone(),
            &price_ths_min,
        );

        let value_ths = PerBlock::forced_import(db, "hash_value_ths", version + v6, indexes)?;
        let value_ths_min =
            PerBlock::forced_import(db, "hash_value_ths_min", version + v6, indexes)?;
        let value_phs = LazyPerBlock::from_computed::<ThsToPhsF32>(
            "hash_value_phs",
            version + v6,
            value_ths.height.read_only_boxed_clone(),
            &value_ths,
        );
        let value_phs_min = LazyPerBlock::from_computed::<ThsToPhsF32>(
            "hash_value_phs_min",
            version + v6,
            value_ths_min.height.read_only_boxed_clone(),
            &value_ths_min,
        );

        Ok(Self {
            rate: RateVecs {
                base: PerBlock::forced_import(db, "hash_rate", version + v6, indexes)?,
                sma: HashRateSmaVecs {
                    _1w: PerBlock::forced_import(db, "hash_rate_sma_1w", version + v6, indexes)?,
                    _1m: PerBlock::forced_import(db, "hash_rate_sma_1m", version + v6, indexes)?,
                    _2m: PerBlock::forced_import(db, "hash_rate_sma_2m", version + v6, indexes)?,
                    _1y: PerBlock::forced_import(db, "hash_rate_sma_1y", version + v6, indexes)?,
                },
                ath: PerBlock::forced_import(db, "hash_rate_ath", version + v6, indexes)?,
                drawdown: PercentPerBlock::forced_import(
                    db,
                    "hash_rate_drawdown",
                    version + v6,
                    indexes,
                )?,
            },
            price: HashPriceValueVecs {
                ths: price_ths,
                ths_min: price_ths_min,
                phs: price_phs,
                phs_min: price_phs_min,
                rebound: PercentPerBlock::forced_import(
                    db,
                    "hash_price_rebound",
                    version + v6,
                    indexes,
                )?,
            },
            value: HashPriceValueVecs {
                ths: value_ths,
                ths_min: value_ths_min,
                phs: value_phs,
                phs_min: value_phs_min,
                rebound: PercentPerBlock::forced_import(
                    db,
                    "hash_value_rebound",
                    version + v6,
                    indexes,
                )?,
            },
        })
    }
}
