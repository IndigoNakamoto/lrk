//! Litecoin block prices from exchange daily closes, below the oracle's first height.
//!
//! `brk_oracle`'s pre-oracle tape is BTC/USD, so Litecoin prices each block below
//! [`START_HEIGHT_SLOW`](brk_oracle::START_HEIGHT_SLOW) at the LTC/USD daily close of its date
//! instead, and seeds the oracle at that height's.
//! Bitfinex lists LTC/USD from 2013, Coinbase from 2016; Coinbase overlays Bitfinex where both
//! exist. Heights before the first listing carry the earliest close.

use std::{
    collections::BTreeMap,
    io::Read,
    sync::{Mutex, MutexGuard, PoisonError},
    thread::sleep,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use bitview_primitives::Date;
use brk_error::{Error, Result};
use brk_types::{Cents, Dollars, Timestamp};
use serde_json::Value;
use tracing::{info, warn};
use ureq::Agent;

const BITFINEX_SYMBOL: &str = "tLTCUSD";
const COINBASE_PRODUCT: &str = "LTC-USD";
/// Litecoin's genesis block time (2011-10-07).
const GENESIS_UNIX: i64 = 1_317_972_665;
const DAY_SECS: i64 = 86_400;
/// Bitfinex caps candle responses at 10,000 rows.
const BITFINEX_PAGE: usize = 10_000;
/// Coinbase returns at most 300 candles per request.
const COINBASE_WINDOW_DAYS: i64 = 300;
/// How far back an incremental refresh re-reads, so a late-closing candle gets its final value.
const REFRESH_DAYS: i64 = 3;

/// Bump when the sources or the per-block pricing rule change, so stored prices recompute.
pub const VERSION: u32 = 1;

/// Daily LTC/USD closes, fetched in full once and then refreshed from the last few days.
#[derive(Default)]
pub struct ExchangeCloses {
    closes: BTreeMap<Date, Cents>,
}

/// The process's closes, shared across updates so only the first one downloads the history.
pub fn closes() -> MutexGuard<'static, ExchangeCloses> {
    static CLOSES: Mutex<ExchangeCloses> = Mutex::new(ExchangeCloses {
        closes: BTreeMap::new(),
    });
    CLOSES.lock().unwrap_or_else(PoisonError::into_inner)
}

impl ExchangeCloses {
    /// Makes sure `through` has a close, fetching the full history on first use and only the
    /// recent window after. A failed refresh keeps the cached closes (later dates forward-fill);
    /// only an empty history is an error.
    pub fn ensure(&mut self, through: Date) -> Result<()> {
        let agent = agent();
        if self.closes.is_empty() {
            match bitfinex(&agent, GENESIS_UNIX) {
                Ok(closes) => self.closes.extend(closes),
                Err(error) => warn!("Bitfinex LTC/USD history unavailable: {error}"),
            }
            match coinbase(&agent, GENESIS_UNIX) {
                Ok(closes) => self.closes.extend(closes),
                Err(error) => warn!("Coinbase LTC/USD history unavailable: {error}"),
            }
            if self.closes.is_empty() {
                return Err(Error::NotFound(
                    "no LTC/USD exchange history (Bitfinex and Coinbase both failed)".into(),
                ));
            }
            info!("Loaded {} LTC/USD daily closes", self.closes.len());
        } else if self.last_date().is_some_and(|last| last < through) {
            match coinbase(&agent, now_unix() - REFRESH_DAYS * DAY_SECS) {
                Ok(closes) => self.closes.extend(closes),
                Err(error) => warn!("LTC/USD refresh failed, forward-filling: {error}"),
            }
        }
        Ok(())
    }

    fn last_date(&self) -> Option<Date> {
        self.closes.keys().next_back().copied()
    }

    /// The close at or before `date`, else the earliest close.
    pub fn price_at(&self, date: Date) -> Cents {
        self.closes
            .range(..=date)
            .next_back()
            .or_else(|| self.closes.iter().next())
            .map_or(Cents::ZERO, |(_, cents)| *cents)
    }
}

fn agent() -> Agent {
    Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(30)))
        .http_status_as_error(false)
        .build()
        .into()
}

fn now_unix() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as i64)
}

