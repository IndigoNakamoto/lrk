use brk_cohort::{AGE_RANGE_COUNT, AGE_RANGE_FILTERS, ByTerm, TERM_FILTERS};
use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::{Cents, Height, Sats, StoredF64, Version};
use vecdb::{AnyStoredVec, AnyVec, Exit, ReadableVec, WritableVec};

use super::{AllCohortVecs, StoredCohortVecs, Vecs};
use crate::{
    distribution,
    frameworks::{WeightedCohortState, realized_price},
    internal::{PerBlock, db_utils::validate_any_computed_version_or_reset},
};

const WRITE_INTERVAL: usize = 10_000;
const ALL_PRIMARY_VEC_COUNT: usize = 4;
const STORED_PRIMARY_VEC_COUNT: usize = 5;

impl Vecs {
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        distribution: &distribution::Vecs,
        age_range: &super::super::AgeRangeVecs,
        all_supply_in_loss_share: &mut PerBlock<StoredF64>,
        exit: &Exit,
    ) -> Result<()> {
        let starting_height = indexer.safe_lengths().height;
        let source_cohorts: Vec<_> = distribution.utxo_cohorts.age_range.iter().collect();
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
        let weights: Vec<_> = age_range
            .iter()
            .map(|cohort| &cohort.wakefulness.height)
            .collect();

        self.compute_primary(
            starting_height,
            &supplies,
            &loss_supplies,
            &realized_caps,
            &weights,
            all_supply_in_loss_share,
            exit,
        )
    }

    #[allow(clippy::too_many_arguments)]
    fn compute_primary<S, C, W>(
        &mut self,
        starting_height: Height,
        supplies: &[&S],
        loss_supplies: &[&S],
        realized_caps: &[&C],
        weights: &[&W],
        all_supply_in_loss_share: &mut PerBlock<StoredF64>,
        exit: &Exit,
    ) -> Result<()>
    where
        S: ReadableVec<Height, Sats>,
        C: ReadableVec<Height, Cents>,
        W: ReadableVec<Height, StoredF64>,
    {
        debug_assert_eq!(supplies.len(), AGE_RANGE_COUNT);
        debug_assert_eq!(loss_supplies.len(), AGE_RANGE_COUNT);
        debug_assert_eq!(realized_caps.len(), AGE_RANGE_COUNT);
        debug_assert_eq!(weights.len(), AGE_RANGE_COUNT);

        let source_version: Version = supplies
            .iter()
            .map(|vec| vec.version())
            .chain(loss_supplies.iter().map(|vec| vec.version()))
            .chain(realized_caps.iter().map(|vec| vec.version()))
            .chain(weights.iter().map(|vec| vec.version()))
            .sum();

        for vec in self.primary_vecs_mut() {
            validate_any_computed_version_or_reset(vec, source_version)?;
        }
        let all_supply_in_loss_share = &mut all_supply_in_loss_share.height;
        validate_any_computed_version_or_reset(all_supply_in_loss_share, source_version)?;

        let start = self
            .primary_vecs_mut()
            .into_iter()
            .map(|vec| vec.len())
            .chain(std::iter::once(all_supply_in_loss_share.len()))
            .min()
            .unwrap_or_default()
            .min(usize::from(starting_height));

        for vec in self.primary_vecs_mut() {
            vec.any_truncate_if_needed_at(start)?;
        }
        all_supply_in_loss_share.truncate_if_needed_at(start)?;

        let source_end = supplies
            .iter()
            .map(|vec| vec.len())
            .chain(loss_supplies.iter().map(|vec| vec.len()))
            .chain(realized_caps.iter().map(|vec| vec.len()))
            .chain(weights.iter().map(|vec| vec.len()))
            .min()
            .unwrap_or_default();

        let mut age_filters = AGE_RANGE_FILTERS.iter();
        let is_sth: [bool; AGE_RANGE_COUNT] =
            std::array::from_fn(|_| TERM_FILTERS.short.includes(age_filters.next().unwrap()));

        let mut chunk_start = start;
        while chunk_start < source_end {
            let chunk_end = (chunk_start + WRITE_INTERVAL).min(source_end);
            let supply_batches: Vec<_> = supplies
                .iter()
                .map(|vec| vec.collect_range_at(chunk_start, chunk_end))
                .collect();
            let loss_batches: Vec<_> = loss_supplies
                .iter()
                .map(|vec| vec.collect_range_at(chunk_start, chunk_end))
                .collect();
            let cap_batches: Vec<_> = realized_caps
                .iter()
                .map(|vec| vec.collect_range_at(chunk_start, chunk_end))
                .collect();
            let weight_batches: Vec<_> = weights
                .iter()
                .map(|vec| vec.collect_range_at(chunk_start, chunk_end))
                .collect();

            for offset in 0..(chunk_end - chunk_start) {
                let mut terms = ByTerm::<WeightedCohortState>::default();
                for age in 0..AGE_RANGE_COUNT {
                    let term = if is_sth[age] {
                        &mut terms.short
                    } else {
                        &mut terms.long
                    };
                    term.add(
                        supply_batches[age][offset],
                        loss_batches[age][offset],
                        cap_batches[age][offset],
                        weight_batches[age][offset],
                    );
                }
                let all = terms.short.merged(terms.long);
                all_supply_in_loss_share.push(all.supply_in_loss.value());
                self.all.push(all);
                self.sth.push(terms.short);
                self.lth.push(terms.long);
            }

            {
                let _lock = exit.lock();
                for vec in self.primary_vecs_mut() {
                    vec.write()?;
                }
                all_supply_in_loss_share.write()?;
            }
            chunk_start = chunk_end;
        }

        Ok(())
    }

    fn primary_vecs_mut(&mut self) -> Vec<&mut dyn AnyStoredVec> {
        let mut vecs = Vec::with_capacity(ALL_PRIMARY_VEC_COUNT + 2 * STORED_PRIMARY_VEC_COUNT);
        vecs.extend(self.all.primary_vecs_mut());
        vecs.extend(self.sth.primary_vecs_mut());
        vecs.extend(self.lth.primary_vecs_mut());
        vecs
    }
}

