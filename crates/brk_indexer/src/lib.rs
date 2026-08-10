#![doc = include_str!("../README.md")]

use std::{
    fs,
    path::{Path, PathBuf},
    thread,
    time::Instant,
};

use brk_chain::Chain;
use brk_error::Result;
use brk_reader::{Reader, XORBytes};
use brk_rpc::Client;
use brk_types::{BlockHash, Height, TxidPrefix};
use fjall::PersistMode;
use tracing::{debug, error, info};
use vecdb::{
    Exit, RawDBError, ReadOnlyClone, ReadableVec, Ro, Rw, StorageMode, VecIndex, WritableVec,
    unlikely,
};
mod constants;
mod lengths;
mod mweb_summary;
mod processor;
mod readers;
mod safe_lengths;
mod stores;
mod vecs;

use constants::*;
use processor::{BlockBuffers, BlockProcessor};
use readers::Readers;

pub use lengths::Lengths;
pub use mweb_summary::MwebSummary;
pub use safe_lengths::SafeLengths;
pub use stores::Stores;
pub use vecs::*;

pub struct Indexer<M: StorageMode = Rw> {
    path: PathBuf,
    pub chain: Chain,
    pub vecs: Vecs<M>,
    pub stores: Stores,
    safe_lengths: SafeLengths,
}

impl<M: StorageMode> Indexer<M> {
    /// Tip block hash at the pipeline-safe ceiling.
    ///
    /// Reads the on-disk blockhash vec at `safe_lengths.height - 1` so
    /// the answer always agrees with `safe_lengths`. The indexer's loop
    /// pushes new hashes per block before `safe_lengths` advances (that
    /// only happens after the compute pass via
    /// [`Indexer::advance_safe_lengths`]); reading from a live cache
    /// here would mint a tip ahead of every safe-bound endpoint and
    /// cause cache etags to invalidate before the data they cover is
    /// actually queryable.
    pub fn tip_blockhash(&self) -> BlockHash {
        match self.safe_lengths().height.decremented() {
            Some(h) => self.vecs.blocks.blockhash.collect_one(h).unwrap_or_default(),
            None => BlockHash::default(),
        }
    }

    /// Pipeline-safe `Lengths` snapshot shared with `Query`. Writers
    /// advance and lower this internally; readers clamp non-series
    /// answers against this loaded snapshot.
    pub fn safe_lengths(&self) -> Lengths {
        self.safe_lengths.load()
    }
}

impl Indexer<Ro> {
    /// Live indexer stamp for diagnostics. For data reads use
    /// [`crate::SafeLengths::load`] (via `Query::height`).
    pub fn indexed_height(&self) -> Height {
        Height::from(self.vecs.blocks.blockhash.inner.stamp())
    }
}

impl Indexer {
    pub fn forced_import(outputs_dir: &Path) -> Result<Self> {
        Self::forced_import_with_chain(outputs_dir, Chain::Bitcoin)
    }

    pub fn forced_import_with_chain(outputs_dir: &Path, chain: Chain) -> Result<Self> {
        Self::forced_import_inner(outputs_dir, chain, true)
    }

    fn forced_import_inner(outputs_dir: &Path, chain: Chain, can_retry: bool) -> Result<Self> {
        info!("Importing indexer...");

        let indexed_path = outputs_dir.join("indexed");

        let try_import = || -> Result<Self> {
            let i = Instant::now();
            let vecs = Vecs::forced_import(&indexed_path, VERSION)?;
            info!("Imported vecs in {:?}", i.elapsed());

            let i = Instant::now();
            let stores = Stores::forced_import(&indexed_path, VERSION)?;
            info!("Imported stores in {:?}", i.elapsed());

            let safe_lengths = SafeLengths::new();
            if let Some(lengths) = Lengths::from_local(&vecs, &stores) {
                safe_lengths.advance(lengths);
            }

            Ok(Self {
                path: indexed_path.clone(),
                chain,
                vecs,
                stores,
                safe_lengths,
            })
        };

        match try_import() {
            Ok(result) => Ok(result),
            Err(err) if err.is_lock_error() => {
                // Lock errors are transient - another process has the database open.
                // Don't delete data, just return the error.
                Err(err)
            }
            Err(err) if can_retry && err.is_data_error() => {
                // Data corruption or version mismatch - safe to delete and retry
                info!("{err:?}, deleting {indexed_path:?} and retrying");
                fs::remove_dir_all(&indexed_path)?;
                Self::forced_import_inner(outputs_dir, chain, false)
            }
            Err(err) => Err(err),
        }
    }

    /// Fully resets the indexer by deleting stores from disk and reimporting.
    /// Unlike stores.reset() which uses keyspace.clear() (leaving a journal
    /// record that gets replayed on every recovery), this cleanly recreates.
    fn full_reset(&mut self) -> Result<()> {
        info!("Full reset...");
        self.safe_lengths.reset();
        self.vecs.reset()?;
        let stores_path = self.path.join("stores");
        fs::remove_dir_all(&stores_path).ok();
        self.stores = Stores::forced_import(&self.path, VERSION)?;
        Ok(())
    }

