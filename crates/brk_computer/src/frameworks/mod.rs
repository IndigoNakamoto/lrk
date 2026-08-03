use brk_types::StoredF64;

pub(crate) mod coinflow;
pub(crate) mod cointime;
mod compute;
mod import;
mod vecs;

pub use vecs::Vecs;

pub const DB_NAME: &str = "frameworks";

#[derive(Clone, Copy, Default)]
pub(crate) struct WeightedRatio {
    numerator: f64,
    denominator: f64,
}

impl WeightedRatio {
    #[inline]
    pub(crate) fn add(&mut self, numerator: f64, denominator: f64, weight: f64) {
        if weight.is_finite() && weight > 0.0 {
            self.numerator += numerator * weight;
            self.denominator += denominator * weight;
        }
    }

    #[inline]
    pub(crate) fn value(&self) -> StoredF64 {
        if self.denominator > 0.0 {
            StoredF64::from((self.numerator / self.denominator).clamp(0.0, 1.0))
        } else {
            StoredF64::NAN
        }
    }
}
