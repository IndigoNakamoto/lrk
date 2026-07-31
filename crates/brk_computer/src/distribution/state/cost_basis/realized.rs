use std::cmp::Ordering;

use brk_types::{Cents, CentsSats, CentsSquaredSats, Sats};
use vecdb::unlikely;

/// Trait for realized state operations, implemented by Minimal, Core, and Full variants.
pub trait RealizedOps: Default + Clone + Send + Sync + 'static {
    const TRACK_ACTIVITY: bool = false;
    fn cap(&self) -> Cents;
    fn profit(&self) -> Cents;
    fn loss(&self) -> Cents;
    fn value_destroyed(&self) -> Cents {
        Cents::ZERO
    }
    fn sent_in_profit(&self) -> Sats {
        Sats::ZERO
    }
    fn sent_in_loss(&self) -> Sats {
        Sats::ZERO
    }
    fn set_cap_raw(&mut self, cap_raw: CentsSats);
    fn set_capitalized_cap_raw(&mut self, capitalized_cap_raw: CentsSquaredSats);
    fn reset_single_iteration_values(&mut self);
    fn increment(&mut self, price: Cents, sats: Sats);
    fn increment_snapshot(&mut self, price_sats: CentsSats, capitalized_cap: CentsSquaredSats);
    fn decrement_snapshot(&mut self, price_sats: CentsSats, capitalized_cap: CentsSquaredSats);
    fn receive(&mut self, price: Cents, sats: Sats) {
        self.increment(price, sats);
    }
    fn send(
        &mut self,
        sats: Sats,
        current_ps: CentsSats,
        prev_ps: CentsSats,
        ath_ps: CentsSats,
        prev_capitalized_cap: CentsSquaredSats,
    );
}

/// Minimal realized state: cap, profit, and loss.
/// Used by MinimalCohortMetrics cohorts (amount_range and type) and address cohorts.
#[derive(Debug, Default, Clone)]
pub struct MinimalRealizedState {
    cap_raw: u128,
    profit_raw: u128,
    loss_raw: u128,
}

impl MinimalRealizedState {
    #[inline]
    pub(crate) fn increment_cap(&mut self, cap: CentsSats) {
        self.cap_raw += cap.as_u128();
    }

    #[inline]
    pub(crate) fn decrement_cap(&mut self, cap: CentsSats) {
        self.cap_raw -= cap.as_u128();
    }

    #[inline]
    pub(crate) fn realize_spend(&mut self, current: CentsSats, previous: CentsSats) {
        match current.cmp(&previous) {
            Ordering::Greater => {
                self.profit_raw += (current - previous).as_u128();
            }
            Ordering::Less => {
                self.loss_raw += (previous - current).as_u128();
            }
            Ordering::Equal => {}
        }
        self.decrement_cap(previous);
    }
}

impl RealizedOps for MinimalRealizedState {
    #[inline]
    fn cap(&self) -> Cents {
        if self.cap_raw == 0 {
            return Cents::ZERO;
        }
        Cents::new((self.cap_raw / Sats::ONE_BTC_U128) as u64)
    }

    #[inline]
    fn profit(&self) -> Cents {
        if self.profit_raw == 0 {
            return Cents::ZERO;
        }
        Cents::new((self.profit_raw / Sats::ONE_BTC_U128) as u64)
    }

    #[inline]
    fn loss(&self) -> Cents {
        if self.loss_raw == 0 {
            return Cents::ZERO;
        }
        Cents::new((self.loss_raw / Sats::ONE_BTC_U128) as u64)
    }

    #[inline]
    fn set_cap_raw(&mut self, cap_raw: CentsSats) {
        self.cap_raw = cap_raw.inner();
    }

    #[inline]
    fn set_capitalized_cap_raw(&mut self, _capitalized_cap_raw: CentsSquaredSats) {}

    #[inline]
    fn reset_single_iteration_values(&mut self) {
        self.profit_raw = 0;
        self.loss_raw = 0;
    }

    #[inline]
    fn increment(&mut self, price: Cents, sats: Sats) {
        if sats.is_zero() {
            return;
        }
        self.increment_cap(CentsSats::from_price_sats(price, sats));
    }

    #[inline]
    fn increment_snapshot(&mut self, price_sats: CentsSats, _capitalized_cap: CentsSquaredSats) {
        self.increment_cap(price_sats);
    }

    #[inline]
    fn decrement_snapshot(&mut self, price_sats: CentsSats, _capitalized_cap: CentsSquaredSats) {
        self.decrement_cap(price_sats);
    }

    #[inline]
    fn send(
        &mut self,
        _sats: Sats,
        current_ps: CentsSats,
        prev_ps: CentsSats,
        _ath_ps: CentsSats,
        _prev_capitalized_cap: CentsSquaredSats,
    ) {
        self.realize_spend(current_ps, prev_ps);
    }
}

