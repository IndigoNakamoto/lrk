//! Each block's exchange price range, the yardstick `oracle_tune` measures the oracle against:
//! the LTC/USD low and high over the minutes from the previous block's time to its own.
//!
//! `cargo run --example oracle_truth --features litecoin -- <dir> [start]` reads
//! `<dir>/timestamp.u32` (from `oracle_extract`) and writes `<dir>/band_height.f64`, one
//! `(low, high)` pair of dollars per height, `(0, 0)` below `start` (default 2,200,000) and where
//! no exchange covers the block. Bitfinex minute candles price a block; a block whose minutes
//! Bitfinex didn't trade in takes the close of its last traded minute within the hour, and failing
//! that Coinbase's hourly candle. Candles are cached in `<dir>/candles_1m.bin` and
//! `<dir>/candles_1h.bin`, and a rerun only fetches what's newer.

use std::{
    env, fs,
    io::Read,
    path::{Path, PathBuf},
    thread::sleep,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use serde_json::Value;
use ureq::Agent;

/// A block's span reaches back at most this far: out-of-order times and long gaps can't widen it.
const MAX_SPAN_SECS: u32 = 30 * 60;
/// How far back a block without a traded minute of its own reads the last close.
const CARRY_SECS: u32 = 60 * 60;
/// Bitfinex caps candle responses at 10,000 rows.
const BITFINEX_PAGE: usize = 10_000;
/// Coinbase returns at most 300 candles per request.
const COINBASE_PAGE: u32 = 300;

/// One candle: its period's start (unix seconds), low, high and close in dollars.
#[derive(Clone, Copy)]
struct Candle {
    start: u32,
    low: f64,
    high: f64,
    close: f64,
}

const CANDLE_BYTES: usize = 4 + 3 * 8;

impl Candle {
    fn to_bytes(self) -> [u8; CANDLE_BYTES] {
        let mut b = [0; CANDLE_BYTES];
        b[..4].copy_from_slice(&self.start.to_le_bytes());
        b[4..12].copy_from_slice(&self.low.to_le_bytes());
        b[12..20].copy_from_slice(&self.high.to_le_bytes());
        b[20..].copy_from_slice(&self.close.to_le_bytes());
        b
    }

    fn from_bytes(b: &[u8]) -> Self {
        let f = |r: std::ops::Range<usize>| f64::from_le_bytes(b[r].try_into().unwrap());
        Self {
            start: u32::from_le_bytes(b[..4].try_into().unwrap()),
            low: f(4..12),
            high: f(12..20),
            close: f(20..28),
        }
    }
}

fn now_unix() -> u32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |d| d.as_secs() as u32)
}

fn agent() -> Agent {
    Agent::config_builder()
        .timeout_global(Some(Duration::from_secs(60)))
        .http_status_as_error(false)
        .build()
        .into()
}

/// GET `url` as JSON, waiting out rate limits and retrying transient failures.
fn get_json(agent: &Agent, url: &str) -> Value {
    for attempt in 1.. {
        let result = agent
            .get(url)
            .call()
            .map_err(|e| e.to_string())
            .and_then(|mut r| {
                let status = r.status().as_u16();
                let mut body = Vec::new();
                r.body_mut()
                    .as_reader()
                    .read_to_end(&mut body)
                    .map_err(|e| e.to_string())?;
                match status {
                    200 => serde_json::from_slice(&body).map_err(|e| e.to_string()),
                    _ => Err(format!("HTTP {status}: {}", String::from_utf8_lossy(&body))),
                }
            });
        match result {
            Ok(json) => return json,
            Err(error) if attempt < 10 => {
                eprintln!("GET {url}: {error}; retrying in a minute");
                sleep(Duration::from_secs(60));
            }
            Err(error) => panic!("GET {url}: {error}"),
        }
    }
    unreachable!()
}

/// The cached candles in `path`, oldest first.
fn load(path: &Path) -> Vec<Candle> {
    fs::read(path)
        .map(|b| {
            b.chunks_exact(CANDLE_BYTES)
                .map(Candle::from_bytes)
                .collect()
        })
        .unwrap_or_default()
}

fn append(path: &Path, candles: &[Candle]) {
    use std::io::Write;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .unwrap();
    for c in candles {
        file.write_all(&c.to_bytes()).unwrap();
    }
}

/// Bitfinex's LTC/USD minute candles from `from` to `to`, appended to the cache page by page.
/// Rows are `[MTS, OPEN, CLOSE, HIGH, LOW, VOLUME]`, ascending with `sort=1`.
fn fetch_bitfinex(agent: &Agent, cache: &Path, from: u32, to: u32) {
    let mut start = from;
    while start < to {
        let url = format!(
            "https://api-pub.bitfinex.com/v2/candles/trade:1m:tLTCUSD/hist?start={}&end={}&limit={BITFINEX_PAGE}&sort=1",
            start as u64 * 1000,
            to as u64 * 1000
        );
        let json = get_json(agent, &url);
        let rows = json.as_array().expect("Bitfinex candles are an array");
        let candles: Vec<Candle> = rows
            .iter()
            .filter_map(|row| {
                let row = row.as_array()?;
                let f = |i: usize| row.get(i)?.as_f64();
                Some(Candle {
                    start: (row.first()?.as_u64()? / 1000) as u32,
                    close: f(2)?,
                    high: f(3)?,
                    low: f(4)?,
                })
            })
            .filter(|c| c.start >= start && c.low > 0.0)
            .collect();
        append(cache, &candles);
        let Some(last) = candles.last() else { break };
        eprintln!(
            "Bitfinex 1m: {} candles through {}",
            candles.len(),
            last.start
        );
        start = last.start + 60;
        if rows.len() < BITFINEX_PAGE {
            break;
        }
        sleep(Duration::from_millis(2500));
    }
}

