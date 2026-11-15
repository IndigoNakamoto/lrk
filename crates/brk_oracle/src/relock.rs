//! Litecoin's relock: the round-dollar stencil tracks the price from block to block, but on
//! Litecoin's thinner payments it can slip onto an alias of it (1.5, 2, 2.5, 3 or 5 times off) and
//! stay there. Every [`CHECK`] blocks the last [`DAY`] blocks' payments are matched against a
//! template of what a day of Litecoin payments looks like around $100 (log-compressed, round-LTC
//! amounts masked), at the oracle's bin and at each alias of it; an alias that fits better by more
//! than [`MARGIN`] gets a vote. [`VOTES`] votes in a row that agree move the oracle onto them.

use std::{collections::VecDeque, sync::OnceLock};

use crate::{HistogramRaw, scale::NUM_BINS};

/// Bump on any change that alters a relock, so stored prices recompute.
pub const VERSION: u32 = 1;
/// Blocks of payments a vote reads: two days at 150 seconds a block.
pub const DAY: usize = 1152;
/// Heights a vote is taken at: multiples of this.
pub const CHECK: usize = 144;
/// Votes in a row a relock takes.
pub const VOTES: usize = 6;
/// How much better an alias must fit than the oracle's own bin.
const MARGIN: f64 = 0.03;
/// How far from their median agreeing votes lie, in bins.
const AGREE: f64 = 10.0;
/// Bins either side of a candidate it's read at its best within.
const NEAR: i64 = 6;
/// The aliases' distances from the oracle's bin: 1.5, 2, 2.5, 3 and 5 times.
const ALIASES: [i64; 5] = [35, 60, 80, 95, 140];
/// The template's reach either side of $100's bin.
const SPAN: i64 = 520;
/// Bins (within each decade) near round LTC amounts, left out of the match.
const MASK: [usize; 15] = [0, 1, 59, 60, 61, 79, 80, 81, 94, 95, 96, 139, 140, 141, 199];
/// Bins either side of a bin its baseline averages over.
const BASELINE: usize = 20;

/// Whether `height` takes a vote.
pub fn is_check(height: usize) -> bool {
    height.is_multiple_of(CHECK)
}

/// The template, `i` the offset `i - SPAN` from $100's bin.
fn template() -> &'static [f64] {
    static TEMPLATE: OnceLock<Vec<f64>> = OnceLock::new();
    TEMPLATE.get_or_init(|| {
        let template: Vec<f64> = include_str!("template_ltc.txt")
            .lines()
            .map(|line| line.trim().parse().expect("template_ltc.txt holds numbers"))
            .collect();
        assert_eq!(
            template.len(),
            2 * SPAN as usize + 1,
            "template_ltc.txt's length"
        );
        template
    })
}

fn masked(bin: usize) -> bool {
    MASK.contains(&(bin % 200))
}

/// The last [`DAY`] blocks' payments, and the last [`VOTES`] votes.
#[derive(Clone)]
pub struct Relock {
    day: Box<[u32; NUM_BINS]>,
    blocks: VecDeque<Vec<(u16, u32)>>,
    votes: VecDeque<Option<f64>>,
}

impl Default for Relock {
    fn default() -> Self {
        Self {
            day: Box::new([0; NUM_BINS]),
            blocks: VecDeque::with_capacity(DAY + 1),
            votes: VecDeque::with_capacity(VOTES + 1),
        }
    }
}

impl Relock {
    /// A block's payments in, the one [`DAY`] blocks before it out.
    pub fn push(&mut self, hist: &HistogramRaw) {
        let block: Vec<(u16, u32)> = hist
            .iter()
            .enumerate()
            .filter(|&(_, &n)| n > 0)
            .map(|(bin, &n)| (bin as u16, n))
            .collect();
        block
            .iter()
            .for_each(|&(bin, n)| self.day[bin as usize] += n);
        self.blocks.push_back(block);
        if self.blocks.len() > DAY
            && let Some(old) = self.blocks.pop_front()
        {
            old.iter().for_each(|&(bin, n)| self.day[bin as usize] -= n);
        }
    }