    pub fn index(&mut self, reader: &Reader, client: &Client, exit: &Exit) -> Result<()> {
        self.index_(reader, client, exit, false)
    }

    pub fn checked_index(&mut self, reader: &Reader, client: &Client, exit: &Exit) -> Result<()> {
        self.index_(reader, client, exit, true)
    }

    fn index_(
        &mut self,
        reader: &Reader,
        client: &Client,
        exit: &Exit,
        check_collisions: bool,
    ) -> Result<()> {
        self.vecs.db.sync_bg_tasks()?;

        self.check_xor_bytes(reader)?;

        debug!("Starting indexing...");

        let last_blockhash = self.vecs.blocks.blockhash.collect_last();
        // Rollback sim
        // let last_blockhash = self
        //     .vecs
        //     .blocks
        //     .blockhash
        //     .collect_one_at(self.vecs.blocks.blockhash.len() - 2);
        debug!("Last block hash found.");

        let (mut starting_lengths, mut prev_hash) = if let Some(hash) = last_blockhash {
            let (height, hash) = client.get_closest_valid_height(hash)?;
            match Lengths::resume_at(height.incremented(), &self.vecs, &self.stores) {
                Some(starting_lengths) => {
                    if starting_lengths.height > client.get_last_height()? {
                        info!("Up to date, nothing to index.");
                        return Ok(());
                    }
                    (starting_lengths, Some(hash))
                }
                None => {
                    info!("Data inconsistency detected, resetting indexer...");
                    self.full_reset()?;
                    (Lengths::default(), None)
                }
            }
        } else {
            (Lengths::default(), None)
        };
        debug!("Starting lengths set.");

        let lock = exit.lock();
        self.safe_lengths.lower_before(&starting_lengths);
        self.stores
            .rollback_if_needed(&mut self.vecs, &starting_lengths)?;
        debug!("Rollback stores done.");
        self.vecs.rollback_if_needed(&starting_lengths)?;
        debug!("Rollback vecs done.");
        drop(lock);

        self.recover_incomplete_store(&mut starting_lengths, &mut prev_hash)?;

        let mut lengths = starting_lengths;

        let is_export_height =
            |height: Height| -> bool { height != 0 && height % SNAPSHOT_BLOCK_RANGE == 0 };

        let export = move |stores: &mut Stores, vecs: &mut Vecs, height: Height| -> Result<()> {
            info!("Exporting...");
            let i = Instant::now();
            let _lock = exit.lock();
            thread::scope(|s| -> Result<()> {
                let stores_res = s.spawn(|| -> Result<()> {
                    let i = Instant::now();
                    stores.commit(height)?;
                    debug!("Stores exported in {:?}", i.elapsed());
                    Ok(())
                });
                let vecs_res = s.spawn(|| -> Result<()> {
                    let i = Instant::now();
                    vecs.flush(height)?;
                    debug!("Vecs exported in {:?}", i.elapsed());
                    Ok(())
                });
                stores_res.join().unwrap()?;
                vecs_res.join().unwrap()?;
                Ok(())
            })?;
            info!("Exported in {:?}", i.elapsed());
            Ok(())
        };

        let mut readers = Readers::new(&self.vecs);
        let mut buffers = BlockBuffers::default();

        let vecs = &mut self.vecs;
        let stores = &mut self.stores;

        for block in reader.after(prev_hash)?.iter() {
            let block = match block {
                Ok(block) => block,
                Err(e) => {
                    // The reader hit an unrecoverable mid-stream issue
                    // (chain break, parse failure, missing blocks).
                    // Stop cleanly so what we've already indexed gets
                    // flushed in the post-loop export — the next
                    // `index` call will resume from the new tip.
                    error!("Reader stream stopped early: {e}");
                    break;
                }
            };
            let height = block.height();

            if unlikely(height.is_multiple_of(100)) {
                info!("Indexing block {height}...");
            } else {
                debug!("Indexing block {height}...");
            }

            lengths.height = height;

            vecs.blocks.position.push(block.metadata().position());
            block.tx_metadata().iter().for_each(|m| {
                vecs.transactions.position.push(m.position());
            });

            let mut processor = BlockProcessor {
                block: &block,
                height,
                chain: self.chain,
                check_collisions,
                lengths: &mut lengths,
                vecs,
                stores,
                readers: &readers,
            };

            processor.process_block_metadata()?;

            let txs = processor.compute_txids()?;

            processor.push_block_size_and_weight(&txs)?;

            let (txins_result, txouts_result) = rayon::join(
                || processor.process_inputs(&txs, &mut buffers.txid_prefix_map),
                || processor.process_outputs(),
            );
            let txins = txins_result?;
            let txouts = txouts_result?;

            let tx_count = block.txdata.len();
            let input_count = txins.len();
            let output_count = txouts.len();

            BlockProcessor::collect_same_block_spent_outpoints(
                &txins,
                &mut buffers.same_block_spent,
            );

            processor.check_txid_collisions(&txs)?;

            let sigops = processor.compute_sigops(&txins, &txouts);

            processor.finalize_and_store_metadata(
                txs,
                txouts,
                txins,
                sigops,
                &buffers.same_block_spent,
                &mut buffers.already_added_addrs,
                &mut buffers.same_block_output_info,
            )?;

            processor
                .lengths
                .add_block(tx_count, input_count, output_count);

            if is_export_height(height) {
                drop(readers);
                export(stores, vecs, height)?;
                readers = Readers::new(vecs);
            }
        }

        drop(readers);

        let lock = exit.lock();
        // Commit stores before stamping vecs so a restart cannot observe a
        // higher vec bound with store meta/data still from the prior block.
        self.stores.commit(lengths.height)?;
        self.vecs.stamped_write(lengths.height)?;
        let fjall_db = self.stores.db.clone();

        self.vecs.db.run_bg(move |db| {
            let _lock = lock;

            info!("Compacting...");
            let i = Instant::now();
            fjall_db
                .persist(PersistMode::SyncData)
                .map_err(RawDBError::other)?;
            db.compact()?;
            info!("Compacted in {:?}", i.elapsed());
            Ok(())
        });

        Ok(())
    }

