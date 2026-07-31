use brk_error::Result;
use brk_indexer::{Indexer, Lengths};
use brk_types::{Bitcoin, Cents, Height, Sats, StoredF64, Timestamp, Version};
use vecdb::{AnyStoredVec, Exit, ReadableVec, WritableVec};

use super::super::cointime;
use super::{
    AGE_COHORT_COUNT, AgeBand, CohortVecs, HORIZON_COUNT, Horizons, MINIMUM_DURATION_DAYS, Vecs,
    age_bounds_days, horizon_mobility, mobility,
};
use crate::{
    distribution, frameworks::WeightedRatio, indexes,
    internal::db_utils::validate_any_computed_version_or_reset, price,
};

const WRITE_INTERVAL: usize = 10_000;

#[derive(Clone, Copy)]
struct DecayFit {
    slope: f64,
    tau: f64,
    anchor_age: f64,
    anchor_hazard: f64,
}

impl Vecs {
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        indexes: &indexes::Vecs,
        _prices: &price::Vecs,
        distribution: &distribution::Vecs,
        cointime: &cointime::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        self.db.sync_bg_tasks()?;

        let starting_lengths = indexer.safe_lengths();
        let source_cohorts: Vec<_> = distribution.utxo_cohorts.age_range.iter().collect();
        let transfer_volumes: Vec<_> = source_cohorts
            .iter()
            .map(|cohort| {
                &cohort
                    .metrics
                    .activity
                    .transfer_volume
                    .cumulative
                    .sats
                    .height
            })
            .collect();
        let supplies: Vec<_> = source_cohorts
            .iter()
            .map(|cohort| &cohort.metrics.supply.total.sats.height)
            .collect();
        let loss_supplies: Vec<_> = source_cohorts
            .iter()
            .map(|cohort| &cohort.metrics.supply.in_loss.sats.height)
            .collect();
        let realized_caps: Vec<_> = source_cohorts
            .iter()
            .map(|cohort| &cohort.metrics.realized.cap.cents.height)
            .collect();
        let coindays_created: Vec<_> = cointime
            .age_range
            .iter()
            .map(|cohort| &cohort.coindays_created.cumulative.height)
            .collect();

        self.compute_primary(
            &starting_lengths,
            &indexes.timestamp.monotonic,
            &transfer_volumes,
            &coindays_created,
            &supplies,
            &loss_supplies,
            &realized_caps,
            exit,
        )?;