    /// The alias of the oracle at `bin` the day's payments fit better than `bin` itself, if any.
    pub fn vote(&self, bin: f64) -> Option<f64> {
        let h = self.prepared()?;
        let template = template();
        let best_near = |c: i64| {
            (-NEAR..=NEAR)
                .map(|k| (correlation(&h, template, c + k), c + k))
                .max_by(|a, b| a.0.total_cmp(&b.0))
                .expect("a nonempty neighborhood")
        };
        let center = bin.round() as i64;
        let here = best_near(center).0;
        let (score, at) = ALIASES
            .iter()
            .flat_map(|&a| [center + a, center - a])
            .map(best_near)
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .expect("aliases");
        (score - here > MARGIN).then_some(at as f64)
    }

    /// A vote in; the bin to relock onto when the last [`VOTES`] all voted and agree.
    pub fn record(&mut self, vote: Option<f64>) -> Option<f64> {
        self.votes.push_back(vote);
        if self.votes.len() > VOTES {
            self.votes.pop_front();
        }
        if self.votes.len() < VOTES {
            return None;
        }
        let mut votes = self.votes.iter().copied().collect::<Option<Vec<f64>>>()?;
        votes.sort_by(f64::total_cmp);
        let median = votes[VOTES / 2];
        votes
            .iter()
            .all(|v| (v - median).abs() <= AGREE)
            .then_some(median)
    }

    /// The day's payments as the template reads them: shares of the unmasked total,
    /// log-compressed, less their running mean; masked bins NaN.
    fn prepared(&self) -> Option<Vec<f64>> {
        let total: f64 = (0..NUM_BINS)
            .filter(|&b| !masked(b))
            .map(|b| self.day[b] as f64)
            .sum();
        if total <= 0.0 {
            return None;
        }
        let v: Vec<f64> = (0..NUM_BINS)
            .map(|b| {
                if masked(b) {
                    f64::NAN
                } else {
                    (self.day[b] as f64 / total * 1e4).ln_1p()
                }
            })
            .collect();
        Some(
            (0..NUM_BINS)
                .map(|b| {
                    if masked(b) {
                        return f64::NAN;
                    }
                    let (lo, hi) = (b.saturating_sub(BASELINE), (b + BASELINE + 1).min(NUM_BINS));
                    let (sum, n) = v[lo..hi]
                        .iter()
                        .filter(|x| !x.is_nan())
                        .fold((0.0, 0.0), |(s, n), x| (s + x, n + 1.0));
                    v[b] - sum / n
                })
                .collect(),
        )
    }
}

/// The correlation between the prepared day and the template with $100 at bin `c`.
fn correlation(h: &[f64], template: &[f64], c: i64) -> f64 {
    let (mut n, mut sx, mut sy, mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for (i, &t) in template.iter().enumerate() {
        let b = c + i as i64 - SPAN;
        if !(0..NUM_BINS as i64).contains(&b) || h[b as usize].is_nan() {
            continue;
        }
        let x = h[b as usize];
        n += 1.0;
        sx += x;
        sy += t;
        sxx += x * x;
        syy += t * t;
        sxy += x * t;
    }
    if n < 200.0 {
        return f64::MIN;
    }
    (sxy - sx * sy / n)
        / ((sxx - sx * sx / n) * (syy - sy * sy / n))
            .sqrt()
            .max(1e-30)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn template_loads() {
        assert_eq!(template().len(), 1041);
        assert!(template().iter().all(|t| t.is_finite()));
    }

    #[test]
    fn relocks_on_agreeing_votes_only() {
        let mut relock = Relock::default();
        for _ in 0..VOTES - 1 {
            assert_eq!(relock.record(Some(1000.0)), None);
        }
        assert_eq!(relock.record(Some(1004.0)), Some(1000.0));
        assert_eq!(relock.record(None), None);
        let mut relock = Relock::default();
        for v in [1000.0, 1000.0, 1000.0, 1100.0, 1000.0, 1000.0] {
            assert_eq!(relock.record(Some(v)), None);
        }
    }

    #[test]
    fn day_window_drops_old_blocks() {
        let mut relock = Relock::default();
        let mut hist = HistogramRaw::default();
        hist.increment(500);
        for _ in 0..DAY + 10 {
            relock.push(&hist);
        }
        assert_eq!(relock.day[500], DAY as u32);
    }
}