/// Core realized state: extends Minimal with activity tracking.
/// Used by CoreCohortMetrics cohorts (entry, epoch, class, under_age, over_age).
#[derive(Debug, Default, Clone)]
pub struct CoreRealizedState {
    minimal: MinimalRealizedState,
    value_destroyed_raw: u128,
    sent_in_profit: Sats,
    sent_in_loss: Sats,
}

impl RealizedOps for CoreRealizedState {
    const TRACK_ACTIVITY: bool = true;

    #[inline]
    fn cap(&self) -> Cents {
        self.minimal.cap()
    }

    #[inline]
    fn profit(&self) -> Cents {
        self.minimal.profit()
    }

    #[inline]
    fn loss(&self) -> Cents {
        self.minimal.loss()
    }

    #[inline]
    fn value_destroyed(&self) -> Cents {
        if unlikely(self.value_destroyed_raw == 0) {
            return Cents::ZERO;
        }
        Cents::new((self.value_destroyed_raw / Sats::ONE_BTC_U128) as u64)
    }

    #[inline]
    fn sent_in_profit(&self) -> Sats {
        self.sent_in_profit
    }

    #[inline]
    fn sent_in_loss(&self) -> Sats {
        self.sent_in_loss
    }

    #[inline]
    fn set_cap_raw(&mut self, cap_raw: CentsSats) {
        self.minimal.set_cap_raw(cap_raw);
    }

    #[inline]
    fn set_capitalized_cap_raw(&mut self, _capitalized_cap_raw: CentsSquaredSats) {}

    #[inline]
    fn reset_single_iteration_values(&mut self) {
        self.minimal.reset_single_iteration_values();
        self.value_destroyed_raw = 0;
        self.sent_in_profit = Sats::ZERO;
        self.sent_in_loss = Sats::ZERO;
    }

    #[inline]
    fn increment(&mut self, price: Cents, sats: Sats) {
        self.minimal.increment(price, sats);
    }

    #[inline]
    fn increment_snapshot(&mut self, price_sats: CentsSats, _capitalized_cap: CentsSquaredSats) {
        self.minimal
            .increment_snapshot(price_sats, _capitalized_cap);
    }

    #[inline]
    fn decrement_snapshot(&mut self, price_sats: CentsSats, _capitalized_cap: CentsSquaredSats) {
        self.minimal
            .decrement_snapshot(price_sats, _capitalized_cap);
    }

    #[inline]
    fn send(
        &mut self,
        sats: Sats,
        current_ps: CentsSats,
        prev_ps: CentsSats,
        ath_ps: CentsSats,
        prev_capitalized_cap: CentsSquaredSats,
    ) {
        self.minimal
            .send(sats, current_ps, prev_ps, ath_ps, prev_capitalized_cap);
        self.value_destroyed_raw += prev_ps.as_u128();
        match current_ps.cmp(&prev_ps) {
            Ordering::Greater | Ordering::Equal => {
                self.sent_in_profit += sats;
            }
            Ordering::Less => {
                self.sent_in_loss += sats;
            }
        }
    }
}

impl CoreRealizedState {
    #[inline(always)]
    pub(super) fn cap_raw_u128(&self) -> u128 {
        self.minimal.cap_raw
    }
}

/// Full realized state (~160 bytes).
/// Used by BasicCohortMetrics cohorts (age_range — 23 separate cohorts).
#[derive(Debug, Default, Clone)]
pub struct RealizedState {
    core: CoreRealizedState,
    /// Raw capitalized cap: Σ(price² × sats)
    capitalized_cap_raw: CentsSquaredSats,
    /// Raw realized peak regret: Σ((peak - sell_price) × sats)
    peak_regret_raw: u128,
}

impl RealizedOps for RealizedState {
    const TRACK_ACTIVITY: bool = true;

    #[inline]
    fn cap(&self) -> Cents {
        self.core.cap()
    }

    #[inline]
    fn profit(&self) -> Cents {
        self.core.profit()
    }

    #[inline]
    fn loss(&self) -> Cents {
        self.core.loss()
    }

    #[inline]
    fn value_destroyed(&self) -> Cents {
        self.core.value_destroyed()
    }

    #[inline]
    fn sent_in_profit(&self) -> Sats {
        self.core.sent_in_profit()
    }

    #[inline]
    fn sent_in_loss(&self) -> Sats {
        self.core.sent_in_loss()
    }

    #[inline]
    fn set_cap_raw(&mut self, cap_raw: CentsSats) {
        self.core.set_cap_raw(cap_raw);
    }

