use std::ops::Range;

use brk_traversable::Traversable;
use brk_types::Age;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde::Serialize;
use vecdb::{ColumnId, VecValue, Version};

use super::{CohortName, Filter, TimeFilter};

// Age boundary constants in hours
pub const HOURS_1H: usize = 1;
pub const HOURS_1D: usize = 24;
pub const HOURS_1W: usize = 24 * 7;
pub const HOURS_1M: usize = 24 * 30;
pub const HOURS_2M: usize = 24 * 2 * 30;
pub const HOURS_3M: usize = 24 * 3 * 30;
pub const HOURS_4M: usize = 24 * 4 * 30;
pub const HOURS_5M: usize = 24 * 5 * 30; // STH/LTH threshold
pub const HOURS_6M: usize = 24 * 6 * 30;
pub const HOURS_9M: usize = HOURS_6M + HOURS_3M;
pub const HOURS_1Y: usize = 24 * 365;
// Keep year-based ranges anchored to the existing 365-day year convention.
pub const HOURS_18M: usize = HOURS_1Y + HOURS_6M;
pub const HOURS_2Y: usize = 24 * 2 * 365;
pub const HOURS_3Y: usize = 24 * 3 * 365;
pub const HOURS_4Y: usize = 24 * 4 * 365;
pub const HOURS_5Y: usize = 24 * 5 * 365;
pub const HOURS_6Y: usize = 24 * 6 * 365;
pub const HOURS_7Y: usize = 24 * 7 * 365;
pub const HOURS_8Y: usize = 24 * 8 * 365;
pub const HOURS_10Y: usize = 24 * 10 * 365;
pub const HOURS_12Y: usize = 24 * 12 * 365;
pub const HOURS_15Y: usize = 24 * 15 * 365;

pub const AGE_RANGE_COUNT: usize = 23;
pub const STH_AGE_RANGE_COUNT: usize = 8;
pub const LTH_AGE_RANGE_COUNT: usize = AGE_RANGE_COUNT - STH_AGE_RANGE_COUNT;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(u8)]
pub enum AgeRangeId {
    Under1H,
    From1HTo1D,
    From1DTo1W,
    From1WTo1M,
    From1MTo2M,
    From2MTo3M,
    From3MTo4M,
    From4MTo5M,
    From5MTo6M,
    From6MTo9M,
    From9MTo1Y,
    From1YTo18M,
    From18MTo2Y,
    From2YTo3Y,
    From3YTo4Y,
    From4YTo5Y,
    From5YTo6Y,
    From6YTo7Y,
    From7YTo8Y,
    From8YTo10Y,
    From10YTo12Y,
    From12YTo15Y,
    Over15Y,
}

pub const AGE_RANGE_IDS: [AgeRangeId; AGE_RANGE_COUNT] = [
    AgeRangeId::Under1H,
    AgeRangeId::From1HTo1D,
    AgeRangeId::From1DTo1W,
    AgeRangeId::From1WTo1M,
    AgeRangeId::From1MTo2M,
    AgeRangeId::From2MTo3M,
    AgeRangeId::From3MTo4M,
    AgeRangeId::From4MTo5M,
    AgeRangeId::From5MTo6M,
    AgeRangeId::From6MTo9M,
    AgeRangeId::From9MTo1Y,
    AgeRangeId::From1YTo18M,
    AgeRangeId::From18MTo2Y,
    AgeRangeId::From2YTo3Y,
    AgeRangeId::From3YTo4Y,
    AgeRangeId::From4YTo5Y,
    AgeRangeId::From5YTo6Y,
    AgeRangeId::From6YTo7Y,
    AgeRangeId::From7YTo8Y,
    AgeRangeId::From8YTo10Y,
    AgeRangeId::From10YTo12Y,
    AgeRangeId::From12YTo15Y,
    AgeRangeId::Over15Y,
];

