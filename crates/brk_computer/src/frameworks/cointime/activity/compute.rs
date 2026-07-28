use brk_error::Result;
use brk_indexer::Indexer;
use brk_types::{Bitcoin, StoredF64};
use vecdb::Exit;

use super::{DerivedVecs, Vecs};
use crate::{distribution, internal::PerBlockCumulativeRolling};

pub(crate) fn compute_rest(
    starting_height: brk_types::Height,
    created: &PerBlockCumulativeRolling<StoredF64, StoredF64>,
    consumed: &PerBlockCumulativeRolling<StoredF64, StoredF64>,
    stored: &mut PerBlockCumulativeRolling<StoredF64, StoredF64>,
    derived: &mut DerivedVecs,
    exit: &Exit,
) -> Result<()> {
    stored.compute(starting_height, exit, |vec| {
        vec.compute_subtract(starting_height, &created.block, &consumed.block, exit)?;
        Ok(())
    })?;

    derived.liveliness.height.compute_divide(
        starting_height,
        &consumed.cumulative.height,
        &created.cumulative.height,
        exit,
    )?;

    derived.ratio.height.compute_divide(
        starting_height,
        &derived.liveliness.height,
        &derived.vaultedness.height,
        exit,
    )?;

    Ok(())
}

impl Vecs {
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        distribution: &distribution::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        let starting_height = indexer.safe_lengths().height;
        let all_metrics = &distribution.utxo_cohorts.all.metrics;
        let circulating_supply = &all_metrics.supply.total.sats.height;

        self.coinblocks_created
            .compute(starting_height, exit, |vec| {
                vec.compute_transform(
                    starting_height,
                    circulating_supply,
                    |(i, v, ..)| (i, StoredF64::from(Bitcoin::from(v))),
                    exit,
                )?;
                Ok(())
            })?;

        compute_rest(
            starting_height,
            &self.coinblocks_created,
            &distribution.coinblocks_destroyed,
            &mut self.coinblocks_stored,
            &mut self.derived,
            exit,
        )
    }
}
