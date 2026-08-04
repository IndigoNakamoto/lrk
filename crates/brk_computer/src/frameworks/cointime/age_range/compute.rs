use brk_cohort::{AGE_RANGE_BOUNDS, AGE_RANGE_COUNT};
use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::{Bitcoin, Height, ONE_DAY_IN_SEC_F64, Sats, StoredF64, Timestamp, Version};
use vecdb::{AnyVec, Exit, ReadableVec};

use super::super::activity;
use super::{CohortVecs, SupplyVecs, Vecs};
use crate::{distribution, indexes};

const HOURS_PER_DAY: f64 = 24.0;
const WRITE_INTERVAL: usize = 10_000;

impl Vecs {
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        indexes: &indexes::Vecs,
        distribution: &distribution::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        let starting_height = indexer.safe_lengths().height;
        let source_cohorts: Vec<_> = distribution.utxo_cohorts.age_range.iter().collect();
        let supplies: Vec<_> = source_cohorts
            .iter()
            .map(|cohort| &cohort.metrics.supply.total.sats.height)
            .collect();
        let transfer_volumes: Vec<_> = source_cohorts
            .iter()
            .map(|cohort| &cohort.metrics.activity.transfer_volume.block.sats)
            .collect();
        let coindays_destroyed: Vec<_> = source_cohorts
            .iter()
            .map(|cohort| &cohort.metrics.activity.coindays_destroyed.block)
            .collect();

        self.compute_created(
            starting_height,
            &indexes.timestamp.monotonic,
            &supplies,
            exit,
        )?;
        self.compute_consumed(
            starting_height,
            &transfer_volumes,
            &coindays_destroyed,
            exit,
        )?;
        self.compute_rest(starting_height, &supplies, exit)
    }

    fn compute_created<T, S>(
        &mut self,
        starting_height: Height,
        timestamps: &T,
        supplies: &[&S],
        exit: &Exit,
    ) -> Result<()>
    where
        T: ReadableVec<Height, Timestamp>,
        S: ReadableVec<Height, Sats>,
    {
        debug_assert_eq!(supplies.len(), AGE_RANGE_COUNT);

        let created_version: Version = std::iter::once(timestamps.version())
            .chain(supplies.iter().map(|vec| vec.version()))
            .sum();
        let mut cohorts: Vec<&mut CohortVecs> = self.iter_mut().collect();

        for cohort in cohorts.iter_mut() {
            cohort
                .coindays_created
                .validate_computed_version_or_reset(created_version)?;
        }

        let start = cohorts
            .iter()
            .map(|cohort| cohort.coindays_created.cumulative.height.len())
            .min()
            .unwrap_or_default()
            .min(usize::from(starting_height));

        for cohort in cohorts.iter_mut() {
            cohort.coindays_created.truncate_if_needed_at(start)?;
        }

        let source_end = supplies
            .iter()
            .map(|vec| vec.len())
            .chain(std::iter::once(timestamps.len()))
            .min()
            .unwrap_or_default();

        let mut chunk_start = start;
        while chunk_start < source_end {
            let chunk_end = (chunk_start + WRITE_INTERVAL).min(source_end);
            let timestamp_start = chunk_start.saturating_sub(1);
            let timestamp_batch = timestamps.collect_range_at(timestamp_start, chunk_end);
            let supply_batches: Vec<_> = supplies
                .iter()
                .map(|vec| vec.collect_range_at(chunk_start, chunk_end))
                .collect();

            for (offset, _) in supply_batches.first().unwrap().iter().enumerate() {
                let interval_seconds =
                    monotonic_interval_seconds(&timestamp_batch, chunk_start, offset);

                for (index, cohort) in cohorts.iter_mut().enumerate() {
                    cohort.coindays_created.push_block(coindays_created(
                        supply_batches[index][offset],
                        interval_seconds,
                    ));
                }
            }

            {
                let _lock = exit.lock();
                for cohort in cohorts.iter_mut() {
                    cohort.coindays_created.write()?;
                }
            }
            chunk_start = chunk_end;
        }

        Ok(())
    }

    fn compute_consumed<V, D>(
        &mut self,
        starting_height: Height,
        transfer_volumes: &[&V],
        source_coindays_destroyed: &[&D],
        exit: &Exit,
    ) -> Result<()>
    where
        V: ReadableVec<Height, Sats>,
        D: ReadableVec<Height, StoredF64>,
    {
        debug_assert_eq!(transfer_volumes.len(), AGE_RANGE_COUNT);
        debug_assert_eq!(source_coindays_destroyed.len(), AGE_RANGE_COUNT);

        let destroyed_version: Version = transfer_volumes
            .iter()
            .map(|vec| vec.version())
            .chain(source_coindays_destroyed.iter().map(|vec| vec.version()))
            .sum();
        let mut cohorts: Vec<&mut CohortVecs> = self.iter_mut().collect();

        for cohort in cohorts.iter_mut() {
            cohort
                .coindays_consumed
                .validate_computed_version_or_reset(destroyed_version)?;
        }

        let start = cohorts
            .iter()
            .map(|cohort| cohort.coindays_consumed.cumulative.height.len())
            .min()
            .unwrap_or_default()
            .min(usize::from(starting_height));

        for cohort in cohorts.iter_mut() {
            cohort.coindays_consumed.truncate_if_needed_at(start)?;
        }

        let source_end = transfer_volumes
            .iter()
            .map(|vec| vec.len())
            .chain(source_coindays_destroyed.iter().map(|vec| vec.len()))
            .min()
            .unwrap_or_default();
        let bounds = age_bounds_days();

        let mut chunk_start = start;
        while chunk_start < source_end {
            let chunk_end = (chunk_start + WRITE_INTERVAL).min(source_end);
            let transfer_batches: Vec<_> = transfer_volumes
                .iter()
                .map(|vec| vec.collect_range_at(chunk_start, chunk_end))
                .collect();
            let destroyed_batches: Vec<_> = source_coindays_destroyed
                .iter()
                .map(|vec| vec.collect_range_at(chunk_start, chunk_end))
                .collect();

            for offset in 0..(chunk_end - chunk_start) {
                let volumes_btc: [f64; AGE_RANGE_COUNT] = std::array::from_fn(|index| {
                    f64::from(Bitcoin::from(transfer_batches[index][offset]))
                });
                let cdd: [f64; AGE_RANGE_COUNT] =
                    std::array::from_fn(|index| f64::from(destroyed_batches[index][offset]));
                let consumed = allocate_consumed_coindays(volumes_btc, cdd, &bounds);

                for (index, cohort) in cohorts.iter_mut().enumerate() {
                    cohort
                        .coindays_consumed
                        .push_block(StoredF64::from(consumed[index]));
                }
            }

            {
                let _lock = exit.lock();
                for cohort in cohorts.iter_mut() {
                    cohort.coindays_consumed.write()?;
                }
            }
            chunk_start = chunk_end;
        }

        Ok(())
    }

    fn compute_rest<S>(
        &mut self,
        starting_height: Height,
        supplies: &[&S],
        exit: &Exit,
    ) -> Result<()>
    where
        S: ReadableVec<Height, Sats>,
    {
        debug_assert_eq!(supplies.len(), AGE_RANGE_COUNT);

        for (cohort, &total_supply) in self.iter_mut().zip(supplies) {
            let CohortVecs {
                coindays_created,
                coindays_consumed,
                coindays_stored,
                activity: activity_vecs,
                supply,
            } = cohort;

            activity::compute_rest(
                starting_height,
                coindays_created,
                coindays_consumed,
                coindays_stored,
                &mut activity_vecs.wakefulness,
                exit,
            )?;

            supply.compute_from(
                starting_height,
                total_supply,
                &activity_vecs.wakefulness.height,
                &activity_vecs.dormancy.height,
                exit,
            )?;
        }

        Ok(())
    }
}

