use std::{
    collections::BTreeMap,
    thread::sleep,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use brk_error::{Error, Result};
use brk_types::{Cents, Close, Date, Dollars, Height, High, Low, OHLCCents, Open, Timestamp};
use serde_json::Value;
use tracing::info;
use ureq::Agent;

use crate::{PriceSource, checked_get, ohlc::date_from_timestamp};

/// Coinbase Exchange returns at most 300 candles per request, so history is
/// backfilled one 300-day window at a time.
const WINDOW_DAYS: i64 = 300;
const DAY_SECS: i64 = 86_400;

/// Coinbase Exchange daily-candle source. Unlike Binance (geo-restricted) and
/// Kraken (last ~720 candles only), Coinbase serves the full daily history for a
/// product via paginated `start`/`end` unix-time queries, making it the primary
/// price source for chains without BRK's on-chain oracle (e.g. Litecoin).
#[derive(Clone)]
pub struct Coinbase {
    agent: Agent,
    product: &'static str,
    /// Unix seconds to start backfilling from (typically the chain's genesis).
    /// Windows before the product's listing return an empty array and are
    /// skipped, so an early start just costs a few extra empty requests.
    start_unix: i64,
    _1d: Option<BTreeMap<Date, OHLCCents>>,
}

impl Coinbase {
    pub fn new_with_agent(agent: Agent, product: &'static str, start_unix: i64) -> Self {
        Self {
            agent,
            product,
            start_unix,
            _1d: None,
        }
    }

    fn ensure_loaded(&mut self) -> Result<()> {
        if self._1d.is_none() {
            self._1d.replace(self.fetch_all_daily()?);
        }
        Ok(())
    }

    /// Full daily OHLC history, fetched via 300-candle windows from
    /// `start_unix` to now.
    pub fn fetch_all_daily(&self) -> Result<BTreeMap<Date, OHLCCents>> {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_secs() as i64)
            .unwrap_or(0);

        let mut out: BTreeMap<Date, OHLCCents> = BTreeMap::new();
        let mut start = self.start_unix;

        while start < now {
            let end = (start + WINDOW_DAYS * DAY_SECS).min(now);
            let url = format!(
                "https://api.exchange.coinbase.com/products/{}/candles?granularity=86400&start={start}&end={end}",
                self.product
            );
            info!("Fetching {url} ...");

            let bytes = checked_get(&self.agent, &url)?;
            let json: Value = serde_json::from_slice(&bytes)?;

            // Response is an array of `[time, low, high, open, close, volume]`
            // (descending by time). A non-array body is an error object.
            let arr = json
                .as_array()
                .ok_or_else(|| Error::Parse(format!("Coinbase: unexpected response {json}")))?;

            for candle in arr.iter().filter_map(|v| v.as_array()) {
                let ts = candle.first().and_then(|v| v.as_i64()).unwrap_or(0);
                let cents = |i: usize| {
                    Cents::from(Dollars::from(
                        candle.get(i).and_then(|v| v.as_f64()).unwrap_or(0.0),
                    ))
                };
                let date = date_from_timestamp(Timestamp::from(ts as u32));
                out.insert(
                    date,
                    OHLCCents::from((
                        Open::new(cents(3)),
                        High::new(cents(2)),
                        Low::new(cents(1)),
                        Close::new(cents(4)),
                    )),
                );
            }

            start = end;
            // Stay well under Coinbase's public rate limit (~10 req/s).
            sleep(Duration::from_millis(200));
        }

        if out.is_empty() {
            return Err(Error::NotFound(format!(
                "Coinbase returned no candles for {}",
                self.product
            )));
        }

        Ok(out)
    }

    /// Full daily close-price history as `Date -> Cents`, loaded (and cached) on
    /// first call.
    pub fn daily_closes(&mut self) -> Result<BTreeMap<Date, Cents>> {
        self.ensure_loaded()?;
        Ok(self
            ._1d
            .as_ref()
            .unwrap()
            .iter()
            .map(|(date, ohlc)| (*date, *ohlc.close))
            .collect())
    }

    pub fn ping(&self) -> Result<()> {
        self.agent
            .get("https://api.exchange.coinbase.com/time")
            .call()?;
        Ok(())
    }
}

impl PriceSource for Coinbase {
    fn name(&self) -> &'static str {
        "Coinbase"
    }

    fn get_date(&mut self, date: Date) -> Option<Result<OHLCCents>> {
        if let Err(e) = self.ensure_loaded() {
            return Some(Err(e));
        }
        Some(
            self._1d
                .as_ref()
                .unwrap()
                .get(&date)
                .cloned()
                .ok_or(Error::NotFound("Couldn't find date in Coinbase".into())),
        )
    }

    fn get_1mn(
        &mut self,
        _timestamp: Timestamp,
        _previous_timestamp: Option<Timestamp>,
    ) -> Option<Result<OHLCCents>> {
        None // Coinbase daily source doesn't serve minute data
    }

    fn get_height(&mut self, _height: Height) -> Option<Result<OHLCCents>> {
        None // Coinbase doesn't support height-based queries
    }

    fn ping(&self) -> Result<()> {
        self.ping()
    }

    fn clear(&mut self) {
        self._1d.take();
    }
}