/// Coinbase's LTC/USD hourly candles from `from` to `to`, appended to the cache window by window.
/// Rows are `[time, low, high, open, close, volume]`, descending.
fn fetch_coinbase(agent: &Agent, cache: &Path, from: u32, to: u32) {
    let mut start = from;
    while start < to {
        let end = (start + COINBASE_PAGE * 3600).min(to);
        let url = format!(
            "https://api.exchange.coinbase.com/products/LTC-USD/candles?granularity=3600&start={start}&end={end}"
        );
        let json = get_json(agent, &url);
        let mut candles: Vec<Candle> = json
            .as_array()
            .expect("Coinbase candles are an array")
            .iter()
            .filter_map(|row| {
                let row = row.as_array()?;
                let f = |i: usize| row.get(i)?.as_f64();
                Some(Candle {
                    start: row.first()?.as_u64()? as u32,
                    low: f(1)?,
                    high: f(2)?,
                    close: f(4)?,
                })
            })
            .filter(|c| (start..end).contains(&c.start) && c.low > 0.0)
            .collect();
        candles.sort_by_key(|c| c.start);
        append(cache, &candles);
        start = end;
        sleep(Duration::from_millis(200));
    }
    eprintln!("Coinbase 1h: through {to}");
}

/// The candles of one period length, indexed by period from the first.
struct Series {
    first: u32,
    period: u32,
    candles: Vec<Option<Candle>>,
}

impl Series {
    fn new(candles: &[Candle], period: u32) -> Self {
        let first = candles.first().map_or(0, |c| c.start / period);
        let last = candles.last().map_or(0, |c| c.start / period);
        let mut slots = vec![None; (last + 1 - first) as usize];
        candles
            .iter()
            .for_each(|c| slots[(c.start / period - first) as usize] = Some(*c));
        Self {
            first,
            period,
            candles: slots,
        }
    }

    fn at(&self, time: u32) -> Option<Candle> {
        let slot = (time / self.period).checked_sub(self.first)?;
        *self.candles.get(slot as usize)?
    }
}

#[derive(Default, Debug)]
struct Coverage {
    minutes: usize,
    carried: usize,
    hourly: usize,
    uncovered: usize,
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let dir = PathBuf::from(&args[0]);
    let start: usize = args.get(1).map_or(2_200_000, |s| s.parse().unwrap());
    let timestamps: Vec<u32> = fs::read(dir.join("timestamp.u32"))
        .expect("timestamp.u32 (run oracle_extract)")
        .chunks_exact(4)
        .map(|c| u32::from_le_bytes(c.try_into().unwrap()))
        .collect();
    let from = timestamps[start..].iter().min().unwrap() - MAX_SPAN_SECS - CARRY_SECS;
    let to = (timestamps[start..].iter().max().unwrap() + 3600).min(now_unix());

    let agent = agent();
    let minute_cache = dir.join("candles_1m.bin");
    let hour_cache = dir.join("candles_1h.bin");
    let resume = |cache: &Path, period: u32| {
        load(cache)
            .last()
            .map_or(from, |c| c.start + period)
            .max(from)
    };
    fetch_bitfinex(&agent, &minute_cache, resume(&minute_cache, 60), to);
    fetch_coinbase(&agent, &hour_cache, resume(&hour_cache, 3600), to);
    let minutes = Series::new(&load(&minute_cache), 60);
    let hours = Series::new(&load(&hour_cache), 3600);

    let mut coverage = Coverage::default();
    let mut bands = vec![0.0f64; timestamps.len() * 2];
    for h in start..timestamps.len() {
        let time = timestamps[h];
        let prev = timestamps[h - 1];
        let span_start = prev.clamp(time.saturating_sub(MAX_SPAN_SECS), time);
        let traded: Vec<Candle> = (span_start / 60..=time / 60)
            .filter_map(|m| minutes.at(m * 60))
            .collect();
        let band = if !traded.is_empty() {
            coverage.minutes += 1;
            Some(traded.iter().fold((f64::MAX, f64::MIN), |(lo, hi), c| {
                (lo.min(c.low), hi.max(c.high))
            }))
        } else if let Some(c) =
            (1..=CARRY_SECS / 60).find_map(|back| minutes.at(span_start.checked_sub(back * 60)?))
        {
            coverage.carried += 1;
            Some((c.close, c.close))
        } else if let Some(c) = hours.at(time) {
            coverage.hourly += 1;
            Some((c.low, c.high))
        } else {
            coverage.uncovered += 1;
            None
        };
        if let Some((low, high)) = band {
            bands[2 * h] = low;
            bands[2 * h + 1] = high;
        }
    }
    let bytes: Vec<u8> = bands.iter().flat_map(|v| v.to_le_bytes()).collect();
    fs::write(dir.join("band_height.f64"), bytes).unwrap();
    eprintln!("Blocks {start}..{}: {coverage:?}", timestamps.len());
}
