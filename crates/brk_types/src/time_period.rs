use std::fmt;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use brk_chain::Chain;

/// Time period for mining statistics.
///
/// Used to specify the lookback window for pool statistics, hashrate calculations,
/// and other time-based mining series.
#[derive(Default, Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
pub enum TimePeriod {
    #[default]
    #[serde(rename = "24h")]
    Day,
    #[serde(rename = "3d")]
    ThreeDays,
    #[serde(rename = "1w")]
    Week,
    #[serde(rename = "1m")]
    Month,
    #[serde(rename = "3m")]
    ThreeMonths,
    #[serde(rename = "6m")]
    SixMonths,
    #[serde(rename = "1y")]
    Year,
    #[serde(rename = "2y")]
    TwoYears,
    #[serde(rename = "3y")]
    ThreeYears,
    #[serde(rename = "all")]
    All,
}

impl TimePeriod {
    /// Approximate number of blocks for this time period (10 min/block, Bitcoin default).
    pub fn block_count(&self) -> usize {
        self.block_count_for_chain(Chain::Bitcoin)
    }

    /// Approximate number of blocks for this time period given a specific chain's
    /// block target time.
    pub fn block_count_for_chain(&self, chain: Chain) -> usize {
        let secs = chain.constants().seconds_per_block;
        let day = (86_400 / secs) as usize;
        match self {
            TimePeriod::Day => day,
            TimePeriod::ThreeDays => day * 3,
            TimePeriod::Week => day * 7,
            TimePeriod::Month => day * 30,
            TimePeriod::ThreeMonths => day * 90,
            TimePeriod::SixMonths => day * 180,
            TimePeriod::Year => day * 365,
            TimePeriod::TwoYears => day * 365 * 2,
            TimePeriod::ThreeYears => day * 365 * 3,
            TimePeriod::All => usize::MAX,
        }
    }

    /// Blocks per year for the given chain.
    pub fn blocks_per_year(chain: Chain) -> u64 {
        chain.constants().blocks_per_year
    }

    /// Parse from URL path segment
    pub fn from_path(s: &str) -> Option<Self> {
        match s {
            "24h" => Some(TimePeriod::Day),
            "3d" => Some(TimePeriod::ThreeDays),
            "1w" => Some(TimePeriod::Week),
            "1m" => Some(TimePeriod::Month),
            "3m" => Some(TimePeriod::ThreeMonths),
            "6m" => Some(TimePeriod::SixMonths),
            "1y" => Some(TimePeriod::Year),
            "2y" => Some(TimePeriod::TwoYears),
            "3y" => Some(TimePeriod::ThreeYears),
            "all" => Some(TimePeriod::All),
            _ => None,
        }
    }
}

impl fmt::Display for TimePeriod {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TimePeriod::Day => write!(f, "24h"),
            TimePeriod::ThreeDays => write!(f, "3d"),
            TimePeriod::Week => write!(f, "1w"),
            TimePeriod::Month => write!(f, "1m"),
            TimePeriod::ThreeMonths => write!(f, "3m"),
            TimePeriod::SixMonths => write!(f, "6m"),
            TimePeriod::Year => write!(f, "1y"),
            TimePeriod::TwoYears => write!(f, "2y"),
            TimePeriod::ThreeYears => write!(f, "3y"),
            TimePeriod::All => write!(f, "all"),
        }
    }
}