fn get_json(agent: &Agent, url: &str) -> Result<Value> {
    let mut response = agent
        .get(url)
        .call()
        .map_err(|error| Error::Parse(format!("GET {url}: {error}")))?;
    let status = response.status().as_u16();
    let mut body = Vec::new();
    response
        .body_mut()
        .as_reader()
        .read_to_end(&mut body)
        .map_err(|error| Error::Parse(format!("GET {url}: {error}")))?;
    if status >= 400 {
        return Err(Error::Parse(format!("GET {url}: HTTP {status}")));
    }
    serde_json::from_slice(&body).map_err(|error| Error::Parse(format!("GET {url}: {error}")))
}

fn close_cents(candle: &[Value], index: usize) -> Option<Cents> {
    let dollars = candle.get(index)?.as_f64()?;
    (dollars > 0.0).then(|| Cents::from(Dollars::from(dollars)))
}

fn date_of(unix: i64) -> Date {
    Date::from(Timestamp::from(unix as u32))
}

/// Rows are `[MTS, OPEN, CLOSE, HIGH, LOW, VOLUME]`, ascending with `sort=1`.
fn bitfinex(agent: &Agent, start_unix: i64) -> Result<BTreeMap<Date, Cents>> {
    let now_ms = now_unix() * 1000;
    let mut start_ms = start_unix * 1000;
    let mut out = BTreeMap::new();
    while start_ms < now_ms {
        let url = format!(
            "https://api-pub.bitfinex.com/v2/candles/trade:1D:{BITFINEX_SYMBOL}/hist?start={start_ms}&limit={BITFINEX_PAGE}&sort=1"
        );
        let json = get_json(agent, &url)?;
        let rows = json
            .as_array()
            .ok_or_else(|| Error::Parse(format!("Bitfinex: unexpected response {json}")))?;
        let mut last_ms = start_ms;
        for row in rows.iter().filter_map(Value::as_array) {
            let Some(ms) = row.first().and_then(Value::as_i64) else {
                continue;
            };
            last_ms = last_ms.max(ms);
            if let Some(cents) = close_cents(row, 2) {
                out.insert(date_of(ms / 1000), cents);
            }
        }
        if rows.len() < BITFINEX_PAGE {
            break;
        }
        start_ms = last_ms + DAY_SECS * 1000;
        sleep(Duration::from_millis(2500));
    }
    Ok(out)
}

/// Rows are `[time, low, high, open, close, volume]`, descending.
fn coinbase(agent: &Agent, start_unix: i64) -> Result<BTreeMap<Date, Cents>> {
    let now = now_unix();
    let mut start = start_unix;
    let mut out = BTreeMap::new();
    while start < now {
        let end = (start + COINBASE_WINDOW_DAYS * DAY_SECS).min(now);
        let url = format!(
            "https://api.exchange.coinbase.com/products/{COINBASE_PRODUCT}/candles?granularity=86400&start={start}&end={end}"
        );
        let json = get_json(agent, &url)?;
        let rows = json
            .as_array()
            .ok_or_else(|| Error::Parse(format!("Coinbase: unexpected response {json}")))?;
        for row in rows.iter().filter_map(Value::as_array) {
            if let (Some(time), Some(cents)) =
                (row.first().and_then(Value::as_i64), close_cents(row, 4))
            {
                out.insert(date_of(time), cents);
            }
        }
        start = end;
        sleep(Duration::from_millis(200));
    }
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs network: `cargo test -p bitview_plugin_price --features litecoin -- --ignored`.
    #[test]
    #[ignore]
    fn fetches_ltc_history() {
        let mut closes = ExchangeCloses::default();
        let today = Date::from(Timestamp::from(now_unix() as u32));
        closes.ensure(today).unwrap();
        let first = *closes.closes.keys().next().unwrap();
        assert!(first < Date::new(2014, 1, 1), "history reaches 2013");
        // LTC traded between $1 and $500 for its whole listed history.
        for (date, cents) in &closes.closes {
            let dollars = f64::from(Dollars::from(*cents));
            assert!((1.0..500.0).contains(&dollars), "{date:?}: ${dollars}");
        }
        let two_days_ago = Date::from(Timestamp::from((now_unix() - 2 * DAY_SECS) as u32));
        assert!(closes.last_date().unwrap() >= two_days_ago);
    }
}
