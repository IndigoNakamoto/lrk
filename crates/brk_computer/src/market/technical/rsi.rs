use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::{Height, PartsPerMillion32, StoredF32};
use vecdb::{Exit, ReadableVec};

use super::RsiChain;
use crate::blocks;

pub(super) fn compute(
    chain: &mut RsiChain,
    indexer: &Indexer,
    blocks: &blocks::Vecs,
    returns_source: &impl ReadableVec<Height, StoredF32>,
    rma_days: usize,
    stoch_sma_days: usize,
    exit: &Exit,
) -> Result<()> {
    let starting_height = indexer.safe_lengths().height;
    let ws_rma = blocks.lookback.start_vec(rma_days);
    let ws_sma = blocks.lookback.start_vec(stoch_sma_days);

    chain.gains.height.compute_transform(
        starting_height,
        returns_source,
        |(h, r, ..)| (h, StoredF32::from((*r).max(0.0))),
        exit,
    )?;

    chain.losses.height.compute_transform(
        starting_height,
        returns_source,
        |(h, r, ..)| (h, StoredF32::from((-*r).max(0.0))),
        exit,
    )?;

    chain.average_gain.height.compute_rolling_rma(
        starting_height,
        ws_rma,
        &chain.gains.height,
        exit,
    )?;

    chain.average_loss.height.compute_rolling_rma(
        starting_height,
        ws_rma,
        &chain.losses.height,
        exit,
    )?;

    chain.rsi.raw.height.compute_transform2(
        starting_height,
        &chain.average_gain.height,
        &chain.average_loss.height,
        |(h, g, l, ..)| {
            let sum = *g + *l;
            let rsi = if sum == 0.0 { 0.5 } else { *g / sum };
            (h, PartsPerMillion32::from(rsi as f64))
        },
        exit,
    )?;

    chain.rsi_min.raw.height.compute_rolling_min_from_starts(
        starting_height,
        ws_rma,
        &chain.rsi.raw.height,
        exit,
    )?;

    chain.rsi_max.raw.height.compute_rolling_max_from_starts(
        starting_height,
        ws_rma,
        &chain.rsi.raw.height,
        exit,
    )?;

    chain.stoch_rsi.raw.height.compute_transform3(
        starting_height,
        &chain.rsi.raw.height,
        &chain.rsi_min.raw.height,
        &chain.rsi_max.raw.height,
        |(h, r, mn, mx, ..)| {
            let range = f64::from(*mx) - f64::from(*mn);
            let stoch = if range == 0.0 {
                PartsPerMillion32::ZERO
            } else {
                PartsPerMillion32::from((f64::from(*r) - f64::from(*mn)) / range)
            };
            (h, stoch)
        },
        exit,
    )?;

    chain.stoch_rsi_k.raw.height.compute_rolling_average(
        starting_height,
        ws_sma,
        &chain.stoch_rsi.raw.height,
        exit,
    )?;

    chain.stoch_rsi_d.raw.height.compute_rolling_average(
        starting_height,
        ws_sma,
        &chain.stoch_rsi_k.raw.height,
        exit,
    )?;

    Ok(())
}
