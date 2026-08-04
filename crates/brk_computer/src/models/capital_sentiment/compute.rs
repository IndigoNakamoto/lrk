use brk_error::{OptionData, Result};
use brk_indexer::Indexer;
use brk_types::{CapitalSentimentPhase, Cents, Height, StoredBool, StoredU8, Version};
use vecdb::{AnyStoredVec, AnyVec, Exit, ReadableVec, VecIndex, WritableVec};

use super::Vecs;
use crate::{
    distribution, internal::db_utils::validate_any_computed_version_or_reset, market, price,
};

const WRITE_INTERVAL: usize = 10_000;

impl Vecs {
    pub(crate) fn compute(
        &mut self,
        indexer: &Indexer,
        prices: &price::Vecs,
        distribution: &distribution::Vecs,
        market: &market::Vecs,
        exit: &Exit,
    ) -> Result<()> {
        let spot = &prices.spot.cents.height;
        let all = &distribution
            .utxo_cohorts
            .all
            .metrics
            .realized
            .capitalized
            .price
            .cents
            .height;
        let sth = &distribution
            .utxo_cohorts
            .sth
            .metrics
            .realized
            .capitalized
            .price
            .cents
            .height;
        let lth = &distribution
            .utxo_cohorts
            .lth
            .metrics
            .realized
            .capitalized
            .price
            .cents
            .height;
        let sma_1y = &market.moving_average.sma._1y.cents.height;

        let source_version: Version = [
            spot.version(),
            all.version(),
            sth.version(),
            lth.version(),
            sma_1y.version(),
        ]
        .into_iter()
        .sum();
        validate_any_computed_version_or_reset(&mut self.phase_code.height, source_version)?;
        validate_any_computed_version_or_reset(&mut self.is_long.height, source_version)?;

        let source_end = [spot.len(), all.len(), sth.len(), lth.len(), sma_1y.len()]
            .into_iter()
            .min()
            .unwrap_or_default();
        let start = self
            .phase_code
            .height
            .len()
            .min(self.is_long.height.len())
            .min(indexer.safe_lengths().height.to_usize())
            .min(source_end);
        self.phase_code.height.any_truncate_if_needed_at(start)?;
        self.is_long.height.any_truncate_if_needed_at(start)?;

        let mut is_long = start
            .checked_sub(1)
            .map(Height::from)
            .map(|height| self.is_long.height.collect_one(height).data())
            .transpose()?
            .is_some_and(|value| value.is_true());
        let mut previous_price = start
            .checked_sub(1)
            .map(Height::from)
            .map(|height| spot.collect_one(height).data())
            .transpose()?;
        let mut previous_sth = start
            .checked_sub(1)
            .map(Height::from)
            .map(|height| sth.collect_one(height).data())
            .transpose()?;

        for height_index in start..source_end {
            let height = Height::from(height_index);
            let price = spot.collect_one(height).data()?;
            let sth = sth.collect_one(height).data()?;
            let code = classify_phase_code(
                price,
                all.collect_one(height).data()?,
                sth,
                lth.collect_one(height).data()?,
                sma_1y.collect_one(height).data()?,
            );
            is_long = next_is_long(is_long, previous_price.zip(previous_sth), price, sth, code);

            self.phase_code.height.push(code);
            self.is_long.height.push(StoredBool::from(is_long));
            previous_price = Some(price);
            previous_sth = Some(sth);

            if (height_index + 1).is_multiple_of(WRITE_INTERVAL) || height_index + 1 == source_end {
                let _lock = exit.lock();
                self.phase_code.height.write()?;
                self.is_long.height.write()?;
            }
        }

        Ok(())
    }
}

/// Advance the stateful cash/long strategy used by BRK Signal.
fn next_is_long(
    is_long: bool,
    previous: Option<(Cents, Cents)>,
    price: Cents,
    sth: Cents,
    phase_code: StoredU8,
) -> bool {
    let crossed_above_sth = previous.is_some_and(|(previous_price, previous_sth)| {
        !is_over_sth(previous_price, previous_sth) && is_over_sth(price, sth)
    });

    if !is_long && crossed_above_sth {
        return true;
    }
    if is_long && CapitalSentimentPhase::from_code(*phase_code).is_some_and(|phase| phase.is_sell())
    {
        return false;
    }
    is_long
}

#[inline]
fn is_over_sth(price: Cents, sth: Cents) -> bool {
    !price.is_nan() && !sth.is_nan() && price > Cents::ZERO && sth > Cents::ZERO && price >= sth
}

/// Code `0` means the model's references are not all available yet.
fn classify_phase_code(price: Cents, all: Cents, sth: Cents, lth: Cents, sma: Cents) -> StoredU8 {
    if [price, all, sth, lth, sma].into_iter().any(Cents::is_nan) {
        StoredU8::ZERO
    } else {
        StoredU8::new(classify_phase(price, all, sth, lth, sma).code())
    }
}