    #[inline]
    fn set_capitalized_cap_raw(&mut self, capitalized_cap_raw: CentsSquaredSats) {
        self.capitalized_cap_raw = capitalized_cap_raw;
    }

    #[inline]
    fn reset_single_iteration_values(&mut self) {
        self.core.reset_single_iteration_values();
        self.peak_regret_raw = 0;
    }

    #[inline]
    fn increment(&mut self, price: Cents, sats: Sats) {
        self.core.increment(price, sats);
        if sats.is_not_zero() {
            self.capitalized_cap_raw +=
                CentsSats::from_price_sats(price, sats).to_capitalized_cap(price);
        }
    }

    #[inline]
    fn increment_snapshot(&mut self, price_sats: CentsSats, capitalized_cap: CentsSquaredSats) {
        self.core.increment_snapshot(price_sats, capitalized_cap);
        self.capitalized_cap_raw += capitalized_cap;
    }

    #[inline]
    fn decrement_snapshot(&mut self, price_sats: CentsSats, capitalized_cap: CentsSquaredSats) {
        self.core.decrement_snapshot(price_sats, capitalized_cap);
        self.capitalized_cap_raw -= capitalized_cap;
    }

    #[inline]
    fn send(
        &mut self,
        sats: Sats,
        current_ps: CentsSats,
        prev_ps: CentsSats,
        ath_ps: CentsSats,
        prev_capitalized_cap: CentsSquaredSats,
    ) {
        self.core
            .send(sats, current_ps, prev_ps, ath_ps, prev_capitalized_cap);

        self.peak_regret_raw += (ath_ps - current_ps).as_u128();
        self.capitalized_cap_raw -= prev_capitalized_cap;
    }
}

impl RealizedState {
    /// Get capitalized price as CentsUnsigned.
    /// capitalized_price = Σ(price² × sats) / Σ(price × sats)
    #[inline]
    pub(crate) fn capitalized_price(&self) -> Cents {
        let cap_raw = self.core.cap_raw_u128();
        if cap_raw == 0 {
            return Cents::ZERO;
        }
        Cents::new((self.capitalized_cap_raw / cap_raw) as u64)
    }

    /// Get raw realized cap for aggregation.
    #[inline]
    pub(crate) fn cap_raw(&self) -> CentsSats {
        CentsSats::new(self.core.cap_raw_u128())
    }

    /// Get raw capitalized cap for aggregation.
    #[inline]
    pub(crate) fn capitalized_cap_raw(&self) -> CentsSquaredSats {
        self.capitalized_cap_raw
    }

    /// Get realized peak regret as CentsUnsigned.
    #[inline]
    pub(crate) fn peak_regret(&self) -> Cents {
        if self.peak_regret_raw == 0 {
            return Cents::ZERO;
        }
        Cents::new((self.peak_regret_raw / Sats::ONE_BTC_U128) as u64)
    }

    /// Raw peak regret for lossless aggregation.
    #[inline]
    pub(crate) fn peak_regret_raw(&self) -> u128 {
        self.peak_regret_raw
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn price_sats(price: u64) -> CentsSats {
        CentsSats::from_price_sats(Cents::new(price), Sats::ONE_BTC)
    }

    #[test]
    fn minimal_tracks_only_cap_profit_and_loss() {
        let mut state = MinimalRealizedState::default();
        state.increment(Cents::new(10_000), Sats::ONE_BTC);
        state.send(
            Sats::ONE_BTC,
            price_sats(15_000),
            price_sats(10_000),
            CentsSats::ZERO,
            CentsSquaredSats::ZERO,
        );

        assert_eq!(state.cap(), Cents::ZERO);
        assert_eq!(state.profit(), Cents::new(5_000));
        assert_eq!(state.loss(), Cents::ZERO);
        assert_eq!(state.value_destroyed(), Cents::ZERO);
        assert_eq!(state.sent_in_profit(), Sats::ZERO);
        assert_eq!(state.sent_in_loss(), Sats::ZERO);
    }

    #[test]
    fn core_adds_activity_to_minimal_realized_state() {
        let mut state = CoreRealizedState::default();
        state.increment(Cents::new(10_000), Sats::ONE_BTC);
        state.send(
            Sats::ONE_BTC,
            price_sats(15_000),
            price_sats(10_000),
            CentsSats::ZERO,
            CentsSquaredSats::ZERO,
        );

        assert_eq!(state.cap(), Cents::ZERO);
        assert_eq!(state.profit(), Cents::new(5_000));
        assert_eq!(state.loss(), Cents::ZERO);
        assert_eq!(state.value_destroyed(), Cents::new(10_000));
        assert_eq!(state.sent_in_profit(), Sats::ONE_BTC);
        assert_eq!(state.sent_in_loss(), Sats::ZERO);
    }
}
