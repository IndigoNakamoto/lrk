use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::PartsPerMillionSigned64;
use vecdb::Exit;

use super::super::activity;
use super::Vecs;
use crate::supply;

impl Vecs {
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        supply: &supply::Vecs,
        activity: &activity::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        let starting_height = indexer.safe_lengths().height;

        self.inflation_rate.ppm.height.compute_transform2(
            starting_height,
            &activity.ratio.height,
            &supply.inflation_rate.ppm.height,
            |(h, a2vr, inflation, ..)| {
                (
                    h,
                    PartsPerMillionSigned64::from(f64::from(a2vr) * f64::from(inflation)),
                )
            },
            exit,
        )?;

        self.tx_velocity_native.height.compute_multiply(
            starting_height,
            &activity.ratio.height,
            &supply.velocity.native.height,
            exit,
        )?;

        self.tx_velocity_fiat.height.compute_multiply(
            starting_height,
            &activity.ratio.height,
            &supply.velocity.fiat.height,
            exit,
        )?;

        Ok(())
    }
}