impl AllCohortVecs {
    fn push(&mut self, state: WeightedCohortState) {
        self.awake.supply.sats.height.push(state.weighted_supply);
        self.dormant
            .supply
            .sats
            .height
            .push(state.complement_supply);
        self.awake.cap.cents.height.push(state.weighted_cap);
        self.awake
            .price
            .cents
            .height
            .push(realized_price(state.weighted_cap, state.weighted_supply));
    }

    fn primary_vecs_mut(&mut self) -> [&mut dyn AnyStoredVec; ALL_PRIMARY_VEC_COUNT] {
        [
            &mut self.awake.supply.sats.height,
            &mut self.dormant.supply.sats.height,
            &mut self.awake.cap.cents.height,
            &mut self.awake.price.cents.height,
        ]
    }
}

impl StoredCohortVecs {
    fn push(&mut self, state: WeightedCohortState) {
        self.awake.supply.sats.height.push(state.weighted_supply);
        self.dormant
            .supply
            .sats
            .height
            .push(state.complement_supply);
        self.awake
            .supply_in_loss_share
            .height
            .push(state.supply_in_loss.value());
        self.awake.cap.cents.height.push(state.weighted_cap);
        self.awake
            .price
            .cents
            .height
            .push(realized_price(state.weighted_cap, state.weighted_supply));
    }

    fn primary_vecs_mut(&mut self) -> [&mut dyn AnyStoredVec; STORED_PRIMARY_VEC_COUNT] {
        [
            &mut self.awake.supply.sats.height,
            &mut self.dormant.supply.sats.height,
            &mut self.awake.supply_in_loss_share.height,
            &mut self.awake.cap.cents.height,
            &mut self.awake.price.cents.height,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn term_states_merge_into_all() {
        let mut sth = WeightedCohortState::default();
        sth.add(
            Sats::from(100_u64),
            Sats::from(20_u64),
            Cents::from(1_000_u64),
            StoredF64::from(0.3),
        );
        let mut lth = WeightedCohortState::default();
        lth.add(
            Sats::from(200_u64),
            Sats::from(50_u64),
            Cents::from(3_000_u64),
            StoredF64::from(0.4),
        );

        let all = sth.merged(lth);

        assert_eq!(
            all.weighted_supply,
            sth.weighted_supply + lth.weighted_supply
        );
        assert_eq!(
            all.complement_supply,
            sth.complement_supply + lth.complement_supply
        );
        assert_eq!(all.weighted_cap, sth.weighted_cap + lth.weighted_cap);
    }

    #[test]
    fn awake_and_dormant_supply_are_independently_floored() {
        let supply = Sats::from(123_456_789_u64);
        let weight = StoredF64::from(0.321);
        let mut state = WeightedCohortState::default();

        state.add(supply, Sats::ZERO, Cents::ZERO, weight);

        let sum = state.weighted_supply + state.complement_supply;
        assert!(sum <= supply);
        assert!(supply - sum <= Sats::from(1_u64));
    }
}
