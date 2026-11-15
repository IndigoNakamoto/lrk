//! Runs the oracle over `oracle_extract`'s bins under each config given and reports its distance
//! from the exchange price (`price_height.f64`, dollars per height) in height buckets. With
//! `oracle_truth`'s `band_height.f64` and `oracle_extract`'s `timestamp.u32` beside them, it also
//! reports the distance from each block's exchange range (0 inside it), overall and by year.
//!
//! `cargo run --example oracle_tune --features litecoin -- <dir> <start> <spec>...`, each spec
//! `alpha,window,below,above,shape[,guard[,relock]]`: `guard` (0 off) reseeds at the exchange
//! price past that distance; `relock` (0 off) is how many agreeing votes in a row, one every 144
//! blocks, move the oracle onto an alias of it that the last two days' payments fit better
//! (`template_log.f64`; knobs `MARGIN` and `AGREE`).

use std::{
    collections::{BTreeMap, VecDeque},
    convert::Infallible,
    env, fs,
    path::PathBuf,
};

use brk_oracle::{Config, HistogramRaw, Oracle, bin_to_cents, cents_to_bin};
use rayon::prelude::*;

const BUCKET: usize = 200_000;
const DAY: usize = 1152;
const CHECK: usize = 144;
/// The template's offsets from $100's bin.
const SPAN: i64 = 520;
/// Bins (mod 200) near round LTC amounts: masked from the global search.
const MASK: [usize; 15] = [0, 1, 59, 60, 61, 79, 80, 81, 94, 95, 96, 139, 140, 141, 199];

fn read<T: Copy, const N: usize>(path: PathBuf, from: fn([u8; N]) -> T) -> Vec<T> {
    fs::read(&path)
        .unwrap_or_else(|e| panic!("{}: {e}", path.display()))
        .chunks_exact(N)
        .map(|c| from(c.try_into().unwrap()))
        .collect()
}

/// The window's payments as the template reads them: shares of the unmasked total, log-compressed,
/// less their 41-bin running mean; masked bins NaN.
fn prepare(window: &[u32], masked: &[bool]) -> Option<Vec<f64>> {
    let total: f64 = (0..2400)
        .filter(|&b| !masked[b])
        .map(|b| window[b] as f64)
        .sum();
    if total <= 0.0 {
        return None;
    }
    let v: Vec<f64> = (0..2400)
        .map(|b| {
            if masked[b] {
                f64::NAN
            } else {
                (window[b] as f64 / total * 1e4).ln_1p()
            }
        })
        .collect();
    Some(
        (0..2400)
            .map(|b| {
                if masked[b] {
                    return f64::NAN;
                }
                let (lo, hi) = (b.saturating_sub(20), (b + 21).min(2400));
                let (sum, n) = v[lo..hi]
                    .iter()
                    .filter(|x| !x.is_nan())
                    .fold((0.0, 0.0), |(s, n), x| (s + x, n + 1.0));
                v[b] - sum / n
            })
            .collect(),
    )
}

/// The correlation between the prepared window and the template with $100 at bin `c`.
fn correlation(h: &[f64], template: &[f64], c: i64) -> f64 {
    let (mut n, mut sx, mut sy, mut sxx, mut syy, mut sxy) = (0.0, 0.0, 0.0, 0.0, 0.0, 0.0);
    for (i, &t) in template.iter().enumerate() {
        let b = c + i as i64 - SPAN;
        if !(0..2400).contains(&b) || h[b as usize].is_nan() {
            continue;
        }
        let x = h[b as usize];
        n += 1.0;
        sx += x;
        sy += t;
        sxx += x * x;
        syy += t * t;
        sxy += x * t;
    }
    if n < 200.0 {
        return f64::MIN;
    }
    (sxy - sx * sy / n)
        / ((sxx - sx * sx / n) * (syy - sy * sy / n))
            .sqrt()
            .max(1e-30)
}

/// Bin distances of the round-dollar ladder's aliases: 1.5, 2, 2.5, 3 and 5 times.
const ALIASES: [i64; 5] = [35, 60, 80, 95, 140];

