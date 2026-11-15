use bitview_compute::{prepare_computed, write_locked};
use bitview_plugin::{ComputePlugin, UpdateContext, compute_items};
use bitview_plugin_indexer::Indexer;
use bitview_primitives::Float64;
use brk_error::{OptionData, Result};
use brk_exit::Exit;
use brk_oracle::cents_to_bin;
#[cfg(not(feature = "litecoin"))]
use brk_oracle::{
    Config, Oracle, START_HEIGHT_FAST, START_HEIGHT_SLOW, bin_to_cents, pre_oracle_prices_from,
};
#[cfg(not(feature = "litecoin"))]
use brk_types::Height;
#[cfg(not(feature = "litecoin"))]
use tracing::info;
use vecdb::{AnyStoredVec, AnyVec, ReadableVec, VecIndex, WritableVec};

use crate::{Dependencies, Vecs};
#[cfg(not(feature = "litecoin"))]
use crate::{feed_blocks_for_warmup, feed_blocks_with};

impl Vecs {
    /// Litecoin's prices: exchange daily closes below [`START_HEIGHT_SLOW`], the on-chain oracle
    /// from there, seeded at that height's close and kept off its aliases by the relock.
    #[cfg(feature = "litecoin")]
    fn compute_prices(&mut self, indexer: &Indexer, exit: &Exit) -> Result<()> {
        use bitview_primitives::Date;
        use brk_oracle::{START_HEIGHT_SLOW, bin_to_cents, relock};
        use brk_types::Version;
        use tracing::info;

        use crate::block_histograms;

        let starting_height = indexer.compute_start().height;
        let vecs = indexer.vecs();
        let source_version = [
            vecs.blocks.timestamp.version(),
            vecs.transactions.txid.version(),
            vecs.transactions.first_tx_index.version(),
            vecs.outputs.first_txout_index.version(),
            vecs.transactions.first_txout_index.version(),
            vecs.outputs.value.version(),
            vecs.outputs.output_type.version(),
        ]
        .into_iter()
        .sum::<Version>()
            + Version::new(crate::exchange::VERSION + relock::VERSION);
        let total_heights = vecs.blocks.timestamp.len();
        let committed = prepare_computed(
            [
                &mut self.spot.cents.height as &mut dyn AnyStoredVec,
                &mut self.oracle_bin,
                &mut self.oracle_vote,
            ],
            source_version,
            starting_height.to_usize(),
            exit,
        )?;
        if committed >= total_heights {
            return Ok(());
        }

        let below = total_heights.min(START_HEIGHT_SLOW);
        if committed < below {
            let timestamps = vecs.blocks.timestamp.collect_range_at(committed, below);
            let mut closes = crate::exchange::closes();
            closes.ensure(Date::from(*timestamps.last().data()?))?;
            for timestamp in timestamps {
                let cents = closes.price_at(Date::from(timestamp));
                self.spot.cents.height.push(cents);
                self.oracle_bin
                    .push(Float64::from(cents_to_bin(f64::from(cents))));
                self.oracle_vote.push(Float64::from(f64::NAN));
            }
        }
        let committed = self.spot.cents.height.len();
        if committed >= total_heights {
            return Ok(());
        }

        let (mut oracle, mut relock) = match (self.oracle.take(), self.relock.take()) {
            (Some((next, oracle)), Some(relock)) if next == committed => (oracle, relock),
            _ => self.restore_oracle(indexer, committed)?,
        };
        let num_new = total_heights - committed;
        let (mut processed, mut next_progress) = (0usize, 10u8);
        block_histograms(indexer, committed..total_heights, None, |height, hist| {
            let mut bin = oracle.process_histogram(&hist);
            relock.push(&hist);
            let mut vote = None;
            if relock::is_check(height) {
                vote = relock.vote(bin);
                if let Some(to) = relock.record(vote) {
                    info!(
                        "Oracle relocked at height {height}: ${} to ${}",
                        bin_to_cents(bin),
                        bin_to_cents(to)
                    );
                    oracle.relock(to);
                    bin = to;
                }
            }
            self.spot.cents.height.push(bin_to_cents(bin));
            self.oracle_bin.push(Float64::from(bin));
            self.oracle_vote
                .push(Float64::from(vote.unwrap_or(f64::NAN)));
            processed += 1;
            let progress = (processed * 100 / num_new) as u8;
            while num_new >= 100 && progress >= next_progress {
                info!("Oracle price computation: {next_progress}%");
                next_progress += 10;
            }
        })?;
        self.oracle = Some((total_heights, oracle));
        self.relock = Some(relock);
        Ok(())
    }

    /// The oracle and its relock as they stood after `committed - 1`: at the first on-chain
    /// height seeded at its exchange close, else at the stored bin and warmed over the window;
    /// the relock's day of payments read again, its last votes as stored.
    #[cfg(feature = "litecoin")]
    fn restore_oracle(
        &self,
        indexer: &Indexer,
        committed: usize,
    ) -> Result<(brk_oracle::Oracle, brk_oracle::relock::Relock)> {
        use bitview_primitives::Date;
        use brk_oracle::{
            Config, Oracle, START_HEIGHT_SLOW,
            relock::{self, Relock},
        };

        use crate::{block_histograms, feed_blocks_for_warmup};

        let seed = if committed == START_HEIGHT_SLOW {
            let date = Date::from(
                indexer
                    .vecs()
                    .blocks
                    .timestamp
                    .collect_one_at(committed)
                    .data()?,
            );
            let mut closes = crate::exchange::closes();
            closes.ensure(date)?;
            cents_to_bin(f64::from(closes.price_at(date)))
        } else {
            f64::from(self.oracle_bin.collect_one_at(committed - 1).data()?)
        };
        let config = Config::default();
        let warmup = config.window_size.min(committed - START_HEIGHT_SLOW);
        let oracle = Oracle::from_checkpoint(seed, config, |o| {
            feed_blocks_for_warmup(o, indexer, (committed - warmup)..committed, None)
        })?;
        let mut relock = Relock::default();
        block_histograms(
            indexer,
            committed.saturating_sub(relock::DAY)..committed,
            None,
            |_, hist| relock.push(&hist),
        )?;
        let first = committed
            .saturating_sub(relock::VOTES * relock::CHECK)
            .max(START_HEIGHT_SLOW);
        let votes = self.oracle_vote.collect_range_at(first, committed);
        for (height, vote) in (first..committed).zip(votes) {
            if relock::is_check(height) {
                let vote = f64::from(vote);
                relock.record((!vote.is_nan()).then_some(vote));
            }
        }
        Ok((oracle, relock))
    }

