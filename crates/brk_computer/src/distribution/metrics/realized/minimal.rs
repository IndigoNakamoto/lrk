use brk_error::Result;
use brk_indexer::Lengths;
use brk_traversable::Traversable;
use brk_types::{
    Bitcoin, Cents, CentsSigned, Height, PartsPerMillion64, PartsPerMillionSigned64, Sats,
    StoredF32, Version,
};
use derive_more::{Deref, DerefMut};
use vecdb::{AnyStoredVec, AnyVec, Exit, ReadableVec, Rw, StorageMode, WritableVec};

use crate::{
    distribution::state::{CohortState, CostBasisOps, RealizedOps},
    internal::{
        FiatPerBlockCumulativeWithSums, FiatPerBlockWithDeltas, Identity, LazyPerBlock,
        PriceWithRatioPerBlock,
    },
    price,
};

use crate::distribution::metrics::ImportConfig;

#[derive(Traversable)]
pub struct RealizedBase<M: StorageMode = Rw> {
    pub cap: FiatPerBlockWithDeltas<Cents, CentsSigned, PartsPerMillionSigned64, M>,
    pub profit: FiatPerBlockCumulativeWithSums<Cents, M>,
    pub loss: FiatPerBlockCumulativeWithSums<Cents, M>,
}

impl RealizedBase {
    pub(crate) fn forced_import(cfg: &ImportConfig) -> Result<Self> {
        let v1 = Version::ONE;

        Ok(Self {
            cap: FiatPerBlockWithDeltas::forced_import(
                cfg.db,
                &cfg.name("realized_cap"),
                cfg.version,
                Version::TWO,
                cfg.indexes,
                cfg.cached_starts,
            )?,
            profit: cfg.import("realized_profit", v1)?,
            loss: cfg.import("realized_loss", v1)?,
        })
    }

    pub(crate) fn min_stateful_len(&self) -> usize {
        self.cap
            .cents
            .height
            .len()
            .min(self.profit.cumulative.cents.height.len())
            .min(self.loss.cumulative.cents.height.len())
    }

    #[inline(always)]
    pub(crate) fn push_state(&mut self, state: &CohortState<impl RealizedOps, impl CostBasisOps>) {
        self.cap.cents.height.push(state.realized.cap());
        self.profit.push_block(state.realized.profit());
        self.loss.push_block(state.realized.loss());
    }

    pub(crate) fn collect_vecs_mut(&mut self) -> Vec<&mut dyn AnyStoredVec> {
        vec![
            &mut self.cap.cents.height as &mut dyn AnyStoredVec,
            self.profit.stored_mut(),
            self.loss.stored_mut(),
        ]
    }

    pub(crate) fn compute_from_stateful(
        &mut self,
        starting_lengths: &Lengths,
        others: &[&Self],
        exit: &Exit,
    ) -> Result<()> {
        sum_others!(self, starting_lengths, others, exit; cap.cents.height);
        self.profit.compute_sum_of_others(
            starting_lengths.height,
            &others.iter().map(|v| &v.profit).collect::<Vec<_>>(),
            exit,
        )?;
        self.loss.compute_sum_of_others(
            starting_lengths.height,
            &others.iter().map(|v| &v.loss).collect::<Vec<_>>(),
            exit,
        )?;
        Ok(())
    }
}

#[derive(Deref, DerefMut, Traversable)]
pub struct RealizedMinimal<M: StorageMode = Rw> {
    #[deref]
    #[deref_mut]
    #[traversable(flatten)]
    pub base: RealizedBase<M>,
    pub price: PriceWithRatioPerBlock<M>,
    pub mvrv: LazyPerBlock<StoredF32>,
}

impl RealizedMinimal {
    pub(crate) fn forced_import(cfg: &ImportConfig) -> Result<Self> {
        let v1 = Version::ONE;

        let base = RealizedBase::forced_import(cfg)?;
        let price: PriceWithRatioPerBlock = cfg.import("realized_price", v1)?;
        let mvrv = LazyPerBlock::from_lazy::<Identity<StoredF32>, PartsPerMillion64>(
            &cfg.name("mvrv"),
            cfg.version,
            &price.ratio,
        );

        Ok(Self { base, price, mvrv })
    }

    pub(crate) fn min_stateful_len(&self) -> usize {
        self.base.min_stateful_len()
    }

    #[inline(always)]
    pub(crate) fn push_state(&mut self, state: &CohortState<impl RealizedOps, impl CostBasisOps>) {
        self.base.push_state(state);
    }

    pub(crate) fn collect_vecs_mut(&mut self) -> Vec<&mut dyn AnyStoredVec> {
        self.base.collect_vecs_mut()
    }

    pub(crate) fn compute_from_stateful(
        &mut self,
        starting_lengths: &Lengths,
        others: &[&Self],
        exit: &Exit,
    ) -> Result<()> {
        let bases: Vec<_> = others.iter().map(|v| &v.base).collect();
        self.base
            .compute_from_stateful(starting_lengths, &bases, exit)
    }

    pub(crate) fn compute_rest_part2(
        &mut self,
        _prices: &price::Vecs,
        starting_lengths: &Lengths,
        height_to_supply: &impl ReadableVec<Height, Bitcoin>,
        exit: &Exit,
    ) -> Result<()> {
        let cap = &self.base.cap.cents.height;
        self.price.cents.height.compute_transform2(
            starting_lengths.height,
            cap,
            height_to_supply,
            |(i, cap_cents, supply, ..)| {
                let cap = cap_cents.as_u128();
                let supply_sats = Sats::from(supply).as_u128();
                let cents = (cap * Sats::ONE_BTC_U128)
                    .checked_div(supply_sats)
                    .map(Cents::from)
                    .unwrap_or(Cents::ZERO);
                (i, cents)
            },
            exit,
        )?;

        Ok(())
    }
}