pub const STH_AGE_RANGE_IDS: [AgeRangeId; STH_AGE_RANGE_COUNT] = [
    AgeRangeId::Under1H,
    AgeRangeId::From1HTo1D,
    AgeRangeId::From1DTo1W,
    AgeRangeId::From1WTo1M,
    AgeRangeId::From1MTo2M,
    AgeRangeId::From2MTo3M,
    AgeRangeId::From3MTo4M,
    AgeRangeId::From4MTo5M,
];

pub const LTH_AGE_RANGE_IDS: [AgeRangeId; LTH_AGE_RANGE_COUNT] = [
    AgeRangeId::From5MTo6M,
    AgeRangeId::From6MTo9M,
    AgeRangeId::From9MTo1Y,
    AgeRangeId::From1YTo18M,
    AgeRangeId::From18MTo2Y,
    AgeRangeId::From2YTo3Y,
    AgeRangeId::From3YTo4Y,
    AgeRangeId::From4YTo5Y,
    AgeRangeId::From5YTo6Y,
    AgeRangeId::From6YTo7Y,
    AgeRangeId::From7YTo8Y,
    AgeRangeId::From8YTo10Y,
    AgeRangeId::From10YTo12Y,
    AgeRangeId::From12YTo15Y,
    AgeRangeId::Over15Y,
];

impl ColumnId for AgeRangeId {
    type Row<T>
        = [T; AGE_RANGE_COUNT]
    where
        T: VecValue;

    const VERSION: Version = Version::ONE;
    const ALL: &'static [Self] = &AGE_RANGE_IDS;

    #[inline]
    fn index(self) -> usize {
        self as usize
    }

    #[inline]
    fn get<T: VecValue>(self, row: &Self::Row<T>) -> &T {
        &row[self as usize]
    }

    #[inline]
    fn get_mut<T: VecValue>(self, row: &mut Self::Row<T>) -> &mut T {
        &mut row[self as usize]
    }

    #[inline]
    fn from_fn<T, F>(mut f: F) -> Self::Row<T>
    where
        T: VecValue,
        F: FnMut(Self) -> T,
    {
        std::array::from_fn(|index| f(AGE_RANGE_IDS[index]))
    }

    #[inline]
    fn map<T, U, F>(row: Self::Row<T>, f: F) -> Self::Row<U>
    where
        T: VecValue,
        U: VecValue,
        F: FnMut(T) -> U,
    {
        row.map(f)
    }
}

/// Age boundaries in hours. Defines the cohort ranges:
/// [0, 1h), [1h, 1d), [1d, 1w), [1w, 1m), ..., [15y, ∞)
pub const AGE_BOUNDARIES: [usize; AGE_RANGE_COUNT - 1] = [
    HOURS_1H, HOURS_1D, HOURS_1W, HOURS_1M, HOURS_2M, HOURS_3M, HOURS_4M, HOURS_5M, HOURS_6M,
    HOURS_9M, HOURS_1Y, HOURS_18M, HOURS_2Y, HOURS_3Y, HOURS_4Y, HOURS_5Y, HOURS_6Y, HOURS_7Y,
    HOURS_8Y, HOURS_10Y, HOURS_12Y, HOURS_15Y,
];

/// Age range bounds (end = usize::MAX means unbounded)
pub const AGE_RANGE_BOUNDS: AgeRange<Range<usize>> = AgeRange {
    under_1h: 0..HOURS_1H,
    _1h_to_1d: HOURS_1H..HOURS_1D,
    _1d_to_1w: HOURS_1D..HOURS_1W,
    _1w_to_1m: HOURS_1W..HOURS_1M,
    _1m_to_2m: HOURS_1M..HOURS_2M,
    _2m_to_3m: HOURS_2M..HOURS_3M,
    _3m_to_4m: HOURS_3M..HOURS_4M,
    _4m_to_5m: HOURS_4M..HOURS_5M,
    _5m_to_6m: HOURS_5M..HOURS_6M,
    _6m_to_9m: HOURS_6M..HOURS_9M,
    _9m_to_1y: HOURS_9M..HOURS_1Y,
    _1y_to_18m: HOURS_1Y..HOURS_18M,
    _18m_to_2y: HOURS_18M..HOURS_2Y,
    _2y_to_3y: HOURS_2Y..HOURS_3Y,
    _3y_to_4y: HOURS_3Y..HOURS_4Y,
    _4y_to_5y: HOURS_4Y..HOURS_5Y,
    _5y_to_6y: HOURS_5Y..HOURS_6Y,
    _6y_to_7y: HOURS_6Y..HOURS_7Y,
    _7y_to_8y: HOURS_7Y..HOURS_8Y,
    _8y_to_10y: HOURS_8Y..HOURS_10Y,
    _10y_to_12y: HOURS_10Y..HOURS_12Y,
    _12y_to_15y: HOURS_12Y..HOURS_15Y,
    over_15y: HOURS_15Y..usize::MAX,
};

