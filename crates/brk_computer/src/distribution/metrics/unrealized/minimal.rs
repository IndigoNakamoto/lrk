use brk_error::Result;
use brk_traversable::Traversable;
use brk_types::{Cents, Height, PartsPerMillionSigned32, Version};
use vecdb::{Exit, ReadableVec, Rw, StorageMode};

use crate::internal::RatioPerBlock;

use crate::distribution::metrics::ImportConfig;

#[derive(Traversable)]
pub struct UnrealizedMinimal<M: StorageMode = Rw> {
    pub nupl: RatioPerBlock<PartsPerMillionSigned32, M>,
}

impl UnrealizedMinimal {
    pub(crate) fn forced_import(cfg: &ImportConfig) -> Result<Self> {
        Ok(Self {
            nupl: RatioPerBlock::forced_import_ppm(
                cfg.db,
                &cfg.name("nupl"),
                cfg.version + Version::ONE,
                cfg.indexes,
            )?,
        })
    }

    pub(crate) fn compute(
        &mut self,
        max_from: Height,
        spot_price: &impl ReadableVec<Height, Cents>,
        realized_price: &impl ReadableVec<Height, Cents>,
        exit: &Exit,
    ) -> Result<()> {
        self.nupl.ppm.height.compute_transform2(
            max_from,
            spot_price,
            realized_price,
            |(i, price, realized_price, ..)| {
                let p = price.as_u128();
                if p == 0 {
                    (i, PartsPerMillionSigned32::ZERO)
                } else {
                    let rp = realized_price.as_u128();
                    let ratio = (p as f64 - rp as f64) / p as f64;
                    (i, PartsPerMillionSigned32::from(ratio))
                }
            },
            exit,
        )?;
        Ok(())
    }
}