    /// If indexed blocks' txids are absent from the store (e.g. the process
    /// restarted after store meta was exported but before ingest), roll back
    /// until the last complete block so the next `index` pass can re-process.
    fn recover_incomplete_store(
        &mut self,
        starting_lengths: &mut Lengths,
        prev_hash: &mut Option<BlockHash>,
    ) -> Result<()> {
        let mut rolled_back = 0_u32;

        while self.last_block_txids_missing_from_store(starting_lengths)? {
            let Some(last_height) = starting_lengths.height.decremented() else {
                break;
            };

            info!(
                "Store missing txids for block {last_height}; rolling back one block to re-index"
            );

            *starting_lengths = Lengths::collect_at(last_height, &self.vecs)
                .ok_or(brk_error::Error::Internal("Cannot roll back lengths"))?;
            self.safe_lengths.lower_before(starting_lengths);
            self.stores
                .rollback_if_needed(&mut self.vecs, starting_lengths)?;
            self.vecs.rollback_if_needed(starting_lengths)?;

            *prev_hash = if last_height.is_zero() {
                None
            } else {
                Some(
                    self.vecs
                        .blocks
                        .blockhash
                        .collect_one(last_height.decremented().unwrap())
                        .ok_or(brk_error::Error::Internal("Missing rollback blockhash"))?,
                )
            };

            rolled_back += 1;
        }

        if rolled_back > 0 {
            info!("Store recovery: rolled back {rolled_back} block(s), resuming at {}", starting_lengths.height);
        }

        Ok(())
    }

    fn last_block_txids_missing_from_store(&self, lengths: &Lengths) -> Result<bool> {
        let Some(last_height) = lengths.height.decremented() else {
            return Ok(false);
        };

        let start_tx = self
            .vecs
            .transactions
            .first_tx_index
            .collect_one(last_height)
            .ok_or(brk_error::Error::Internal("Missing first_tx_index"))?;
        let end_tx = lengths.tx_index.to_usize();
        let txid_reader = self.vecs.transactions.txid.reader();

        for tx_index in start_tx.to_usize()..end_tx {
            let txid = txid_reader.get(tx_index);
            let prefix = TxidPrefix::from(&txid);
            if self
                .stores
                .txid_prefix_to_tx_index
                .get(&prefix)?
                .is_none()
            {
                return Ok(true);
            }
        }

        Ok(false)
    }

    fn check_xor_bytes(&mut self, reader: &Reader) -> Result<()> {
        let current = reader.xor_bytes();
        let cached = XORBytes::from(self.path.as_path());

        if cached == current {
            return Ok(());
        }

        self.full_reset()?;

        fs::write(self.path.join("xor.dat"), *current)?;

        Ok(())
    }

    /// Publish disk state as the new safe-lengths snapshot. Drains pending
    /// bg ingest first so stores are queryable at the new bound.
    pub fn advance_safe_lengths(&mut self) -> Result<()> {
        self.vecs.db.sync_bg_tasks()?;
        if let Some(lengths) = Lengths::from_local(&self.vecs, &self.stores) {
            self.safe_lengths.advance(lengths);
        }
        Ok(())
    }
}

impl ReadOnlyClone for Indexer {
    type ReadOnly = Indexer<Ro>;

    fn read_only_clone(&self) -> Indexer<Ro> {
        Indexer {
            path: self.path.clone(),
            chain: self.chain,
            vecs: self.vecs.read_only_clone(),
            stores: self.stores.clone(),
            safe_lengths: self.safe_lengths.clone(),
        }
    }
}