/// Age range filters
pub const AGE_RANGE_FILTERS: AgeRange<Filter> = AgeRange {
    under_1h: Filter::Time(TimeFilter::Range(AGE_RANGE_BOUNDS.under_1h)),
    _1h_to_1d: Filter::Time(TimeFilter::Range(AGE_RANGE_BOUNDS._1h_to_1d)),
    _1d_to_1w: Filter::Time(TimeFilter::Range(AGE_RANGE_BOUNDS._1d_to_1w)),
    _1w_to_1m: Filter::Time(TimeFilter::Range(AGE_RANGE_BOUNDS._1w_to_1m)),
    _1m_to_2m: Filter::Time(TimeFilter::Range(AGE_RANGE_BOUNDS._1m_to_2m)),
    _2m_to_3m: Filter::Time(TimeFilter::Range(AGE_RANGE_BOUNDS._2m_to_3m)),
    _3m_to_4m: Filter::Time(TimeFilter::Range(AGE_RANGE_BOUNDS._3m_to_4m)),
    _4m_to_5m: Filter::Time(TimeFilter::Range(AGE_RANGE_BOUNDS._4m_to_5m)),
    _5m_to_6m: Filter::Time(TimeFilter::Range(AGE_RANGE_BOUNDS._5m_to_6m)),
    _6m_to_9m: Filter::Time(TimeFilter::Range(AGE_RANGE_BOUNDS._6m_to_9m)),
    _9m_to_1y: Filter::Time(TimeFilter::Range(AGE_RANGE_BOUNDS._9m_to_1y)),
    _1y_to_18m: Filter::Time(TimeFilter::Range(AGE_RANGE_BOUNDS._1y_to_18m)),
    _18m_to_2y: Filter::Time(TimeFilter::Range(AGE_RANGE_BOUNDS._18m_to_2y)),
    _2y_to_3y: Filter::Time(TimeFilter::Range(AGE_RANGE_BOUNDS._2y_to_3y)),
    _3y_to_4y: Filter::Time(TimeFilter::Range(AGE_RANGE_BOUNDS._3y_to_4y)),
    _4y_to_5y: Filter::Time(TimeFilter::Range(AGE_RANGE_BOUNDS._4y_to_5y)),
    _5y_to_6y: Filter::Time(TimeFilter::Range(AGE_RANGE_BOUNDS._5y_to_6y)),
    _6y_to_7y: Filter::Time(TimeFilter::Range(AGE_RANGE_BOUNDS._6y_to_7y)),
    _7y_to_8y: Filter::Time(TimeFilter::Range(AGE_RANGE_BOUNDS._7y_to_8y)),
    _8y_to_10y: Filter::Time(TimeFilter::Range(AGE_RANGE_BOUNDS._8y_to_10y)),
    _10y_to_12y: Filter::Time(TimeFilter::Range(AGE_RANGE_BOUNDS._10y_to_12y)),
    _12y_to_15y: Filter::Time(TimeFilter::Range(AGE_RANGE_BOUNDS._12y_to_15y)),
    over_15y: Filter::Time(TimeFilter::Range(AGE_RANGE_BOUNDS.over_15y)),
};