impl SupplyVecs {
    fn compute_from(
        &mut self,
        starting_height: Height,
        total_supply: &impl ReadableVec<Height, Sats>,
        wakefulness: &impl ReadableVec<Height, StoredF64>,
        dormancy: &impl ReadableVec<Height, StoredF64>,
        exit: &Exit,
    ) -> Result<()> {
        self.awake.sats.height.compute_multiply(
            starting_height,
            total_supply,
            wakefulness,
            exit,
        )?;
        self.dormant
            .sats
            .height
            .compute_multiply(starting_height, total_supply, dormancy, exit)?;

        Ok(())
    }
}

#[inline(always)]
fn monotonic_interval_seconds(
    timestamp_batch: &[Timestamp],
    chunk_start: usize,
    offset: usize,
) -> u32 {
    if chunk_start + offset == 0 {
        return 0;
    }

    let current_index = offset + usize::from(chunk_start > 0);
    (*timestamp_batch[current_index]).saturating_sub(*timestamp_batch[current_index - 1])
}

#[inline(always)]
fn coindays_created(supply: Sats, interval_seconds: u32) -> StoredF64 {
    StoredF64::from(f64::from(Bitcoin::from(supply)) * interval_seconds as f64 / ONE_DAY_IN_SEC_F64)
}

fn age_bounds_days() -> [(f64, f64); AGE_RANGE_COUNT] {
    let mut bounds = AGE_RANGE_BOUNDS.iter();
    std::array::from_fn(|index| {
        let bound = bounds.next().unwrap();
        let lower = bound.start as f64 / HOURS_PER_DAY;
        let width = if index + 1 < AGE_RANGE_COUNT {
            (bound.end - bound.start) as f64 / HOURS_PER_DAY
        } else {
            0.0
        };
        (lower, width)
    })
}

