use brk_types::StoredF32;

use crate::internal::algo::FenwickTree;

/// First block included in the Rarity Meter distribution.
pub const START_HEIGHT: usize = 210_000;

/// Number of blocks after which an observation has half the weight of a new one.
pub const HALF_LIFE_BLOCKS: usize = 210_000;

/// Block-decayed percentile tracker backed by a Fenwick tree.
///
/// An observation at `height` receives weight
/// `2 ^ ((height - START_HEIGHT) / HALF_LIFE_BLOCKS)`. Multiplying every
/// observation by the same current-height decay factor does not change
/// quantiles, so this fixed scale is exactly equivalent to halving old weights
/// every 210,000 blocks without rescaling the tree on every block.
#[derive(Clone)]
pub(crate) struct BlockDecayPercentiles {
    tree: FenwickTree<f64>,
    len: usize,
    mass: f64,
}

const BUCKET_WIDTH: f64 = 0.001;
const MAX_RATIO: f64 = 43.0;
const TREE_SIZE: usize = (MAX_RATIO / BUCKET_WIDTH) as usize + 1;

impl Default for BlockDecayPercentiles {
    fn default() -> Self {
        Self {
            tree: FenwickTree::new(TREE_SIZE),
            len: 0,
            mass: 0.0,
        }
    }
}

impl BlockDecayPercentiles {
    pub fn len(&self) -> usize {
        self.len
    }

    pub fn reset(&mut self) {
        self.tree.reset();
        self.len = 0;
        self.mass = 0.0;
    }

    #[inline]
    fn to_bucket(value: f32) -> usize {
        (value as f64 / BUCKET_WIDTH)
            .round()
            .clamp(0.0, (TREE_SIZE - 1) as f64) as usize
    }

    #[inline]
    fn weight(height: usize) -> f64 {
        2.0_f64.powf(height.saturating_sub(START_HEIGHT) as f64 / HALF_LIFE_BLOCKS as f64)
    }

    /// Rebuild historical state in O(n + N).
    pub fn add_bulk(&mut self, start_height: usize, values: &[StoredF32]) {
        for (offset, &value) in values.iter().enumerate() {
            self.len += 1;
            let value = *value;
            if value.is_nan() {
                continue;
            }
            let weight = Self::weight(start_height + offset);
            self.mass += weight;
            self.tree.add_raw(Self::to_bucket(value), &weight);
        }
        self.tree.build_in_place();
    }

    /// Add the observation for one block. O(log N).
    #[inline]
    pub fn add(&mut self, height: usize, value: f32) {
        self.len += 1;
        if value.is_nan() {
            return;
        }
        let weight = Self::weight(height);
        self.mass += weight;
        self.tree.add(Self::to_bucket(value), &weight);
    }

    /// Compute sorted quantiles in one shared tree walk.
    pub fn quantiles<const N: usize>(&self, qs: &[f64; N], out: &mut [f64; N]) {
        if self.mass == 0.0 {
            out.fill(0.0);
            return;
        }

        let mut targets = [0.0; N];
        for (i, &q) in qs.iter().enumerate() {
            targets[i] = (q * self.mass).next_down().max(0.0);
        }

        let mut buckets = [0; N];
        self.tree
            .kth(&targets, &|weight: &f64| *weight, &mut buckets);
        for (i, bucket) in buckets.iter().enumerate() {
            out[i] = *bucket as f64 * BUCKET_WIDTH;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quantile(percentiles: &BlockDecayPercentiles, q: f64) -> f64 {
        let mut out = [0.0; 8];
        percentiles.quantiles(&[q, q, q, q, q, q, q, q], &mut out);
        out[0]
    }

    #[test]
    fn basic_quantiles() {
        let mut percentiles = BlockDecayPercentiles::default();
        for i in 1..=1000 {
            percentiles.add(START_HEIGHT + i, i as f32 / 1000.0);
        }
        assert_eq!(percentiles.len(), 1000);

        let median = quantile(&percentiles, 0.5);
        assert!((median - 0.5).abs() < 0.01, "median was {median}");

        let p99 = quantile(&percentiles, 0.99);
        assert!((p99 - 0.99).abs() < 0.01, "p99 was {p99}");

        let p01 = quantile(&percentiles, 0.01);
        assert!((p01 - 0.01).abs() < 0.01, "p01 was {p01}");
    }

    #[test]
    fn empty() {
        let percentiles = BlockDecayPercentiles::default();
        assert_eq!(quantile(&percentiles, 0.5), 0.0);
    }

    #[test]
    fn one_half_life_doubles_relative_weight() {
        let mut percentiles = BlockDecayPercentiles::default();
        percentiles.add(START_HEIGHT, 1.0);
        percentiles.add(START_HEIGHT + HALF_LIFE_BLOCKS, 2.0);

        assert!((percentiles.mass - 3.0).abs() < f64::EPSILON);
        assert_eq!(quantile(&percentiles, 0.5), 2.0);
    }

    #[test]
    fn bulk_recovery_matches_incremental_state() {
        let values: Vec<_> = (0..1000)
            .map(|i| StoredF32::from(i as f64 / 100.0))
            .collect();
        let mut incremental = BlockDecayPercentiles::default();
        for (offset, value) in values.iter().enumerate() {
            incremental.add(START_HEIGHT + offset, **value);
        }

        let mut recovered = BlockDecayPercentiles::default();
        recovered.add_bulk(START_HEIGHT, &values);

        let qs = [0.0001, 0.01, 0.1, 0.5, 0.9, 0.99, 0.999, 0.9999];
        let mut incremental_out = [0.0; 8];
        let mut recovered_out = [0.0; 8];
        incremental.quantiles(&qs, &mut incremental_out);
        recovered.quantiles(&qs, &mut recovered_out);

        assert_eq!(incremental.len, recovered.len);
        assert!((incremental.mass - recovered.mass).abs() < 1e-9);
        assert_eq!(incremental_out, recovered_out);
    }
}
