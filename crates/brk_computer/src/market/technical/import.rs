use brk_error::Result;
use brk_types::{PartsPerMillionSigned64, StoredF32, Version};
use vecdb::{Database, UnaryTransform};

use super::{MacdChain, RsiChain, Vecs};
use crate::{
    indexes,
    internal::{LazyPerBlock, PerBlock, PercentPerBlock, RatioPerBlock, WindowsTo1m},
};

const VERSION: Version = Version::new(4);

struct RsiGain;

impl UnaryTransform<StoredF32, StoredF32> for RsiGain {
    fn apply(value: StoredF32) -> StoredF32 {
        StoredF32::from((*value).max(0.0))
    }
}

struct RsiLoss;

impl UnaryTransform<StoredF32, StoredF32> for RsiLoss {
    fn apply(value: StoredF32) -> StoredF32 {
        StoredF32::from((-*value).max(0.0))
    }
}

impl RsiChain {
    fn forced_import(
        db: &Database,
        tf: &str,
        version: Version,
        indexes: &indexes::Vecs,
        returns: &LazyPerBlock<StoredF32, PartsPerMillionSigned64>,
    ) -> Result<Self> {
        macro_rules! import {
            ($name:expr) => {
                PerBlock::forced_import(db, &format!("rsi_{}_{}", $name, tf), version, indexes)?
            };
        }

        macro_rules! percent_import {
            ($name:expr) => {
                PercentPerBlock::forced_import(
                    db,
                    &format!("rsi_{}_{}", $name, tf),
                    version,
                    indexes,
                )?
            };
        }

        let average_gain = import!("average_gain");
        let average_loss = import!("average_loss");

        let rsi = PercentPerBlock::forced_import(db, &format!("rsi_{tf}"), version, indexes)?;

        Ok(Self {
            gains: LazyPerBlock::from_lazy::<RsiGain, PartsPerMillionSigned64>(
                &format!("rsi_gains_{tf}"),
                version,
                returns,
            ),
            losses: LazyPerBlock::from_lazy::<RsiLoss, PartsPerMillionSigned64>(
                &format!("rsi_losses_{tf}"),
                version,
                returns,
            ),
            average_gain,
            average_loss,
            rsi,
            rsi_min: percent_import!("min"),
            rsi_max: percent_import!("max"),
            stoch_rsi: percent_import!("stoch"),
            stoch_rsi_k: percent_import!("stoch_k"),
            stoch_rsi_d: percent_import!("stoch_d"),
        })
    }
}

impl MacdChain {
    fn forced_import(
        db: &Database,
        tf: &str,
        version: Version,
        indexes: &indexes::Vecs,
    ) -> Result<Self> {
        let line = PerBlock::forced_import(db, &format!("macd_line_{tf}"), version, indexes)?;
        let signal = PerBlock::forced_import(db, &format!("macd_signal_{tf}"), version, indexes)?;

        let histogram =
            PerBlock::forced_import(db, &format!("macd_histogram_{tf}"), version, indexes)?;

        Ok(Self {
            ema_fast: PerBlock::forced_import(
                db,
                &format!("macd_ema_fast_{tf}"),
                version,
                indexes,
            )?,
            ema_slow: PerBlock::forced_import(
                db,
                &format!("macd_ema_slow_{tf}"),
                version,
                indexes,
            )?,
            line,
            signal,
            histogram,
        })
    }
}

impl Vecs {
    pub(crate) fn forced_import(
        db: &Database,
        version: Version,
        indexes: &indexes::Vecs,
        returns: &LazyPerBlock<StoredF32, PartsPerMillionSigned64>,
    ) -> Result<Self> {
        let v = version + VERSION;

        let rsi = WindowsTo1m::try_from_fn(|tf| {
            RsiChain::forced_import(db, tf, v + Version::TWO, indexes, returns)
        })?;
        let macd = WindowsTo1m::try_from_fn(|tf| MacdChain::forced_import(db, tf, v, indexes))?;

        let pi_cycle = RatioPerBlock::forced_import_ppm(db, "pi_cycle", v, indexes)?;

        Ok(Self {
            rsi,
            pi_cycle,
            macd,
        })
    }
}