        let exit = exit.clone();
        self.db.run_bg(move |db| {
            let _lock = exit.lock();
            db.compact_deferred_default()
        });
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn compute_primary<T, D, S, C>(
        &mut self,
        starting_lengths: &Lengths,
        timestamps: &T,
        transfer_volumes: &[&S],
        coindays_created: &[&D],
        supplies: &[&S],
        loss_supplies: &[&S],
        realized_caps: &[&C],
        exit: &Exit,
    ) -> Result<Height>
    where
        T: ReadableVec<Height, Timestamp>,
        D: ReadableVec<Height, StoredF64>,
        S: ReadableVec<Height, Sats>,
        C: ReadableVec<Height, Cents>,
    {
        debug_assert_eq!(transfer_volumes.len(), AGE_COHORT_COUNT);
        debug_assert_eq!(coindays_created.len(), AGE_COHORT_COUNT);
        debug_assert_eq!(supplies.len(), AGE_COHORT_COUNT);
        debug_assert_eq!(loss_supplies.len(), AGE_COHORT_COUNT);
        debug_assert_eq!(realized_caps.len(), AGE_COHORT_COUNT);

        let source_version: Version = std::iter::once(timestamps.version())
            .chain(transfer_volumes.iter().map(|vec| vec.version()))
            .chain(coindays_created.iter().map(|vec| vec.version()))
            .chain(supplies.iter().map(|vec| vec.version()))
            .chain(loss_supplies.iter().map(|vec| vec.version()))
            .chain(realized_caps.iter().map(|vec| vec.version()))
            .sum();

        for vec in self.primary_vecs_mut() {
            validate_any_computed_version_or_reset(vec, source_version)?;
        }

        let start = self
            .primary_vecs_mut()
            .into_iter()
            .map(|vec| vec.len())
            .min()
            .unwrap_or_default()
            .min(usize::from(starting_lengths.height));

        for vec in self.primary_vecs_mut() {
            vec.any_truncate_if_needed_at(start)?;
        }

        let source_end = transfer_volumes
            .iter()
            .map(|vec| vec.len())
            .chain(coindays_created.iter().map(|vec| vec.len()))
            .chain(supplies.iter().map(|vec| vec.len()))
            .chain(loss_supplies.iter().map(|vec| vec.len()))
            .chain(realized_caps.iter().map(|vec| vec.len()))
            .chain(std::iter::once(timestamps.len()))
            .min()
            .unwrap_or_default();

        if source_end == 0 {
            return Ok(Height::ZERO);
        }

        let genesis_timestamp = timestamps
            .collect_one(Height::ZERO)
            .unwrap_or(Timestamp::ZERO);
        let bounds = age_bounds_days();

        let mut chunk_start = start;
        while chunk_start < source_end {
            let chunk_end = (chunk_start + WRITE_INTERVAL).min(source_end);
            let timestamp_batch = timestamps.collect_range_at(chunk_start, chunk_end);
            let transfer_batches: Vec<_> = transfer_volumes
                .iter()
                .map(|vec| vec.collect_range_at(chunk_start, chunk_end))
                .collect();
            let coinday_batches: Vec<_> = coindays_created
                .iter()
                .map(|vec| vec.collect_range_at(chunk_start, chunk_end))
                .collect();
            let supply_batches: Vec<_> = supplies
                .iter()
                .map(|vec| vec.collect_range_at(chunk_start, chunk_end))
                .collect();
            let loss_supply_batches: Vec<_> = loss_supplies
                .iter()
                .map(|vec| vec.collect_range_at(chunk_start, chunk_end))
                .collect();
            let cap_batches: Vec<_> = realized_caps
                .iter()
                .map(|vec| vec.collect_range_at(chunk_start, chunk_end))
                .collect();

            for offset in 0..(chunk_end - chunk_start) {
                let hazards = std::array::from_fn(|index| {
                    spending_rate(
                        transfer_batches[index][offset],
                        coinday_batches[index][offset],
                    )
                });
                let network_age = timestamp_batch[offset]
                    .difference_in_days_between_float(genesis_timestamp)
                    .max(MINIMUM_DURATION_DAYS);
                let exposures = spending_exposures(&hazards, network_age, &bounds);
                let mobilities = exposures.map(mobility);
                let horizon_mobilities: Horizons<[f64; AGE_COHORT_COUNT]> =
                    Horizons::from_fn(|_, horizon| {
                        std::array::from_fn(|age| horizon_mobility(&hazards, age, horizon, &bounds))
                    });

                let mut mobile_supply = Sats::ZERO;
                let mut immobile_supply = Sats::ZERO;
                let mut coinflow_cap = Cents::ZERO;
                let mut supply_in_loss = WeightedRatio::default();
                let mut horizon_supply_in_loss = Horizons::from_fn(|_, _| WeightedRatio::default());

                for (index, cohort) in self.age_range.iter_mut().enumerate() {
                    let mobility = StoredF64::from(mobilities[index]);
                    let total_supply = supply_batches[index][offset];
                    let cohort_mobile_supply = total_supply * mobility;
                    let cohort_immobile_supply = total_supply - cohort_mobile_supply;

                    let total_cap = cap_batches[index][offset];
                    let cohort_mobile_cap = total_cap * mobility;
                    let total = total_supply.as_u128() as f64;
                    let loss = loss_supply_batches[index][offset].as_u128() as f64;
                    supply_in_loss.add(loss, total, mobilities[index]);
                    for (ratio, weights) in horizon_supply_in_loss
                        .iter_mut()
                        .zip(horizon_mobilities.iter())
                    {
                        ratio.add(loss, total, weights[index]);
                    }

                    cohort
                        .spending_rate
                        .height
                        .push(StoredF64::from(hazards[index]));
                    cohort
                        .spending_exposure
                        .height
                        .push(StoredF64::from(exposures[index]));
                    cohort.supply.mobile.sats.height.push(cohort_mobile_supply);
                    cohort
                        .supply
                        .immobile
                        .sats
                        .height
                        .push(cohort_immobile_supply);

                    mobile_supply += cohort_mobile_supply;
                    immobile_supply += cohort_immobile_supply;
                    coinflow_cap += cohort_mobile_cap;
                }

                self.supply.mobile.sats.height.push(mobile_supply);
                self.supply.immobile.sats.height.push(immobile_supply);
                self.supply_in_loss_share
                    .height
                    .push(supply_in_loss.value());
                for (output, ratio) in self.horizon.iter_mut().zip(horizon_supply_in_loss.iter()) {
                    output.supply_in_loss_share.height.push(ratio.value());
                }
                self.cap.cents.height.push(coinflow_cap);
                self.price
                    .cents
                    .height
                    .push(realized_price(coinflow_cap, mobile_supply));
            }

            {
                let _lock = exit.lock();
                for vec in self.primary_vecs_mut() {
                    vec.write()?;
                }
            }
            chunk_start = chunk_end;
        }

        Ok(Height::from(start))
    }

