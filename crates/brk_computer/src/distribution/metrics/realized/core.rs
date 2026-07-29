use brk_error::Result;
use brk_indexer::Lengths;
use brk_traversable::Traversable;
use brk_types::{
    Bitcoin, Cents, CentsSigned, Dollars, Height, PartsPerMillionSigned64, StoredF64, Version,
};
use derive_more::{Deref, DerefMut};
use vecdb::{AnyStoredVec, Exit, LazyVecFrom1, ReadableCloneableVec, ReadableVec, Rw, StorageMode};

use crate::{
    distribution::state::{CohortState, CostBasisOps, RealizedOps},
    internal::{
        FiatPerBlockCumulativeWithSumsAndDeltas, LazyPerBlock, NegCentsUnsignedToDollars,
        PerBlockCumulativeRolling, RatioCents64, RollingWindow24hPerBlock, Windows,
    },
    price,
};

use crate::distribution::metrics::ImportConfig;

use super::RealizedMinimal;

#[derive(Clone, Traversable)]
pub struct NegRealizedLoss {
    #[traversable(flatten)]
    pub base: LazyVecFrom1<Height, Dollars, Height, Cents>,
    pub sum: Windows<LazyPerBlock<Dollars, Cents>>,
}

#[derive(Traversable)]
pub struct RealizedSoprCore<M: StorageMode = Rw> {
    pub value_destroyed: PerBlockCumulativeRolling<Cents, M>,
    pub ratio: RollingWindow24hPerBlock<StoredF64, M>,
}

#[derive(Deref, DerefMut, Traversable)]
pub struct RealizedCore<M: StorageMode = Rw> {
    #[deref]
    #[deref_mut]
    #[traversable(flatten)]
    pub minimal: RealizedMinimal<M>,

    #[traversable(wrap = "loss", rename = "negative")]
    pub neg_loss: NegRealizedLoss,
    pub net_pnl: FiatPerBlockCumulativeWithSumsAndDeltas<
        CentsSigned,
        CentsSigned,
        PartsPerMillionSigned64,
        M,
    >,
    pub sopr: RealizedSoprCore<M>,
}

impl RealizedCore {
    pub(crate) fn forced_import(cfg: &ImportConfig) -> Result<Self> {
        let v1 = Version::ONE;

        let minimal = RealizedMinimal::forced_import(cfg)?;

        let neg_loss_base = LazyVecFrom1::transformed::<NegCentsUnsignedToDollars>(
            &cfg.name("realized_loss_neg"),
            cfg.version + Version::ONE,
            minimal.loss.block.cents.read_only_boxed_clone(),
        );

        let neg_loss_sum = minimal.loss.sum.0.map_with_suffix(|suffix, slot| {
            LazyPerBlock::from_height_source::<NegCentsUnsignedToDollars, _>(
                &cfg.name(&format!("realized_loss_neg_sum_{suffix}")),
                cfg.version + Version::ONE,
                slot.cents.height.clone(),
                cfg.indexes,
            )
        });

        let neg_loss = NegRealizedLoss {
            base: neg_loss_base,
            sum: neg_loss_sum,
        };

        let net_pnl = FiatPerBlockCumulativeWithSumsAndDeltas::forced_import(
            cfg.db,
            &cfg.name("net_realized_pnl"),
            cfg.version + v1,
            Version::new(5),
            cfg.indexes,
            cfg.cached_starts,
        )?;

        let value_destroyed = PerBlockCumulativeRolling::forced_import(
            cfg.db,
            &cfg.name("value_destroyed"),
            cfg.version + v1,
            cfg.indexes,
            cfg.cached_starts,
        )?;

        Ok(Self {
            minimal,
            neg_loss,
            net_pnl,
            sopr: RealizedSoprCore {
                value_destroyed,
                ratio: cfg.import("sopr", v1)?,
            },
        })
    }

    pub(crate) fn min_stateful_len(&self) -> usize {
        self.minimal.min_stateful_len()
    }

    #[inline(always)]
    pub(crate) fn push_state(&mut self, state: &CohortState<impl RealizedOps, impl CostBasisOps>) {
        self.minimal.push_state(state);
        self.sopr
            .value_destroyed
            .push_block(state.realized.value_destroyed());
    }

    pub(crate) fn collect_vecs_mut(&mut self) -> Vec<&mut dyn AnyStoredVec> {
        let mut vecs = self.minimal.collect_vecs_mut();
        vecs.push(self.sopr.value_destroyed.stored_mut());
        vecs
    }

    pub(crate) fn compute_from_stateful(
        &mut self,
        starting_lengths: &Lengths,
        others: &[&Self],
        exit: &Exit,
    ) -> Result<()> {
        let minimal_refs: Vec<&RealizedMinimal> = others.iter().map(|o| &o.minimal).collect();
        self.minimal
            .compute_from_stateful(starting_lengths, &minimal_refs, exit)?;

        sum_others!(self, starting_lengths, others, exit; sopr.value_destroyed.cumulative.height);
        Ok(())
    }

    pub(crate) fn compute_rest_part1(
        &mut self,
        starting_lengths: &Lengths,
        exit: &Exit,
    ) -> Result<()> {
        self.net_pnl.compute_from_cumulative_pair(
            starting_lengths.height,
            &self.minimal.profit.cumulative.cents.height,
            &self.minimal.loss.cumulative.cents.height,
            |_, profit, loss| CentsSigned::new(profit.inner() as i64 - loss.inner() as i64),
            exit,
        )
    }

    pub(crate) fn compute_rest_part2(
        &mut self,
        prices: &price::Vecs,
        starting_lengths: &Lengths,
        height_to_supply: &impl ReadableVec<Height, Bitcoin>,
        transfer_volume_sum_24h_cents: &impl ReadableVec<Height, Cents>,
        exit: &Exit,
    ) -> Result<()> {
        self.minimal
            .compute_rest_part2(prices, starting_lengths, height_to_supply, exit)?;

        self.sopr
            .ratio
            ._24h
            .compute_binary::<Cents, Cents, RatioCents64>(
                starting_lengths.height,
                transfer_volume_sum_24h_cents,
                &self.sopr.value_destroyed.sum._24h.height,
                exit,
            )?;

        Ok(())
    }
}