/// Classify investor sentiment from the three capitalized-price references,
/// using the 1-year SMA only as confirmation and disambiguation.
fn classify_phase(
    price: Cents,
    all: Cents,
    sth: Cents,
    lth: Cents,
    sma: Cents,
) -> CapitalSentimentPhase {
    use CapitalSentimentPhase as Phase;

    let above_all = price >= all;
    let above_sth = price >= sth;
    let above_lth = price >= lth;
    let above_sma = price >= sma;
    let bull_structure = sth >= lth;
    let above_slow_refs = above_all && above_lth;
    let references_above_price = [all, sth, lth, sma]
        .into_iter()
        .filter(|reference| *reference > price)
        .count();
    let price_in_middle = references_above_price == 2;
    let core_bull_phase = if all > sma {
        Phase::RagingBull
    } else {
        Phase::Bull
    };
    let core_bear_phase = if lth > all {
        Phase::DeepBear
    } else {
        Phase::Bear
    };

    if !above_all && !above_sth && !above_lth && !above_sma {
        return core_bear_phase;
    }
    if !above_sth && price_in_middle {
        return Phase::Limbo;
    }
    if bull_structure && (!above_slow_refs || !above_sma) {
        return Phase::EarlyBear;
    }
    if above_sth && above_sma {
        return if above_slow_refs {
            core_bull_phase
        } else {
            Phase::EarlyBull
        };
    }
    if !above_sth && above_slow_refs && above_sma {
        return Phase::WeakBull;
    }
    if above_sth {
        return Phase::CautiousBull;
    }
    if above_sma {
        return Phase::HopefulBull;
    }
    Phase::EarlyBear
}

#[cfg(test)]
mod tests {
    use std::{cmp::Reverse, collections::BTreeSet};

    use super::*;

    fn cents(value: u64) -> Cents {
        Cents::new(value)
    }

    fn classify(price: u64, all: u64, sth: u64, lth: u64, sma: u64) -> CapitalSentimentPhase {
        classify_phase(cents(price), cents(all), cents(sth), cents(lth), cents(sma))
    }

    #[test]
    fn classifies_all_ten_phases_across_all_eight_reference_orders() {
        use CapitalSentimentPhase as Phase;

        let cases = [
            ((100, 70, 80, 50, 60), Phase::RagingBull),
            ((100, 70, 80, 60, 90), Phase::Bull),
            ((90, 70, 60, 80, 100), Phase::CautiousBull),
            ((40, 80, 60, 100, 20), Phase::HopefulBull),
            ((90, 70, 60, 100, 80), Phase::EarlyBull),
            ((90, 70, 100, 60, 80), Phase::WeakBull),
            ((70, 80, 100, 60, 40), Phase::Limbo),
            ((40, 80, 60, 100, 70), Phase::DeepBear),
            ((40, 80, 100, 60, 50), Phase::Bear),
            ((60, 80, 100, 50, 70), Phase::EarlyBear),
        ];

        let mut reference_orders = BTreeSet::new();

        for ((price, all, sth, lth, sma), expected) in cases {
            assert!(
                (sth > all && all > lth) || (lth > all && all > sth),
                "All capitalized price must be between STH and LTH"
            );

            let mut references = [("SMA", sma), ("STH", sth), ("All", all), ("LTH", lth)];
            references.sort_unstable_by_key(|(_, value)| Reverse(*value));
            reference_orders.insert(references.map(|(name, _)| name));

            assert_eq!(classify(price, all, sth, lth, sma), expected);
        }

        assert_eq!(reference_orders.len(), 8);
    }

    #[test]
    fn sma_confirms_the_capitalized_price_structure() {
        use CapitalSentimentPhase as Phase;

        assert_eq!(classify(100, 70, 80, 60, 90), Phase::Bull);
        assert_eq!(classify(100, 70, 80, 60, 50), Phase::RagingBull);
    }

    #[test]
    fn capitalized_crossover_uses_sth_led_tie_break() {
        assert_eq!(
            classify(70, 50, 50, 50, 100),
            CapitalSentimentPhase::EarlyBear
        );
    }

    #[test]
    fn missing_reference_has_no_phase() {
        assert_eq!(
            classify_phase_code(cents(100), cents(70), cents(80), Cents::NAN, cents(50)),
            StoredU8::ZERO
        );
    }

    #[test]
    fn signal_enters_only_on_an_sth_cross_and_exits_on_a_sell_phase() {
        use CapitalSentimentPhase as Phase;

        let bull = StoredU8::new(Phase::Bull.code());
        let bear = StoredU8::new(Phase::Bear.code());

        assert!(!next_is_long(false, None, cents(100), cents(80), bull));
        assert!(next_is_long(
            false,
            Some((cents(70), cents(80))),
            cents(80),
            cents(80),
            bull,
        ));
        assert!(!next_is_long(
            false,
            Some((cents(90), cents(80))),
            cents(100),
            cents(80),
            bull,
        ));
        assert!(next_is_long(
            true,
            Some((cents(90), cents(80))),
            cents(100),
            cents(80),
            bull,
        ));
        assert!(!next_is_long(
            true,
            Some((cents(90), cents(80))),
            cents(100),
            cents(80),
            bear,
        ));
    }
}
