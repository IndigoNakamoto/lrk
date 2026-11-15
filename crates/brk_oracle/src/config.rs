use std::ops::Range;

/// First height the oracle computes on-chain, with the slow cold-start EMA
/// ([`Config::for_height`]). Below it, prices come from
/// [`pre_oracle_prices_from`](crate::pre_oracle_prices_from).
#[cfg(not(feature = "litecoin"))]
pub const START_HEIGHT_SLOW: usize = 340_000;

/// Height where the oracle switches slow -> fast EMA ([`default`](Config::default)).
/// The regimes are complementary: slow resists the round-USD half-price drift
/// that locks fast below here, while fast tracks the 2018-2019 crashes that lock
/// slow.
#[cfg(not(feature = "litecoin"))]
pub const START_HEIGHT_FAST: usize = 508_000;

/// Litecoin's first on-chain height (2022-01-26), straight into the fast EMA: earlier, its
/// payments' mix in dollars no longer fits the relock's template (see `relock`), and exchange
/// closes give the prices.
#[cfg(feature = "litecoin")]
pub const START_HEIGHT_SLOW: usize = 2_200_000;
#[cfg(feature = "litecoin")]
pub const START_HEIGHT_FAST: usize = START_HEIGHT_SLOW;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Config {
    /// EMA decay: 2/(N+1) where N is span in blocks. 2/7 = 6-block span.
    pub(crate) alpha: f64,
    /// Ring buffer depth: the EMA depends on these last blocks only (12 by default).
    pub window_size: usize,
    /// Search window bins below/above previous estimate. Asymmetric for log-scale.
    pub(crate) search_below: usize,
    pub(crate) search_above: usize,
    /// Weight of the adaptive shape-anchoring restoring force added to the
    /// stencil score. `0.0` disables it (mature regime, where the fast EMA
    /// tracks real moves the shape term would resist). The slow cold-start uses
    /// a positive weight to resist round-USD octave aliasing in the thin early
    /// output mix.
    pub(crate) shape_weight: f64,
}

impl Default for Config {
    /// Under `litecoin`, a narrower search (8 bins either way): Litecoin's thinner payments let
    /// a wider one slide onto a neighboring alias.
    fn default() -> Self {
        Self {
            alpha: 2.0 / 7.0,
            window_size: 12,
            search_below: if cfg!(feature = "litecoin") { 8 } else { 12 },
            search_above: if cfg!(feature = "litecoin") { 8 } else { 11 },
            shape_weight: 0.0,
        }
    }
}

impl Config {
    /// A config of one's own: for tuning the oracle offline on another chain.
    #[doc(hidden)]
    pub const fn custom(
        alpha: f64,
        window_size: usize,
        search_below: usize,
        search_above: usize,
        shape_weight: f64,
    ) -> Self {
        Self {
            alpha,
            window_size,
            search_below,
            search_above,
            shape_weight,
        }
    }

    /// Cold-start config below [`START_HEIGHT_FAST`]: a slow EMA
    /// (span ~19) that resists the round-USD half-price drift the fast default
    /// octave-locks onto in the thin pre-2018 output mix. Window grows to 40 to
    /// hold the decay, and a shape-anchoring restoring force (`shape_weight`)
    /// pulls the pick toward the octave whose arm-shape looks like real payments.
    fn slow() -> Self {
        Self {
            alpha: 0.10,
            window_size: 40,
            shape_weight: 8.0,
            ..Self::default()
        }
    }

    /// Config for `height`: the slow cold-start config below [`START_HEIGHT_FAST`], else
    /// [`default`](Self::default).
    pub fn for_height(height: usize) -> Self {
        if height < START_HEIGHT_FAST {
            Self::slow()
        } else {
            Self::default()
        }
    }

    /// Split a block range into sub-ranges with a single EMA configuration.
    pub fn segments_for_range(range: Range<usize>) -> impl Iterator<Item = Range<usize>> {
        let split = START_HEIGHT_FAST.max(range.start).min(range.end);
        [range.start..split, split..range.end]
            .into_iter()
            .filter(|range| !range.is_empty())
    }
}