fn allocate_consumed_coindays(
    transfer_volume_btc: [f64; AGE_RANGE_COUNT],
    coindays_destroyed: [f64; AGE_RANGE_COUNT],
    bounds: &[(f64, f64); AGE_RANGE_COUNT],
) -> [f64; AGE_RANGE_COUNT] {
    let mut result = [0.0; AGE_RANGE_COUNT];
    let mut older_transfer_volume = 0.0;

    for index in (0..AGE_RANGE_COUNT).rev() {
        let (lower_days, width_days) = bounds[index];
        let within_cohort =
            (coindays_destroyed[index] - transfer_volume_btc[index] * lower_days).max(0.0);
        result[index] = within_cohort + older_transfer_volume * width_days;
        older_transfer_volume += transfer_volume_btc[index];
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn created_coindays_use_the_exact_monotonic_block_interval() {
        let created = f64::from(coindays_created(Sats::ONE_BTC, 6 * 60 * 60));

        assert!((created - 0.25).abs() < 1e-12);
    }

    #[test]
    fn monotonic_interval_handles_initial_and_resumed_chunks() {
        let initial = [
            Timestamp::from(100_u32),
            Timestamp::from(160_u32),
            Timestamp::from(220_u32),
        ];
        let resumed = [Timestamp::from(160_u32), Timestamp::from(220_u32)];

        assert_eq!(monotonic_interval_seconds(&initial, 0, 0), 0);
        assert_eq!(monotonic_interval_seconds(&initial, 0, 1), 60);
        assert_eq!(monotonic_interval_seconds(&resumed, 2, 0), 60);
    }

    #[test]
    fn destruction_at_a_boundary_stays_in_the_ranges_already_traversed() {
        let mut volumes = [0.0; AGE_RANGE_COUNT];
        let mut cdd = [0.0; AGE_RANGE_COUNT];
        volumes[2] = 1.0;
        cdd[2] = 1.0;

        let allocated = allocate_consumed_coindays(volumes, cdd, &age_bounds_days());

        assert!((allocated[0] - 1.0 / HOURS_PER_DAY).abs() < 1e-12);
        assert!((allocated[1] - 23.0 / HOURS_PER_DAY).abs() < 1e-12);
        assert!(allocated[2].abs() < 1e-12);
        assert!((allocated.iter().sum::<f64>() - 1.0).abs() < 1e-12);
    }

    #[test]
    fn consumed_coindays_cover_every_traversed_cohort() {
        let mut volumes = [0.0; AGE_RANGE_COUNT];
        let mut cdd = [0.0; AGE_RANGE_COUNT];
        volumes[2] = 2.0;
        cdd[2] = 20.0;

        let allocated = allocate_consumed_coindays(volumes, cdd, &age_bounds_days());

        assert!((allocated[0] - 2.0 / HOURS_PER_DAY).abs() < 1e-12);
        assert!((allocated[1] - 46.0 / HOURS_PER_DAY).abs() < 1e-12);
        assert!((allocated[2] - 18.0).abs() < 1e-12);
        assert!((allocated.iter().sum::<f64>() - 20.0).abs() < 1e-12);
    }

    #[test]
    fn allocated_coindays_conserve_mixed_cohort_destruction() {
        let bounds = age_bounds_days();
        let mut volumes = [0.0; AGE_RANGE_COUNT];
        let mut cdd = [0.0; AGE_RANGE_COUNT];

        for index in [0, 1, 2, 10, 20, AGE_RANGE_COUNT - 2, AGE_RANGE_COUNT - 1] {
            let (lower, width) = bounds[index];
            let volume = index as f64 + 1.0;
            let age = lower + if width > 0.0 { width / 2.0 } else { 30.0 };
            volumes[index] = volume;
            cdd[index] = volume * age;
        }

        let allocated = allocate_consumed_coindays(volumes, cdd, &bounds);

        assert!((allocated.iter().sum::<f64>() - cdd.iter().sum::<f64>()).abs() < 1e-9);
    }

    #[test]
    fn bounds_and_allocation_cover_every_canonical_age_range() {
        let bounds = age_bounds_days();
        let mut volumes = [0.0; AGE_RANGE_COUNT];
        let mut cdd = [0.0; AGE_RANGE_COUNT];
        let last = AGE_RANGE_COUNT - 1;

        volumes[last] = 1.0;
        cdd[last] = bounds[last].0 + 30.0;

        let allocated = allocate_consumed_coindays(volumes, cdd, &bounds);

        assert_eq!(bounds.len(), AGE_RANGE_BOUNDS.iter().count());
        assert!(allocated[last] > 0.0);
        assert!((allocated.iter().sum::<f64>() - cdd.iter().sum::<f64>()).abs() < 1e-9);
    }
}
