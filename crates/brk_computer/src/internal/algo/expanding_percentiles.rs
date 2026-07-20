use brk_types::StoredF32;

use super::fenwick::FenwickTree;

/// Fast expanding percentile tracker using a Fenwick tree (Binary Indexed Tree).
///
/// Values are discretized to 0.001 ratio resolution and tracked in
/// a fixed-size frequency array with Fenwick prefix sums. This gives:
/// - O(log N) insert (N = tree size, ~16 ops for 43k buckets)
/// - O(log N) percentile query via prefix-sum walk
/// - 0.1% value resolution (10 BPS granularity)
#[derive(Clone)]
pub(crate) struct ExpandingPercentiles {
    tree: FenwickTree<u32>,
    count: u32,
}

const BUCKET_WIDTH: f64 = 0.001;
const MAX_RATIO: f64 = 43.0;
const TREE_SIZE: usize = (MAX_RATIO / BUCKET_WIDTH) as usize + 1;

impl Default for ExpandingPercentiles {
    fn default() -> Self {
        Self {
            tree: FenwickTree::new(TREE_SIZE),
            count: 0,
        }
    }
}

impl ExpandingPercentiles {
    pub fn count(&self) -> u32 {
        self.count
    }

    pub fn reset(&mut self) {
        self.tree.reset();
        self.count = 0;
    }

    /// Convert f32 ratio to 0-indexed bucket.
    #[inline]
    fn to_bucket(value: f32) -> usize {
        (value as f64 / BUCKET_WIDTH)
            .round()
            .clamp(0.0, (TREE_SIZE - 1) as f64) as usize
    }

    /// Bulk-load values in O(n + N) instead of O(n log N).
    /// Builds raw frequency counts, then converts to Fenwick in-place.
    pub fn add_bulk(&mut self, values: &[StoredF32]) {
        for &v in values {
            let v = *v;
            if v.is_nan() {
                continue;
            }
            self.count += 1;
            self.tree.add_raw(Self::to_bucket(v), &1);
        }
        self.tree.build_in_place();
    }

    /// Add a value. O(log N).
    #[inline]
    pub fn add(&mut self, value: f32) {
        if value.is_nan() {
            return;
        }
        self.count += 1;
        self.tree.add(Self::to_bucket(value), &1);
    }

    /// Compute 8 percentiles in one call via kth. O(8 × log N) but with
    /// shared tree traversal across all 8 targets for better cache locality.
    /// Quantiles q must be sorted ascending in (0, 1). Output values are ratios.
    pub fn quantiles(&self, qs: &[f64; 8], out: &mut [f64; 8]) {
        if self.count == 0 {
            out.fill(0.0);
            return;
        }
        let mut targets = [0u32; 8];
        for (i, &q) in qs.iter().enumerate() {
            let k = ((q * self.count as f64).ceil() as u32).clamp(1, self.count);
            targets[i] = k - 1; // 0-indexed
        }
        let mut buckets = [0usize; 8];
        self.tree.kth(&targets, &|n: &u32| *n, &mut buckets);
        for (i, bucket) in buckets.iter().enumerate() {
            out[i] = *bucket as f64 * BUCKET_WIDTH;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn quantile(ep: &ExpandingPercentiles, q: f64) -> f64 {
        let mut out = [0.0; 8];
        ep.quantiles(&[q, q, q, q, q, q, q, q], &mut out);
        out[0]
    }

    #[test]
    fn basic_quantiles() {
        let mut ep = ExpandingPercentiles::default();
        for i in 1..=1000 {
            ep.add(i as f32 / 1000.0);
        }
        assert_eq!(ep.count(), 1000);

        let median = quantile(&ep, 0.5);
        assert!((median - 0.5).abs() < 0.01, "median was {median}");

        let p99 = quantile(&ep, 0.99);
        assert!((p99 - 0.99).abs() < 0.01, "p99 was {p99}");

        let p01 = quantile(&ep, 0.01);
        assert!((p01 - 0.01).abs() < 0.01, "p01 was {p01}");
    }

    #[test]
    fn empty() {
        let ep = ExpandingPercentiles::default();
        assert_eq!(ep.count(), 0);
        assert_eq!(quantile(&ep, 0.5), 0.0);
    }

    #[test]
    fn single_value() {
        let mut ep = ExpandingPercentiles::default();
        ep.add(0.42);
        let v = quantile(&ep, 0.5);
        assert!((v - 0.42).abs() <= BUCKET_WIDTH, "got {v}");
    }

    #[test]
    fn reset_works() {
        let mut ep = ExpandingPercentiles::default();
        for i in 0..100 {
            ep.add(i as f32 / 100.0);
        }
        assert_eq!(ep.count(), 100);
        ep.reset();
        assert_eq!(ep.count(), 0);
        assert_eq!(quantile(&ep, 0.5), 0.0);
    }
}
