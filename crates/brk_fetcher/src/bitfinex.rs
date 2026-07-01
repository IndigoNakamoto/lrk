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

/// Bitfinex caps candle responses at 10 000 rows. At daily granularity that is
/// ~27 years, so the full history usually fits in a single request, but the
/// fetch still paginates defensively in case a symbol ever exceeds the cap.
const PAGE_LIMIT: usize = 10_000;
const DAY_MS: i64 = 86_400_000;

/// Bitfinex daily-candle source. Bitfinex has listed LTC/USD since 2013
/// (and BTC/USD earlier), so it reaches ~3 years further back than Coinbase's
/// 2016 listing. It is used to backfill the early-history gap for chains priced
/// from exchanges rather than BRK's on-chain oracle (e.g. Litecoin).
#[derive(Clone)]
pub struct Bitfinex {
    agent: Agent,
    /// Bitfinex trading symbol, e.g. `"tLTCUSD"`.
    symbol: &'static str,
    /// Unix milliseconds to start backfilling from (typically the chain's
    /// genesis). Windows before the symbol's listing return an empty array and
    /// stop pagination, so an early start is harmless.
    start_ms: i64,
    _1d: Option<BTreeMap<Date, OHLCCents>>,
}

impl Bitfinex {
    pub fn new_with_agent(agent: Agent, symbol: &'static str, start_ms: i64) -> Self {
        Self {
            agent,
            symbol,
            start_ms,
            _1d: None,
        }
    }

    fn ensure_loaded(&mut self) -> Result<()> {
        if self._1d.is_none() {
            self._1d.replace(self.fetch_all_daily()?);
        }
        Ok(())
    }

    /// Full daily OHLC history from `start_ms` to now, paginated by `start`.
    pub fn fetch_all_daily(&self) -> Result<BTreeMap<Date, OHLCCents>> {
        let now_ms = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_millis() as i64)
            .unwrap_or(0);

        let mut out: BTreeMap<Date, OHLCCents> = BTreeMap::new();
        let mut start = self.start_ms;

        while start < now_ms {
            let url = format!(
                "https://api-pub.bitfinex.com/v2/candles/trade:1D:{}/hist?start={start}&limit={PAGE_LIMIT}&sort=1",
                self.symbol
            );
            info!("Fetching {url} ...");

            let bytes = checked_get(&self.agent, &url)?;
            let json: Value = serde_json::from_slice(&bytes)?;

            // Response is an array of `[MTS, OPEN, CLOSE, HIGH, LOW, VOLUME]`
            // (ascending by time with `sort=1`). A non-array body is an error.
            let arr = json
                .as_array()
                .ok_or_else(|| Error::Parse(format!("Bitfinex: unexpected response {json}")))?;

            if arr.is_empty() {
                break;
            }

            let mut last_ms = start;
            for candle in arr.iter().filter_map(|v| v.as_array()) {
                let ts_ms = candle.first().and_then(|v| v.as_i64()).unwrap_or(0);
                last_ms = last_ms.max(ts_ms);
                let cents = |i: usize| {
                    Cents::from(Dollars::from(
                        candle.get(i).and_then(|v| v.as_f64()).unwrap_or(0.0),
                    ))
                };
                let date = date_from_timestamp(Timestamp::from((ts_ms / 1000) as u32));
                out.insert(
                    date,
                    OHLCCents::from((
                        Open::new(cents(1)),
                        High::new(cents(3)),
                        Low::new(cents(4)),
                        Close::new(cents(2)),
                    )),
                );
            }

            // Fewer rows than the cap means we reached the present.
            if arr.len() < PAGE_LIMIT {
                break;
            }
            start = last_ms + DAY_MS;
            // Bitfinex public candle endpoint allows ~30 req/min.
            sleep(Duration::from_millis(2500));
        }

        if out.is_empty() {
            return Err(Error::NotFound(format!(
                "Bitfinex returned no candles for {}",
                self.symbol
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
            .get("https://api-pub.bitfinex.com/v2/platform/status")
            .call()?;
        Ok(())
    }
}

impl PriceSource for Bitfinex {
    fn name(&self) -> &'static str {
        "Bitfinex"
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
                .ok_or(Error::NotFound("Couldn't find date in Bitfinex".into())),
        )
    }

    fn get_1mn(
        &mut self,
        _timestamp: Timestamp,
        _previous_timestamp: Option<Timestamp>,
    ) -> Option<Result<OHLCCents>> {
        None // Bitfinex daily source doesn't serve minute data
    }

    fn get_height(&mut self, _height: Height) -> Option<Result<OHLCCents>> {
        None // Bitfinex doesn't support height-based queries
    }

    fn ping(&self) -> Result<()> {
        self.ping()
    }

    fn clear(&mut self) {
        self._1d.take();
    }
}