/// The alias of `oracle` the window's payments fit better than `oracle` itself by more than
/// `margin`, each read at its best within 6 bins.
fn alias_vote(h: &[f64], template: &[f64], oracle: f64, margin: f64) -> Option<f64> {
    let best_near = |c: i64| {
        (-6..=6)
            .map(|k| (correlation(h, template, c + k), c + k))
            .max_by(|a, b| a.0.total_cmp(&b.0))
            .unwrap()
    };
    let center = oracle.round() as i64;
    let here = best_near(center).0;
    let (score, at) = ALIASES
        .iter()
        .flat_map(|&a| [center + a, center - a])
        .map(best_near)
        .max_by(|a, b| a.0.total_cmp(&b.0))
        .unwrap();
    (score - here > margin).then_some(at as f64)
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let dir = PathBuf::from(&args[0]);
    let start: usize = args[1].parse().unwrap();
    let offsets = read(dir.join("offsets.u64"), u64::from_le_bytes);
    let bins = read(dir.join("bins.u16"), u16::from_le_bytes);
    let prices = read(dir.join("price_height.f64"), f64::from_le_bytes);
    let template = fs::exists(dir.join("template_log.f64"))
        .unwrap()
        .then(|| read(dir.join("template_log.f64"), f64::from_le_bytes))
        .unwrap_or_default();
    let bands = (fs::exists(dir.join("band_height.f64")).unwrap()
        && fs::exists(dir.join("timestamp.u32")).unwrap())
    .then(|| {
        (
            read(dir.join("band_height.f64"), f64::from_le_bytes),
            read(dir.join("timestamp.u32"), u32::from_le_bytes),
        )
    });
    let masked: Vec<bool> = (0..2400).map(|b| MASK.contains(&(b % 200))).collect();
    let end = (offsets.len() - 1).min(prices.len());
    let block = |h: usize| &bins[offsets[h] as usize..offsets[h + 1] as usize];
    // Agreeing searches lie within AGREE bins of their median; a relock needs the oracle STRAY away.
    let knob = |name: &str, default: f64| env::var(name).map_or(default, |v| v.parse().unwrap());
    let (agree, margin) = (knob("AGREE", 10.0), knob("MARGIN", 0.05));

    let reports: Vec<String> = args[2..]
        .par_iter()
        .map(|spec| {
            let v: Vec<f64> = spec.split(',').map(|x| x.parse().unwrap()).collect();
            let config = Config::custom(v[0], v[1] as usize, v[2] as usize, v[3] as usize, v[4]);
            let guard = v.get(5).filter(|&&g| g > 0.0).map(|g| (1.0 + g).ln());
            let relock = v.get(6).map_or(0, |&k| k as usize);
            let seed = cents_to_bin(prices[start] * 100.0);
            let Ok(mut oracle) = Oracle::from_checkpoint(seed, config, |_| Ok::<_, Infallible>(()));
            let mut recent: VecDeque<HistogramRaw> = VecDeque::new();
            let mut day = vec![0u32; 2400];
            for h in start.saturating_sub(DAY)..start {
                block(h).iter().for_each(|&b| day[b as usize] += 1);
            }
            let mut searches: VecDeque<Option<f64>> = VecDeque::new();
            let (mut reseeds, mut relocks) = (0, 0);
            let mut errors: Vec<Vec<f64>> = vec![Vec::new(); end / BUCKET + 1];
            let mut band_errors: BTreeMap<i32, Vec<f64>> = BTreeMap::new();
            let mut uncovered = 0usize;
            let mut path = Vec::with_capacity((end - start) * 8);
            let reseed = |oracle: &mut Oracle, recent: &VecDeque<HistogramRaw>, bin: f64| {
                let Ok(o) = Oracle::from_checkpoint(bin, config, |o| {
                    recent.iter().for_each(|h| {
                        o.process_histogram(h);
                    });
                    Ok::<_, Infallible>(())
                });
                *oracle = o;
            };
            for h in start..end {
                let mut hist = HistogramRaw::default();
                for &b in block(h) {
                    hist.increment(b as usize);
                    day[b as usize] += 1;
                }
                if h >= DAY {
                    block(h - DAY).iter().for_each(|&b| day[b as usize] -= 1);
                }
                let mut bin = oracle.process_histogram(&hist);
                recent.push_back(hist);
                if recent.len() > config.window_size {
                    recent.pop_front();
                }
                if relock > 0 && h % CHECK == 0 {
                    let vote =
                        prepare(&day, &masked).and_then(|h| alias_vote(&h, &template, bin, margin));
                    searches.push_back(vote);
                    if searches.len() > relock {
                        searches.pop_front();
                    }
                    if searches.len() == relock && searches.iter().all(Option::is_some) {
                        let mut sorted: Vec<f64> = searches.iter().flatten().copied().collect();
                        sorted.sort_by(f64::total_cmp);
                        let median = sorted[relock / 2];
                        if sorted.iter().all(|g| (g - median).abs() <= agree) {
                            relocks += 1;
                            if env::var_os("RELOCK_LOG").is_some() {
                                let usd = |b: f64| 10f64.powf(10.0 - b / 200.0);
                                eprintln!(
                                    "{spec} relock h={h} oracle ${:.2} -> ${:.2}, exchange ${:.2}",
                                    usd(bin),
                                    usd(median),
                                    prices[h]
                                );
                            }
                            reseed(&mut oracle, &recent, median);
                            bin = median;
                        }
                    }
                }
                let anchor = cents_to_bin(prices[h] * 100.0);
                if let Some(guard) = guard
                    && prices[h] > 0.0
                    && ((anchor - bin) / 200.0 * std::f64::consts::LN_10).abs() > guard
                {
                    reseeds += 1;
                    reseed(&mut oracle, &recent, anchor);
                    bin = anchor;
                }
                path.extend_from_slice(&bin.to_le_bytes());
                let price = f64::from(bin_to_cents(bin)) / 100.0;
                if prices[h] > 0.0 {
                    errors[h / BUCKET].push((price / prices[h]).ln().abs());
                }
                if let Some((bands, timestamps)) = &bands {
                    let (low, high) = (bands[2 * h], bands[2 * h + 1]);
                    if low > 0.0 {
                        band_errors
                            .entry(year(timestamps[h]))
                            .or_default()
                            .push((low / price).ln().max((price / high).ln()).max(0.0));
                    } else {
                        uncovered += 1;
                    }
                }
            }
            fs::write(dir.join(format!("path_{spec}_{start}.f64")), path).unwrap();
            let mut report = format!("{spec} from {start}: {reseeds} reseeds, {relocks} relocks\n");
            let mut all = Vec::new();
            for (i, e) in errors.iter_mut().enumerate().filter(|(_, e)| !e.is_empty()) {
                all.extend_from_slice(e);
                report += &format!("  {:>9}  {}\n", i * BUCKET, stats(e));
            }
            report += &format!("  {:>9}  {}\n", "all", stats(&mut all));
            if bands.is_some() {
                let covered: usize = band_errors.values().map(Vec::len).sum();
                report +=
                    &format!("  vs exchange range: {covered} blocks covered, {uncovered} not\n");
                let mut all = Vec::new();
                for (year, e) in band_errors.iter_mut() {
                    all.extend_from_slice(e);
                    report += &format!("  {year:>9}  {}\n", band_stats(e));
                }
                report += &format!("  {:>9}  {}\n", "all", band_stats(&mut all));
            }
            report
        })
        .collect();
    for report in reports {
        println!("{report}");
    }
}

