use brk_error::{OptionData, Result};
use brk_indexer::Indexer;
use brk_types::{CapitalSentimentPhase, Cents, Height, StoredI8, Version};
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

        let source_end = [spot.len(), all.len(), sth.len(), lth.len(), sma_1y.len()]
            .into_iter()
            .min()
            .unwrap_or_default();
        let start = self
            .phase_code
            .height
            .len()
            .min(indexer.safe_lengths().height.to_usize())
            .min(source_end);
        self.phase_code.height.any_truncate_if_needed_at(start)?;

        for height_index in start..source_end {
            let height = Height::from(height_index);
            let code = classify_phase_code(
                spot.collect_one(height).data()?,
                all.collect_one(height).data()?,
                sth.collect_one(height).data()?,
                lth.collect_one(height).data()?,
                sma_1y.collect_one(height).data()?,
            );

            self.phase_code.height.push(code);

            if (height_index + 1).is_multiple_of(WRITE_INTERVAL) || height_index + 1 == source_end {
                let _lock = exit.lock();
                self.phase_code.height.write()?;
            }
        }

        Ok(())
    }
}

/// Code `0` means the model's references are not all available yet.
fn classify_phase_code(price: Cents, all: Cents, sth: Cents, lth: Cents, sma: Cents) -> StoredI8 {
    if [price, all, sth, lth, sma].into_iter().any(Cents::is_nan) {
        StoredI8::ZERO
    } else {
        StoredI8::new(classify_phase(price, all, sth, lth, sma).code() as i8)
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
    let bull_structure = sth > lth;
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
    use super::*;

    fn cents(value: u64) -> Cents {
        Cents::new(value)
    }

    fn classify(price: u64, all: u64, sth: u64, lth: u64, sma: u64) -> CapitalSentimentPhase {
        classify_phase(cents(price), cents(all), cents(sth), cents(lth), cents(sma))
    }

    #[test]
    fn classifies_all_ten_phases() {
        use CapitalSentimentPhase as Phase;

        let cases = [
            ((100, 70, 80, 60, 50), Phase::RagingBull),
            ((100, 50, 70, 40, 60), Phase::Bull),
            ((50, 40, 30, 60, 70), Phase::CautiousBull),
            ((50, 70, 60, 80, 40), Phase::HopefulBull),
            ((40, 20, 30, 50, 10), Phase::EarlyBull),
            ((100, 50, 110, 60, 70), Phase::WeakBull),
            ((30, 40, 50, 20, 10), Phase::Limbo),
            ((10, 20, 30, 40, 50), Phase::DeepBear),
            ((10, 40, 30, 20, 50), Phase::Bear),
            ((30, 40, 50, 20, 60), Phase::EarlyBear),
        ];

        for ((price, all, sth, lth, sma), expected) in cases {
            assert_eq!(classify(price, all, sth, lth, sma), expected);
        }
    }

    #[test]
    fn sma_confirms_the_capitalized_price_structure() {
        use CapitalSentimentPhase as Phase;

        assert_eq!(classify(100, 70, 80, 60, 80), Phase::Bull);
        assert_eq!(classify(100, 70, 80, 60, 50), Phase::RagingBull);
    }

    #[test]
    fn missing_reference_has_no_phase() {
        assert_eq!(
            classify_phase_code(cents(100), cents(70), cents(80), Cents::NAN, cents(50)),
            StoredI8::ZERO
        );
    }
}