    fn primary_vecs_mut(&mut self) -> Vec<&mut dyn AnyStoredVec> {
        let mut vecs = Vec::with_capacity(AGE_COHORT_COUNT * 4 + 5 + HORIZON_COUNT);

        for cohort in self.age_range.iter_mut() {
            vecs.extend(cohort.primary_vecs_mut());
        }

        vecs.extend([
            &mut self.supply.mobile.sats.height as &mut dyn AnyStoredVec,
            &mut self.supply.immobile.sats.height,
            &mut self.supply_in_loss_share.height,
            &mut self.cap.cents.height,
            &mut self.price.cents.height,
        ]);
        vecs.extend(
            self.horizon
                .iter_mut()
                .map(|horizon| &mut horizon.supply_in_loss_share.height as &mut dyn AnyStoredVec),
        );
        vecs
    }
}

impl CohortVecs {
    fn primary_vecs_mut(&mut self) -> [&mut dyn AnyStoredVec; 4] {
        [
            &mut self.spending_rate.height,
            &mut self.spending_exposure.height,
            &mut self.supply.mobile.sats.height,
            &mut self.supply.immobile.sats.height,
        ]
    }
}

#[inline]
fn spending_rate(transfer_volume: Sats, coindays_created: StoredF64) -> f64 {
    let exposure = f64::from(coindays_created);
    if exposure > 0.0 {
        (f64::from(Bitcoin::from(transfer_volume)) / exposure).max(0.0)
    } else {
        0.0
    }
}

fn fit_decay(
    hazards: &[f64; AGE_COHORT_COUNT],
    network_age: f64,
    bounds: &[AgeBand; AGE_COHORT_COUNT],
) -> Option<DecayFit> {
    let mut total_duration = 0.0;
    let mut weighted_age = 0.0;
    let mut weighted_log_hazard = 0.0;
    let mut anchor = None;

    for (index, band) in bounds[..AGE_COHORT_COUNT - 1].iter().enumerate() {
        let hazard = hazards[index];
        if band.upper > network_age || !hazard.is_finite() || hazard <= 0.0 {
            continue;
        }

        let age = (band.lower + band.upper) / 2.0;
        let duration = band.upper - band.lower;
        let log_hazard = hazard.ln();
        total_duration += duration;
        weighted_age += duration * age;
        weighted_log_hazard += duration * log_hazard;
        anchor = Some((band.upper, hazard));
    }

    if total_duration <= 0.0 {
        return None;
    }

    let mean_age = weighted_age / total_duration;
    let mean_log_hazard = weighted_log_hazard / total_duration;
    let mut covariance = 0.0;
    let mut age_variance = 0.0;

    for (index, band) in bounds[..AGE_COHORT_COUNT - 1].iter().enumerate() {
        let hazard = hazards[index];
        if band.upper > network_age || !hazard.is_finite() || hazard <= 0.0 {
            continue;
        }

        let age = (band.lower + band.upper) / 2.0;
        let duration = band.upper - band.lower;
        let log_hazard = hazard.ln();
        let age_offset = age - mean_age;
        covariance += duration * age_offset * (log_hazard - mean_log_hazard);
        age_variance += duration * age_offset.powi(2);
    }

    if age_variance <= f64::EPSILON {
        return None;
    }

    let slope = covariance / age_variance;
    if slope >= 0.0 {
        return None;
    }

    let tau = -1.0 / slope;
    if !tau.is_finite() || tau <= 0.0 {
        return None;
    }

    let (anchor_age, anchor_hazard) = anchor?;
    Some(DecayFit {
        slope,
        tau,
        anchor_age,
        anchor_hazard,
    })
}

fn spending_exposures(
    hazards: &[f64; AGE_COHORT_COUNT],
    network_age: f64,
    bounds: &[AgeBand; AGE_COHORT_COUNT],
) -> [f64; AGE_COHORT_COUNT] {
    let Some(fit) = fit_decay(hazards, network_age, bounds) else {
        return [0.0; AGE_COHORT_COUNT];
    };

    std::array::from_fn(|start_band| {
        spending_exposure(hazards, start_band, network_age, bounds, fit)
    })
}