/// Age range names
pub const AGE_RANGE_NAMES: AgeRange<CohortName> = AgeRange {
    under_1h: CohortName::new("under_1h_old", "<1h", "Under 1 Hour Old"),
    _1h_to_1d: CohortName::new("1h_to_1d_old", "1h-1d", "1 Hour to 1 Day Old"),
    _1d_to_1w: CohortName::new("1d_to_1w_old", "1d-1w", "1 Day to 1 Week Old"),
    _1w_to_1m: CohortName::new("1w_to_1m_old", "1w-1m", "1 Week to 1 Month Old"),
    _1m_to_2m: CohortName::new("1m_to_2m_old", "1m-2m", "1 to 2 Months Old"),
    _2m_to_3m: CohortName::new("2m_to_3m_old", "2m-3m", "2 to 3 Months Old"),
    _3m_to_4m: CohortName::new("3m_to_4m_old", "3m-4m", "3 to 4 Months Old"),
    _4m_to_5m: CohortName::new("4m_to_5m_old", "4m-5m", "4 to 5 Months Old"),
    _5m_to_6m: CohortName::new("5m_to_6m_old", "5m-6m", "5 to 6 Months Old"),
    _6m_to_9m: CohortName::new("6m_to_9m_old", "6m-9m", "6 to 9 Months Old"),
    _9m_to_1y: CohortName::new("9m_to_1y_old", "9m-1y", "9 Months to 1 Year Old"),
    _1y_to_18m: CohortName::new("1y_to_18m_old", "1y-18m", "1 Year to 18 Months Old"),
    _18m_to_2y: CohortName::new("18m_to_2y_old", "18m-2y", "18 Months to 2 Years Old"),
    _2y_to_3y: CohortName::new("2y_to_3y_old", "2y-3y", "2 to 3 Years Old"),
    _3y_to_4y: CohortName::new("3y_to_4y_old", "3y-4y", "3 to 4 Years Old"),
    _4y_to_5y: CohortName::new("4y_to_5y_old", "4y-5y", "4 to 5 Years Old"),
    _5y_to_6y: CohortName::new("5y_to_6y_old", "5y-6y", "5 to 6 Years Old"),
    _6y_to_7y: CohortName::new("6y_to_7y_old", "6y-7y", "6 to 7 Years Old"),
    _7y_to_8y: CohortName::new("7y_to_8y_old", "7y-8y", "7 to 8 Years Old"),
    _8y_to_10y: CohortName::new("8y_to_10y_old", "8y-10y", "8 to 10 Years Old"),
    _10y_to_12y: CohortName::new("10y_to_12y_old", "10y-12y", "10 to 12 Years Old"),
    _12y_to_15y: CohortName::new("12y_to_15y_old", "12y-15y", "12 to 15 Years Old"),
    over_15y: CohortName::new("over_15y_old", "15y+", "15+ Years Old"),
};

impl AgeRange<CohortName> {
    pub const fn names() -> &'static Self {
        &AGE_RANGE_NAMES
    }
}

#[derive(Default, Clone, Traversable, Serialize)]
pub struct AgeRange<T> {
    pub under_1h: T,
    pub _1h_to_1d: T,
    pub _1d_to_1w: T,
    pub _1w_to_1m: T,
    pub _1m_to_2m: T,
    pub _2m_to_3m: T,
    pub _3m_to_4m: T,
    pub _4m_to_5m: T,
    pub _5m_to_6m: T,
    pub _6m_to_9m: T,
    pub _9m_to_1y: T,
    pub _1y_to_18m: T,
    pub _18m_to_2y: T,
    pub _2y_to_3y: T,
    pub _3y_to_4y: T,
    pub _4y_to_5y: T,
    pub _5y_to_6y: T,
    pub _6y_to_7y: T,
    pub _7y_to_8y: T,
    pub _8y_to_10y: T,
    pub _10y_to_12y: T,
    pub _12y_to_15y: T,
    pub over_15y: T,
}

