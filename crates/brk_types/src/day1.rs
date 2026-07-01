use std::{
    fmt,
    ops::{Add, Rem},
};

use brk_error::Error;
use jiff::Span;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use vecdb::{CheckedSub, Formattable, Pco, PrintableIndex};

use crate::{FromCoarserIndex, Month1, Month3, Month6, Week1, Year1, Year10};

use super::{Date, Timestamp, date::date_index_zero, year::genesis_year};

#[derive(
    Debug,
    Default,
    Clone,
    Copy,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    Pco,
    JsonSchema,
)]
pub struct Day1(u16);

impl Day1 {
    pub const BYTES: usize = size_of::<Self>();

    pub fn to_timestamp(&self) -> Timestamp {
        Timestamp::from(Date::from(*self))
    }
}

impl From<Day1> for usize {
    #[inline]
    fn from(value: Day1) -> Self {
        value.0 as usize
    }
}

impl From<Day1> for u64 {
    #[inline]
    fn from(value: Day1) -> Self {
        value.0 as u64
    }
}

impl From<usize> for Day1 {
    #[inline]
    fn from(value: usize) -> Self {
        Self(value as u16)
    }
}

impl From<Day1> for i64 {
    #[inline]
    fn from(value: Day1) -> Self {
        value.0 as i64
    }
}

impl Add<usize> for Day1 {
    type Output = Self;
    fn add(self, rhs: usize) -> Self::Output {
        Self(self.0 + rhs as u16)
    }
}

impl TryFrom<Date> for Day1 {
    type Error = Error;
    fn try_from(value: Date) -> Result<Self, Self::Error> {
        // Anchor on the active chain's index epoch (Bitcoin: 2009-01-01,
        // Litecoin: 2011-10-03). This must match `From<Day1> for Date`, which
        // also uses `date_index_zero()`; a hardcoded epoch here would desync the
        // date↔index round-trip and shift every dated series (e.g. it made
        // Litecoin's ~2014-2016 prices render ~2.75 years too late).
        let zero = jiff::civil::Date::from(date_index_zero());
        let value_ = jiff::civil::Date::from(value);
        if value_ < zero {
            Err(Error::UnindexableDate)
        } else {
            Ok(Self(zero.until(value_)?.get_days() as u16))
        }
    }
}

impl CheckedSub for Day1 {
    fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }
}

impl Rem<usize> for Day1 {
    type Output = Self;
    fn rem(self, rhs: usize) -> Self::Output {
        Self(self.0 % rhs as u16)
    }
}

impl FromCoarserIndex<Week1> for Day1 {
    fn min_from(coarser: Week1) -> usize {
        usize::from(coarser) * 7
    }

    fn max_from_(coarser: Week1) -> usize {
        usize::from(coarser) * 7 + 6
    }
}

/// Day1 index for `date`, clamped to 0 for dates before the chain epoch.
/// Coarser buckets can start before the epoch (e.g. Litecoin's genesis year
/// 2011 begins in January, but the epoch is 2011-10-03), so those days simply
/// map to the first index rather than failing.
fn day1_index_or_zero(date: Date) -> usize {
    Day1::try_from(date).map(usize::from).unwrap_or(0)
}

impl FromCoarserIndex<Month1> for Day1 {
    fn min_from(coarser: Month1) -> usize {
        let d = Date::new(genesis_year(), 1, 1)
            .into_jiff()
            .checked_add(Span::new().months(u16::from(coarser)))
            .unwrap();
        day1_index_or_zero(Date::from(d))
    }

    fn max_from_(coarser: Month1) -> usize {
        let d = Date::new(genesis_year(), 1, 31)
            .into_jiff()
            .checked_add(Span::new().months(u16::from(coarser)))
            .unwrap();
        day1_index_or_zero(Date::from(d))
    }
}

impl FromCoarserIndex<Month3> for Day1 {
    fn min_from(coarser: Month3) -> usize {
        let d = Date::new(genesis_year(), 1, 1)
            .into_jiff()
            .checked_add(Span::new().months(3 * u8::from(coarser)))
            .unwrap();
        day1_index_or_zero(Date::from(d))
    }

    fn max_from_(coarser: Month3) -> usize {
        let d = Date::new(genesis_year(), 3, 31)
            .into_jiff()
            .checked_add(Span::new().months(3 * u8::from(coarser)))
            .unwrap();
        day1_index_or_zero(Date::from(d))
    }
}

impl FromCoarserIndex<Month6> for Day1 {
    fn min_from(coarser: Month6) -> usize {
        let d = Date::new(genesis_year(), 1, 1)
            .into_jiff()
            .checked_add(Span::new().months(6 * u8::from(coarser)))
            .unwrap();
        day1_index_or_zero(Date::from(d))
    }

    fn max_from_(coarser: Month6) -> usize {
        let d = Date::new(genesis_year(), 5, 31)
            .into_jiff()
            .checked_add(Span::new().months(1 + 6 * u8::from(coarser)))
            .unwrap();
        day1_index_or_zero(Date::from(d))
    }
}

impl FromCoarserIndex<Year1> for Day1 {
    fn min_from(coarser: Year1) -> usize {
        day1_index_or_zero(Date::new(genesis_year() + u8::from(coarser) as u16, 1, 1))
    }

    fn max_from_(coarser: Year1) -> usize {
        day1_index_or_zero(Date::new(genesis_year() + u8::from(coarser) as u16, 12, 31))
    }
}

impl FromCoarserIndex<Year10> for Day1 {
    fn min_from(coarser: Year10) -> usize {
        let coarser = u8::from(coarser);
        if coarser == 0 {
            // Decade 0 starts before the epoch; clamp to the first index.
            0
        } else {
            // `Year10` buckets by calendar decade (see `Year10::from(Date)`).
            day1_index_or_zero(Date::new(2000 + 10 * coarser as u16, 1, 1))
        }
    }

    fn max_from_(coarser: Year10) -> usize {
        let coarser = u8::from(coarser);
        day1_index_or_zero(Date::new(2009 + 10 * coarser as u16, 12, 31))
    }
}

impl PrintableIndex for Day1 {
    fn to_string() -> &'static str {
        "day1"
    }

    fn to_possible_strings() -> &'static [&'static str] {
        &["1d", "d", "day", "date", "daily", "day1", "dateindex"]
    }
}

impl fmt::Display for Day1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut buf = itoa::Buffer::new();
        let str = buf.format(self.0);
        f.write_str(str)
    }
}

impl Formattable for Day1 {
    #[inline(always)]
    fn write_to(&self, buf: &mut Vec<u8>) {
        let mut b = itoa::Buffer::new();
        buf.extend_from_slice(b.format(self.0).as_bytes());
    }
}
