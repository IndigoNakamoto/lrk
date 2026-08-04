use brk_error::Result;
use brk_types::{Cents, Height, Version};
use vecdb::{CachedBoxedVec, Database};

use super::{Vecs, sma::SmaVecs, vecs::EmaVecs};
use crate::{blocks, indexes, internal::PriceWithRatioPerBlock};

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexes: &indexes::Vecs,
        blocks: &blocks::Vecs,
        spot_price: &CachedBoxedVec<Height, Cents>,
    ) -> Result<Self> {
        macro_rules! import_ema {
            ($name:expr) => {
                PriceWithRatioPerBlock::forced_import(db, $name, version, indexes, spot_price)?
            };
        }

        let sma = SmaVecs::new(version, indexes, &blocks.lookback, spot_price.clone());

        let ema = EmaVecs {
            _1w: import_ema!("price_ema_1w"),
            _8d: import_ema!("price_ema_8d"),
            _12d: import_ema!("price_ema_12d"),
            _13d: import_ema!("price_ema_13d"),
            _21d: import_ema!("price_ema_21d"),
            _26d: import_ema!("price_ema_26d"),
            _1m: import_ema!("price_ema_1m"),
            _34d: import_ema!("price_ema_34d"),
            _55d: import_ema!("price_ema_55d"),
            _89d: import_ema!("price_ema_89d"),
            _144d: import_ema!("price_ema_144d"),
            _200d: import_ema!("price_ema_200d"),
            _1y: import_ema!("price_ema_1y"),
            _2y: import_ema!("price_ema_2y"),
            _200w: import_ema!("price_ema_200w"),
            _4y: import_ema!("price_ema_4y"),
        };

        Ok(Self { sma, ema })
    }
}