impl<T> AgeRange<T> {
    /// Get mutable reference by Age. O(1).
    #[inline]
    pub fn get_mut(&mut self, age: Age) -> &mut T {
        match age.hours() {
            0..HOURS_1H => &mut self.under_1h,
            HOURS_1H..HOURS_1D => &mut self._1h_to_1d,
            HOURS_1D..HOURS_1W => &mut self._1d_to_1w,
            HOURS_1W..HOURS_1M => &mut self._1w_to_1m,
            HOURS_1M..HOURS_2M => &mut self._1m_to_2m,
            HOURS_2M..HOURS_3M => &mut self._2m_to_3m,
            HOURS_3M..HOURS_4M => &mut self._3m_to_4m,
            HOURS_4M..HOURS_5M => &mut self._4m_to_5m,
            HOURS_5M..HOURS_6M => &mut self._5m_to_6m,
            HOURS_6M..HOURS_9M => &mut self._6m_to_9m,
            HOURS_9M..HOURS_1Y => &mut self._9m_to_1y,
            HOURS_1Y..HOURS_18M => &mut self._1y_to_18m,
            HOURS_18M..HOURS_2Y => &mut self._18m_to_2y,
            HOURS_2Y..HOURS_3Y => &mut self._2y_to_3y,
            HOURS_3Y..HOURS_4Y => &mut self._3y_to_4y,
            HOURS_4Y..HOURS_5Y => &mut self._4y_to_5y,
            HOURS_5Y..HOURS_6Y => &mut self._5y_to_6y,
            HOURS_6Y..HOURS_7Y => &mut self._6y_to_7y,
            HOURS_7Y..HOURS_8Y => &mut self._7y_to_8y,
            HOURS_8Y..HOURS_10Y => &mut self._8y_to_10y,
            HOURS_10Y..HOURS_12Y => &mut self._10y_to_12y,
            HOURS_12Y..HOURS_15Y => &mut self._12y_to_15y,
            _ => &mut self.over_15y,
        }
    }

    /// Get reference by Age. O(1).
    #[inline]
    pub fn get(&self, age: Age) -> &T {
        match age.hours() {
            0..HOURS_1H => &self.under_1h,
            HOURS_1H..HOURS_1D => &self._1h_to_1d,
            HOURS_1D..HOURS_1W => &self._1d_to_1w,
            HOURS_1W..HOURS_1M => &self._1w_to_1m,
            HOURS_1M..HOURS_2M => &self._1m_to_2m,
            HOURS_2M..HOURS_3M => &self._2m_to_3m,
            HOURS_3M..HOURS_4M => &self._3m_to_4m,
            HOURS_4M..HOURS_5M => &self._4m_to_5m,
            HOURS_5M..HOURS_6M => &self._5m_to_6m,
            HOURS_6M..HOURS_9M => &self._6m_to_9m,
            HOURS_9M..HOURS_1Y => &self._9m_to_1y,
            HOURS_1Y..HOURS_18M => &self._1y_to_18m,
            HOURS_18M..HOURS_2Y => &self._18m_to_2y,
            HOURS_2Y..HOURS_3Y => &self._2y_to_3y,
            HOURS_3Y..HOURS_4Y => &self._3y_to_4y,
            HOURS_4Y..HOURS_5Y => &self._4y_to_5y,
            HOURS_5Y..HOURS_6Y => &self._5y_to_6y,
            HOURS_6Y..HOURS_7Y => &self._6y_to_7y,
            HOURS_7Y..HOURS_8Y => &self._7y_to_8y,
            HOURS_8Y..HOURS_10Y => &self._8y_to_10y,
            HOURS_10Y..HOURS_12Y => &self._10y_to_12y,
            HOURS_12Y..HOURS_15Y => &self._12y_to_15y,
            _ => &self.over_15y,
        }
    }

