mod compute;
mod import;
mod vecs;

use brk_cohort::AGE_RANGE_BOUNDS;

pub(crate) use brk_cohort::AGE_RANGE_COUNT as AGE_COHORT_COUNT;
pub(crate) use vecs::HORIZON_DAYS;
pub use vecs::{CohortVecs, HorizonVecs, Horizons, Split, Vecs};

pub const DB_NAME: &str = "coinflow";

pub(crate) const HORIZON_COUNT: usize = 7;
pub(crate) const HOURS_PER_DAY: f64 = 24.0;
pub(crate) const MINIMUM_DURATION_DAYS: f64 = 1.0 / HOURS_PER_DAY;

#[derive(Clone, Copy)]
pub(crate) struct AgeBand {
    pub lower: f64,
    pub upper: f64,
}

pub(crate) fn age_bounds_days() -> [AgeBand; AGE_COHORT_COUNT] {
    let mut bounds = AGE_RANGE_BOUNDS.iter();
    std::array::from_fn(|index| {
        let bound = bounds.next().unwrap();
        AgeBand {
            lower: bound.start as f64 / HOURS_PER_DAY,
            upper: if index + 1 < AGE_COHORT_COUNT {
                bound.end as f64 / HOURS_PER_DAY
            } else {
                f64::INFINITY
            },
        }
    })
}

#[inline]
pub(crate) fn mobility(exposure: f64) -> f64 {
    if exposure.is_nan() || exposure <= 0.0 {
        0.0
    } else {
        (-(-exposure).exp_m1()).min(1.0 - 1e-12)
    }
}

pub(crate) fn horizon_mobility(
    hazards: &[f64; AGE_COHORT_COUNT],
    start_band: usize,
    horizon: f64,
    bounds: &[AgeBand; AGE_COHORT_COUNT],
) -> f64 {
    let start = bounds[start_band];
    let mut age = if start.upper.is_finite() {
        (start.lower + start.upper) / 2.0
    } else {
        start.lower
    };
    let mut remaining = horizon;
    let mut band = start_band;
    let mut exposure = 0.0;

    while remaining > 0.0 && band < AGE_COHORT_COUNT {
        let upper = bounds[band].upper;
        let duration = if upper.is_finite() {
            remaining.min((upper - age).max(MINIMUM_DURATION_DAYS))
        } else {
            remaining
        };
        exposure += hazards[band].max(0.0) * duration;
        remaining -= duration;
        band += 1;
        if band < AGE_COHORT_COUNT {
            age = bounds[band].lower;
        }
    }

    mobility(exposure)
}
