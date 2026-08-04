use brk_error::Result;
use brk_indexer::Indexer;
use vecdb::Exit;

use super::Vecs;
use crate::{blocks, price};

impl Vecs {
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        blocks: &blocks::Vecs,
        prices: &price::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        let starting_lengths = indexer.safe_lengths();
        let close = &prices.spot.cents.height;

        for (ema, period) in [
            (&mut self.ema._1w, 7),
            (&mut self.ema._8d, 8),
            (&mut self.ema._12d, 12),
            (&mut self.ema._13d, 13),
            (&mut self.ema._21d, 21),
            (&mut self.ema._26d, 26),
            (&mut self.ema._1m, 30),
            (&mut self.ema._34d, 34),
            (&mut self.ema._55d, 55),
            (&mut self.ema._89d, 89),
            (&mut self.ema._144d, 144),
            (&mut self.ema._200d, 200),
            (&mut self.ema._1y, 365),
            (&mut self.ema._2y, 2 * 365),
            (&mut self.ema._200w, 200 * 7),
            (&mut self.ema._4y, 4 * 365),
        ] {
            let window_starts = blocks.lookback.start_vec(period);
            ema.cents.height.compute_rolling_ema(
                starting_lengths.height,
                window_starts,
                close,
                exit,
            )?;
        }

        Ok(())
    }
}