/// The calendar year (UTC) of a unix time.
fn year(unix: u32) -> i32 {
    // Howard Hinnant's days-to-civil, years starting in March.
    let z = unix as i64 / 86_400 + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z.rem_euclid(146_097);
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let month = (5 * doy + 2) / 153;
    (yoe + era * 400 + i64::from(month >= 10)) as i32
}

/// Distances from the exchange range, as upstream's report gives them: blocks, median, p95, p99,
/// p99.9, RMSE and max in percent, and the blocks more than 5%, 10% and 20% off.
fn band_stats(e: &mut [f64]) -> String {
    e.sort_by(f64::total_cmp);
    let pct = |x: f64| (x.exp() - 1.0) * 100.0;
    let q = |p: f64| pct(e[((e.len() - 1) as f64 * p).round() as usize]);
    let rmse = pct((e.iter().map(|x| x * x).sum::<f64>() / e.len() as f64).sqrt());
    let over = |x: f64| e.iter().filter(|&&v| v > (1.0 + x).ln()).count();
    let (over5, over10, over20) = (over(0.05), over(0.1), over(0.2));
    format!(
        "{:>7} blocks  median {:5.2}%  p95 {:5.2}%  p99 {:6.2}%  p99.9 {:6.2}%  rmse {:5.2}%  max {:6.2}%  >5% {over5:>6} ({:4.2}%)  >10% {over10:>6} ({:4.2}%)  >20% {over20:>5}",
        e.len(),
        q(0.5),
        q(0.95),
        q(0.99),
        q(0.999),
        rmse,
        pct(e[e.len() - 1]),
        over5 as f64 / e.len() as f64 * 100.0,
        over10 as f64 / e.len() as f64 * 100.0,
    )
}

/// Median, p90 and p99 distance in percent, and the share of blocks more than 10% and 30% off.
fn stats(e: &mut [f64]) -> String {
    e.sort_by(f64::total_cmp);
    let q = |p: f64| ((e[((e.len() - 1) as f64 * p) as usize]).exp() - 1.0) * 100.0;
    let over =
        |x: f64| e.iter().filter(|&&v| v > (1.0 + x).ln()).count() as f64 / e.len() as f64 * 100.0;
    format!(
        "median {:6.2}%  p90 {:7.2}%  p99 {:7.2}%  >10% {:5.1}%  >30% {:5.1}%",
        q(0.5),
        q(0.9),
        q(0.99),
        over(0.1),
        over(0.3)
    )
}
