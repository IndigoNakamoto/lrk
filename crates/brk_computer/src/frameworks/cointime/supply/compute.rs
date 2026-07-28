use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::{Height, Sats, StoredF64, Version};
use vecdb::{AnyStoredVec, AnyVec, Exit, ReadableVec, WritableVec};

use super::super::{activity, age_range};
use super::{BaseVecs, Vecs};
use crate::{distribution, frameworks::WeightedRatio, price};

const WRITE_INTERVAL: usize = 10_000;

impl BaseVecs {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn compute_from(
        &mut self,
        starting_height: Height,
        prices: &price::Vecs,
        total_supply: &impl ReadableVec<Height, Sats>,
        liveliness: &impl ReadableVec<Height, StoredF64>,
        vaultedness: &impl ReadableVec<Height, StoredF64>,
        exit: &Exit,
    ) -> Result<()> {
        self.vaulted.sats.height.compute_multiply(
            starting_height,
            total_supply,
            vaultedness,
            exit,
        )?;

        self.active.sats.height.compute_multiply(
            starting_height,
            total_supply,
            liveliness,
            exit,
        )?;

        self.vaulted.compute(prices, starting_height, exit)?;
        self.active.compute(prices, starting_height, exit)?;

        Ok(())
    }
}

impl Vecs {
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        prices: &price::Vecs,
        distribution: &distribution::Vecs,
        activity: &activity::Vecs,
        age_range: &age_range::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        let starting_height = indexer.safe_lengths().height;
        let circulating_supply = &distribution
            .utxo_cohorts
            .all
            .metrics
            .supply
            .total
            .sats
            .height;

        self.base.compute_from(
            starting_height,
            prices,
            circulating_supply,
            &activity.liveliness.height,
            &activity.vaultedness.height,
            exit,
        )?;

        let source_cohorts: Vec<_> = distribution.utxo_cohorts.age_range.iter().collect();
        let total_supplies: Vec<_> = source_cohorts
            .iter()
            .map(|cohort| &cohort.metrics.supply.total.sats.height)
            .collect();
        let loss_supplies: Vec<_> = source_cohorts
            .iter()
            .map(|cohort| &cohort.metrics.supply.in_loss.sats.height)
            .collect();
        let weights: Vec<_> = age_range
            .iter()
            .map(|cohort| &cohort.liveliness.height)
            .collect();

        self.compute_active_supply_in_loss_share(
            starting_height,
            &total_supplies,
            &loss_supplies,
            &weights,
            exit,
        )
    }

    fn compute_active_supply_in_loss_share<S, W>(
        &mut self,
        starting_height: Height,
        total_supplies: &[&S],
        loss_supplies: &[&S],
        weights: &[&W],
        exit: &Exit,
    ) -> Result<()>
    where
        S: ReadableVec<Height, Sats>,
        W: ReadableVec<Height, StoredF64>,
    {
        debug_assert_eq!(total_supplies.len(), loss_supplies.len());
        debug_assert_eq!(total_supplies.len(), weights.len());

        let source_version: Version = total_supplies
            .iter()
            .map(|vec| vec.version())
            .chain(loss_supplies.iter().map(|vec| vec.version()))
            .chain(weights.iter().map(|vec| vec.version()))
            .sum();
        let output = &mut self.active_supply_in_loss_share.height;
        output.validate_computed_version_or_reset(source_version)?;

        let start = output.len().min(usize::from(starting_height));
        output.truncate_if_needed_at(start)?;
        let source_end = total_supplies
            .iter()
            .map(|vec| vec.len())
            .chain(loss_supplies.iter().map(|vec| vec.len()))
            .chain(weights.iter().map(|vec| vec.len()))
            .min()
            .unwrap_or_default();

        let mut chunk_start = start;
        while chunk_start < source_end {
            let chunk_end = (chunk_start + WRITE_INTERVAL).min(source_end);
            let total_batches: Vec<_> = total_supplies
                .iter()
                .map(|vec| vec.collect_range_at(chunk_start, chunk_end))
                .collect();
            let loss_batches: Vec<_> = loss_supplies
                .iter()
                .map(|vec| vec.collect_range_at(chunk_start, chunk_end))
                .collect();
            let weight_batches: Vec<_> = weights
                .iter()
                .map(|vec| vec.collect_range_at(chunk_start, chunk_end))
                .collect();

            for offset in 0..(chunk_end - chunk_start) {
                let mut supply_in_loss = WeightedRatio::default();
                for cohort in 0..weights.len() {
                    let weight = f64::from(weight_batches[cohort][offset]);
                    supply_in_loss.add(
                        loss_batches[cohort][offset].as_u128() as f64,
                        total_batches[cohort][offset].as_u128() as f64,
                        weight,
                    );
                }
                output.push(supply_in_loss.value());
            }

            {
                let _lock = exit.lock();
                output.write()?;
            }
            chunk_start = chunk_end;
        }

        Ok(())
    }
}