    /// The vecs this plugin computes, in the order it writes them.
    fn stored(&mut self) -> Vec<&mut dyn AnyStoredVec> {
        vec![
            &mut self.spot.cents.height as &mut dyn AnyStoredVec,
            &mut self.oracle_bin,
            #[cfg(feature = "litecoin")]
            &mut self.oracle_vote,
        ]
    }

    #[cfg(not(feature = "litecoin"))]
    fn compute_prices(&mut self, indexer: &Indexer, exit: &Exit) -> Result<()> {
        let starting_height = indexer.compute_start().height;

        let source_version = [
            indexer.vecs().transactions.txid.version(),
            indexer.vecs().transactions.first_tx_index.version(),
            indexer.vecs().outputs.first_txout_index.version(),
            indexer.vecs().transactions.first_txout_index.version(),
            indexer.vecs().outputs.value.version(),
            indexer.vecs().outputs.output_type.version(),
        ]
        .into_iter()
        .sum();
        let total_heights = indexer.vecs().blocks.timestamp.len();
        let committed = prepare_computed(
            [
                &mut self.spot.cents.height as &mut dyn AnyStoredVec,
                &mut self.oracle_bin,
            ],
            source_version,
            starting_height.to_usize(),
            exit,
        )?;
        // The slow regime's shape profile is not stored, so resuming inside that regime
        // replays it from its start, once readers have left the rows it rewrites.
        if (START_HEIGHT_SLOW + 1..START_HEIGHT_FAST).contains(&committed) {
            indexer.rewrite_from(Height::from(START_HEIGHT_SLOW), || {
                let _lock = exit.lock();
                self.spot
                    .cents
                    .height
                    .truncate_if_needed_at(START_HEIGHT_SLOW)?;
                self.oracle_bin.truncate_if_needed_at(START_HEIGHT_SLOW)?;
                Ok(())
            })?;
        }

        let seed_len = total_heights.min(START_HEIGHT_SLOW);
        let seed_start = self.spot.cents.height.len();
        for cents in pre_oracle_prices_from(seed_start).take(seed_len.saturating_sub(seed_start)) {
            self.spot.cents.height.push(cents);
            self.oracle_bin
                .push(Float64::from(cents_to_bin(f64::from(cents))));
        }

        if self.spot.cents.height.len() >= total_heights {
            return Ok(());
        }

        let committed = self.spot.cents.height.len();
        // The retained oracle continues exactly where the stored prices end; otherwise rebuild it
        // from the last stored bin and its warmup blocks (the same state: the EMA is a fold of
        // the window, newest first).
        let mut oracle = match self.oracle.take() {
            Some((next, oracle)) if next == committed => oracle,
            _ => {
                let config = Config::for_height(committed);
                let ref_bin = f64::from(self.oracle_bin.collect_one_at(committed - 1).data()?);
                let warmup = config.window_size.min(committed - START_HEIGHT_SLOW);
                Oracle::from_checkpoint(ref_bin, config, |o| {
                    feed_blocks_for_warmup(o, indexer, (committed - warmup)..committed, None)
                })?
            }
        };

        let num_new = total_heights - committed;

        // Slow cold-start EMA up to START_HEIGHT_FAST, then switch to the fast
        // mature-market EMA. Steady-state runs start past START_HEIGHT_FAST and skip
        // the slow segment entirely.
        {
            let mut processed = 0usize;
            let mut next_progress = 10u8;
            let mut push_ref_bin = |ref_bin| {
                self.spot.cents.height.push(bin_to_cents(ref_bin));
                self.oracle_bin.push(Float64::from(ref_bin));

                processed += 1;
                let progress = (processed * 100 / num_new) as u8;
                while num_new >= 100 && progress >= next_progress {
                    info!("Oracle price computation: {next_progress}%");
                    next_progress += 10;
                }
            };

            for segment in Config::segments_for_range(committed..total_heights) {
                let end = segment.end;
                feed_blocks_with(&mut oracle, indexer, segment, None, |_, _, ref_bin| {
                    push_ref_bin(ref_bin)
                })?;
                if end == START_HEIGHT_FAST {
                    oracle.reconfigure(Config::default());
                }
            }
        }

        self.oracle = Some((total_heights, oracle));

        Ok(())
    }
}

impl ComputePlugin for Vecs {
    compute_items!(Dependencies, db);

    fn compute_state(
        &mut self,
        dependencies: Self::Dependencies<'_>,
        context: UpdateContext<'_>,
    ) -> Result<()> {
        let Dependencies { indexer } = dependencies;
        let exit = context.exit();

        self.compute_prices(indexer, exit)?;
        write_locked(self.stored(), exit)
    }
}