    pub fn from_array(arr: [T; AGE_RANGE_COUNT]) -> Self {
        let [
            a0,
            a1,
            a2,
            a3,
            a4,
            a5,
            a6,
            a7,
            a8,
            a9,
            a10,
            a11,
            a12,
            a13,
            a14,
            a15,
            a16,
            a17,
            a18,
            a19,
            a20,
            a21,
            a22,
        ] = arr;
        Self {
            under_1h: a0,
            _1h_to_1d: a1,
            _1d_to_1w: a2,
            _1w_to_1m: a3,
            _1m_to_2m: a4,
            _2m_to_3m: a5,
            _3m_to_4m: a6,
            _4m_to_5m: a7,
            _5m_to_6m: a8,
            _6m_to_9m: a9,
            _9m_to_1y: a10,
            _1y_to_18m: a11,
            _18m_to_2y: a12,
            _2y_to_3y: a13,
            _3y_to_4y: a14,
            _4y_to_5y: a15,
            _5y_to_6y: a16,
            _6y_to_7y: a17,
            _7y_to_8y: a18,
            _8y_to_10y: a19,
            _10y_to_12y: a20,
            _12y_to_15y: a21,
            over_15y: a22,
        }
    }

    pub fn new<F>(mut create: F) -> Self
    where
        F: FnMut(Filter, &'static str) -> T,
    {
        let f = AGE_RANGE_FILTERS;
        let n = AGE_RANGE_NAMES;
        Self {
            under_1h: create(f.under_1h.clone(), n.under_1h.id),
            _1h_to_1d: create(f._1h_to_1d.clone(), n._1h_to_1d.id),
            _1d_to_1w: create(f._1d_to_1w.clone(), n._1d_to_1w.id),
            _1w_to_1m: create(f._1w_to_1m.clone(), n._1w_to_1m.id),
            _1m_to_2m: create(f._1m_to_2m.clone(), n._1m_to_2m.id),
            _2m_to_3m: create(f._2m_to_3m.clone(), n._2m_to_3m.id),
            _3m_to_4m: create(f._3m_to_4m.clone(), n._3m_to_4m.id),
            _4m_to_5m: create(f._4m_to_5m.clone(), n._4m_to_5m.id),
            _5m_to_6m: create(f._5m_to_6m.clone(), n._5m_to_6m.id),
            _6m_to_9m: create(f._6m_to_9m.clone(), n._6m_to_9m.id),
            _9m_to_1y: create(f._9m_to_1y.clone(), n._9m_to_1y.id),
            _1y_to_18m: create(f._1y_to_18m.clone(), n._1y_to_18m.id),
            _18m_to_2y: create(f._18m_to_2y.clone(), n._18m_to_2y.id),
            _2y_to_3y: create(f._2y_to_3y.clone(), n._2y_to_3y.id),
            _3y_to_4y: create(f._3y_to_4y.clone(), n._3y_to_4y.id),
            _4y_to_5y: create(f._4y_to_5y.clone(), n._4y_to_5y.id),
            _5y_to_6y: create(f._5y_to_6y.clone(), n._5y_to_6y.id),
            _6y_to_7y: create(f._6y_to_7y.clone(), n._6y_to_7y.id),
            _7y_to_8y: create(f._7y_to_8y.clone(), n._7y_to_8y.id),
            _8y_to_10y: create(f._8y_to_10y.clone(), n._8y_to_10y.id),
            _10y_to_12y: create(f._10y_to_12y.clone(), n._10y_to_12y.id),
            _12y_to_15y: create(f._12y_to_15y.clone(), n._12y_to_15y.id),
            over_15y: create(f.over_15y.clone(), n.over_15y.id),
        }
    }

    pub fn try_new<F, E>(mut create: F) -> Result<Self, E>
    where
        F: FnMut(Filter, &'static str) -> Result<T, E>,
    {
        let f = AGE_RANGE_FILTERS;
        let n = AGE_RANGE_NAMES;
        Ok(Self {
            under_1h: create(f.under_1h.clone(), n.under_1h.id)?,
            _1h_to_1d: create(f._1h_to_1d.clone(), n._1h_to_1d.id)?,
            _1d_to_1w: create(f._1d_to_1w.clone(), n._1d_to_1w.id)?,
            _1w_to_1m: create(f._1w_to_1m.clone(), n._1w_to_1m.id)?,
            _1m_to_2m: create(f._1m_to_2m.clone(), n._1m_to_2m.id)?,
            _2m_to_3m: create(f._2m_to_3m.clone(), n._2m_to_3m.id)?,
            _3m_to_4m: create(f._3m_to_4m.clone(), n._3m_to_4m.id)?,
            _4m_to_5m: create(f._4m_to_5m.clone(), n._4m_to_5m.id)?,
            _5m_to_6m: create(f._5m_to_6m.clone(), n._5m_to_6m.id)?,
            _6m_to_9m: create(f._6m_to_9m.clone(), n._6m_to_9m.id)?,
            _9m_to_1y: create(f._9m_to_1y.clone(), n._9m_to_1y.id)?,
            _1y_to_18m: create(f._1y_to_18m.clone(), n._1y_to_18m.id)?,
            _18m_to_2y: create(f._18m_to_2y.clone(), n._18m_to_2y.id)?,
            _2y_to_3y: create(f._2y_to_3y.clone(), n._2y_to_3y.id)?,
            _3y_to_4y: create(f._3y_to_4y.clone(), n._3y_to_4y.id)?,
            _4y_to_5y: create(f._4y_to_5y.clone(), n._4y_to_5y.id)?,
            _5y_to_6y: create(f._5y_to_6y.clone(), n._5y_to_6y.id)?,
            _6y_to_7y: create(f._6y_to_7y.clone(), n._6y_to_7y.id)?,
            _7y_to_8y: create(f._7y_to_8y.clone(), n._7y_to_8y.id)?,
            _8y_to_10y: create(f._8y_to_10y.clone(), n._8y_to_10y.id)?,
            _10y_to_12y: create(f._10y_to_12y.clone(), n._10y_to_12y.id)?,
            _12y_to_15y: create(f._12y_to_15y.clone(), n._12y_to_15y.id)?,
            over_15y: create(f.over_15y.clone(), n.over_15y.id)?,
        })
    }

    pub fn iter(&self) -> impl Iterator<Item = &T> {
        [
            &self.under_1h,
            &self._1h_to_1d,
            &self._1d_to_1w,
            &self._1w_to_1m,
            &self._1m_to_2m,
            &self._2m_to_3m,
            &self._3m_to_4m,
            &self._4m_to_5m,
            &self._5m_to_6m,
            &self._6m_to_9m,
            &self._9m_to_1y,
            &self._1y_to_18m,
            &self._18m_to_2y,
            &self._2y_to_3y,
            &self._3y_to_4y,
            &self._4y_to_5y,
            &self._5y_to_6y,
            &self._6y_to_7y,
            &self._7y_to_8y,
            &self._8y_to_10y,
            &self._10y_to_12y,
            &self._12y_to_15y,
            &self.over_15y,
        ]
        .into_iter()
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        [
            &mut self.under_1h,
            &mut self._1h_to_1d,
            &mut self._1d_to_1w,
            &mut self._1w_to_1m,
            &mut self._1m_to_2m,
            &mut self._2m_to_3m,
            &mut self._3m_to_4m,
            &mut self._4m_to_5m,
            &mut self._5m_to_6m,
            &mut self._6m_to_9m,
            &mut self._9m_to_1y,
            &mut self._1y_to_18m,
            &mut self._18m_to_2y,
            &mut self._2y_to_3y,
            &mut self._3y_to_4y,
            &mut self._4y_to_5y,
            &mut self._5y_to_6y,
            &mut self._6y_to_7y,
            &mut self._7y_to_8y,
            &mut self._8y_to_10y,
            &mut self._10y_to_12y,
            &mut self._12y_to_15y,
            &mut self.over_15y,
        ]
        .into_iter()
    }

    pub fn par_iter_mut(&mut self) -> impl ParallelIterator<Item = &mut T>
    where
        T: Send + Sync,
    {
        [
            &mut self.under_1h,
            &mut self._1h_to_1d,
            &mut self._1d_to_1w,
            &mut self._1w_to_1m,
            &mut self._1m_to_2m,
            &mut self._2m_to_3m,
            &mut self._3m_to_4m,
            &mut self._4m_to_5m,
            &mut self._5m_to_6m,
            &mut self._6m_to_9m,
            &mut self._9m_to_1y,
            &mut self._1y_to_18m,
            &mut self._18m_to_2y,
            &mut self._2y_to_3y,
            &mut self._3y_to_4y,
            &mut self._4y_to_5y,
            &mut self._5y_to_6y,
            &mut self._6y_to_7y,
            &mut self._7y_to_8y,
            &mut self._8y_to_10y,
            &mut self._10y_to_12y,
            &mut self._12y_to_15y,
            &mut self.over_15y,
        ]
        .into_par_iter()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_are_contiguous_and_match_the_declared_count() {
        let ranges: Vec<_> = AGE_RANGE_BOUNDS.iter().collect();

        assert_eq!(ranges.len(), AGE_RANGE_COUNT);
        assert_eq!(ranges.first().unwrap().start, 0);
        assert_eq!(ranges.last().unwrap().end, usize::MAX);

        for adjacent in ranges.windows(2) {
            assert_eq!(adjacent[0].end, adjacent[1].start);
        }
    }

    #[test]
    fn column_ids_match_storage_order_and_term_split() {
        assert_eq!(AgeRangeId::ALL, &AGE_RANGE_IDS);
        assert_eq!(
            STH_AGE_RANGE_IDS.len() + LTH_AGE_RANGE_IDS.len(),
            AGE_RANGE_COUNT
        );
        assert_eq!(
            STH_AGE_RANGE_IDS.as_slice(),
            &AGE_RANGE_IDS[..STH_AGE_RANGE_COUNT]
        );
        assert_eq!(
            LTH_AGE_RANGE_IDS.as_slice(),
            &AGE_RANGE_IDS[STH_AGE_RANGE_COUNT..]
        );
        assert_eq!(STH_AGE_RANGE_IDS.last(), Some(&AgeRangeId::From4MTo5M));
        assert_eq!(LTH_AGE_RANGE_IDS.first(), Some(&AgeRangeId::From5MTo6M));

        let row = AgeRangeId::from_fn(|column| column.index());
        for (index, &column) in AGE_RANGE_IDS.iter().enumerate() {
            assert_eq!(column.index(), index);
            assert_eq!(*column.get(&row), index);
        }
    }

    #[test]
    fn split_range_names_and_boundaries_match() {
        assert_eq!(HOURS_9M, HOURS_6M + HOURS_3M);
        assert_eq!(HOURS_18M, HOURS_1Y + HOURS_6M);
        assert_eq!(AGE_RANGE_NAMES._6m_to_9m.id, "6m_to_9m_old");
        assert_eq!(AGE_RANGE_NAMES._9m_to_1y.id, "9m_to_1y_old");
        assert_eq!(AGE_RANGE_NAMES._1y_to_18m.id, "1y_to_18m_old");
        assert_eq!(AGE_RANGE_NAMES._18m_to_2y.id, "18m_to_2y_old");
    }

    #[test]
    fn eighteen_month_classifier_stays_anchored_to_one_year_plus_six_months() {
        let age_at_540_days = Age::new(
            brk_types::Timestamp::new((HOURS_6M * 3 * 60 * 60) as u32),
            brk_types::Timestamp::ZERO,
        );
        let age_at_18m = Age::new(
            brk_types::Timestamp::new((HOURS_18M * 60 * 60) as u32),
            brk_types::Timestamp::ZERO,
        );

        assert_eq!(AGE_RANGE_NAMES.get(age_at_540_days).id, "1y_to_18m_old");
        assert_eq!(AGE_RANGE_NAMES.get(age_at_18m).id, "18m_to_2y_old");
    }
}