fn spending_exposure(
    hazards: &[f64; AGE_COHORT_COUNT],
    start_band: usize,
    network_age: f64,
    bounds: &[AgeBand; AGE_COHORT_COUNT],
    fit: DecayFit,
) -> f64 {
    let start = bounds[start_band];
    let occupied_upper = if start.upper.is_finite() {
        start.upper.min(network_age.max(start.lower))
    } else {
        start.lower
    };
    let mut age = if start.upper.is_finite() {
        (start.lower + occupied_upper) / 2.0
    } else {
        start.lower
    };
    let mut exposure = 0.0;

    for band_index in start_band..AGE_COHORT_COUNT - 1 {
        let band = bounds[band_index];
        let duration = (band.upper - age.max(band.lower)).max(MINIMUM_DURATION_DAYS);
        let hazard = hazards[band_index];
        let observed = band.upper <= network_age && hazard.is_finite() && hazard > 0.0;
        if !observed {
            break;
        }

        exposure += hazard * duration;
        age = band.upper;
    }

    let tail = bounds[AGE_COHORT_COUNT - 1];
    let tail_hazard = hazards[AGE_COHORT_COUNT - 1];
    let observed_tail = network_age > tail.lower && tail_hazard.is_finite() && tail_hazard > 0.0;
    let (anchor_age, anchor_hazard) = if observed_tail {
        (tail.lower, tail_hazard)
    } else {
        (fit.anchor_age, fit.anchor_hazard)
    };
    let continuation_age = age.max(anchor_age);
    let continuation_hazard = anchor_hazard * (fit.slope * (continuation_age - anchor_age)).exp();
    exposure + (continuation_hazard * fit.tau).max(0.0)
}

#[inline]
fn realized_price(cap: Cents, supply: Sats) -> Cents {
    (cap.as_u128() * Sats::ONE_BTC_U128)
        .checked_div(supply.as_u128())
        .map(Cents::from)
        .unwrap_or(Cents::ZERO)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mobility_is_the_complement_of_survival() {
        assert_eq!(mobility(0.0), 0.0);
        assert!((mobility(2.0_f64.ln()) - 0.5).abs() < 1e-12);
        assert!((mobility(1e-15) - 1e-15).abs() < 1e-27);
        assert!(mobility(1_000.0) < 1.0);
        assert_eq!(mobility(f64::INFINITY), 1.0 - 1e-12);
        assert_eq!(mobility(f64::NAN), 0.0);
    }

    #[test]
    fn fixed_horizon_compounds_hazards_across_age_ranges() {
        let bounds = age_bounds_days();
        let hazards = [0.01; AGE_COHORT_COUNT];
        let probability = horizon_mobility(&hazards, 2, 30.0, &bounds);

        assert!((probability - mobility(0.3)).abs() < 1e-12);
    }

    #[test]
    fn decay_fit_recovers_an_exponential_lifetime() {
        let bounds = age_bounds_days();
        let expected_tau = 1_000.0;
        let hazards = std::array::from_fn(|index| {
            let band = bounds[index];
            let age = if band.upper.is_finite() {
                (band.lower + band.upper) / 2.0
            } else {
                band.lower
            };
            (-age / expected_tau).exp()
        });

        let fit = fit_decay(&hazards, 20.0 * 365.0, &bounds).unwrap();

        assert!((fit.tau - expected_tau).abs() < 1e-9);
    }

    #[test]
    fn oldest_cohort_exposure_is_its_observed_tail_lifetime() {
        let bounds = age_bounds_days();
        let hazards = std::array::from_fn(|index| {
            let band = bounds[index];
            let age = if band.upper.is_finite() {
                (band.lower + band.upper) / 2.0
            } else {
                band.lower
            };
            (-age / 1_000.0).exp()
        });
        let network_age = 20.0 * 365.0;
        let fit = fit_decay(&hazards, network_age, &bounds).unwrap();
        let exposures = spending_exposures(&hazards, network_age, &bounds);

        assert!(
            (exposures[AGE_COHORT_COUNT - 1] - hazards[AGE_COHORT_COUNT - 1] * fit.tau).abs()
                < 1e-12
        );
    }

    #[test]
    fn supply_partitions_and_coinflow_cap_is_bounded() {
        let total_supply = Sats::from(123_456_789_u64);
        let total_cap = Cents::from(987_654_321_u64);
        let mobility = StoredF64::from(0.321);
        let mobile_supply = total_supply * mobility;
        let coinflow_cap = total_cap * mobility;

        assert_eq!(mobile_supply + (total_supply - mobile_supply), total_supply);
        assert!(coinflow_cap <= total_cap);
    }
}
