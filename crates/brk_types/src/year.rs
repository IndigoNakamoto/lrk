use std::{
    fmt::Debug,
    ops::{Add, AddAssign, Div},
    sync::atomic::{AtomicU16, Ordering},
};

use serde::{Deserialize, Serialize};
use vecdb::{CheckedSub, Formattable, Pco, PrintableIndex};

use super::{Date, Timestamp};

/// Bitcoin genesis year; Litecoin uses 2011. Set at startup via `brk_types::init_chain_epoch`.
pub const GENESIS_YEAR: u16 = 2009;

static GENESIS_YEAR_GLOBAL: AtomicU16 = AtomicU16::new(GENESIS_YEAR);

/// Returns the active chain's genesis year.
#[inline]
pub fn genesis_year() -> u16 {
    GENESIS_YEAR_GLOBAL.load(Ordering::Relaxed)
}

/// Set the chain's genesis year once at program startup.
pub fn set_genesis_year(year: u16) {
    GENESIS_YEAR_GLOBAL.store(year, Ordering::Relaxed);
}

/// Bitcoin year (2009, 2010, ..., 2025+)
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default, Hash, Serialize, Deserialize, Pco,
)]
pub struct Year(u16);

impl Year {
    pub const GENESIS: Self = Self(2009);

    pub const fn new(value: u16) -> Self {
        Self(value)
    }

    /// Returns the year as an index (0 = genesis year, 1 = genesis+1, etc.)
    pub fn to_index(self) -> usize {
        self.0.saturating_sub(genesis_year()) as usize
    }
}

impl From<u16> for Year {
    #[inline]
    fn from(value: u16) -> Self {
        Self(value)
    }
}

impl From<usize> for Year {
    #[inline]
    fn from(value: usize) -> Self {
        Self(value as u16)
    }
}

impl From<Year> for usize {
    #[inline]
    fn from(value: Year) -> Self {
        value.0 as usize
    }
}

impl From<Year> for u16 {
    #[inline]
    fn from(value: Year) -> Self {
        value.0
    }
}

impl Add for Year {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self::from(self.0 + rhs.0)
    }
}

impl AddAssign for Year {
    fn add_assign(&mut self, rhs: Self) {
        *self = *self + rhs
    }
}

impl Add<usize> for Year {
    type Output = Self;

    fn add(self, rhs: usize) -> Self::Output {
        Self::from(self.0 + rhs as u16)
    }
}

impl From<Timestamp> for Year {
    #[inline]
    fn from(value: Timestamp) -> Self {
        Self(Date::from(value).year())
    }
}

impl From<Date> for Year {
    #[inline]
    fn from(value: Date) -> Self {
        Self(value.year())
    }
}

impl CheckedSub for Year {
    fn checked_sub(self, rhs: Self) -> Option<Self> {
        self.0.checked_sub(rhs.0).map(Self)
    }
}

impl Div<usize> for Year {
    type Output = Self;
    fn div(self, rhs: usize) -> Self::Output {
        Self::from(self.0 as usize / rhs)
    }
}

impl PrintableIndex for Year {
    fn to_string() -> &'static str {
        "year"
    }

    fn to_possible_strings() -> &'static [&'static str] {
        &["year"]
    }
}

impl std::fmt::Display for Year {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let mut buf = itoa::Buffer::new();
        let str = buf.format(self.0);
        f.write_str(str)
    }
}

impl Formattable for Year {
    #[inline(always)]
    fn write_to(&self, buf: &mut Vec<u8>) {
        let mut b = itoa::Buffer::new();
        buf.extend_from_slice(b.format(self.0).as_bytes());
    }
}
