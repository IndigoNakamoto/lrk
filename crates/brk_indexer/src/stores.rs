use std::{fs, path::Path, time::Instant};

use rustc_hash::FxHashSet;

use brk_cohort::ByAddrType;
use brk_error::{Error, Result};
use brk_store::{AnyStore, Kind, Mode, Store};
use brk_types::{
    AddrHash, AddrIndexOutPoint, AddrIndexTxIndex, BlockHashPrefix, Height, OutPoint, OutputType,
    TxIndex, TxOutIndex, TxidPrefix, TypeIndex, Unit, Version, Vout,
};
use fjall::{Database, PersistMode};
use rayon::prelude::*;
use tracing::{debug, info};
use vecdb::{AnyVec, ReadableVec, VecIndex};

use crate::{Lengths, constants::DUPLICATE_TXID_PREFIXES};

use super::Vecs;

#[derive(Clone)]
pub struct Stores {
    pub db: Database,

    pub addr_type_to_addr_hash_to_addr_index: ByAddrType<Store<AddrHash, TypeIndex>>,
    pub addr_type_to_addr_index_and_tx_index: ByAddrType<Store<AddrIndexTxIndex, Unit>>,
    pub addr_type_to_addr_index_and_unspent_outpoint: ByAddrType<Store<AddrIndexOutPoint, Unit>>,
    pub blockhash_prefix_to_height: Store<BlockHashPrefix, Height>,
    pub txid_prefix_to_tx_index: Store<TxidPrefix, TxIndex>,
}

impl Stores {
    pub fn forced_import(parent: &Path, version: Version) -> Result<Self> {
        Self::forced_import_inner(parent, version, true)
    }

    fn forced_import_inner(parent: &Path, version: Version, can_retry: bool) -> Result<Self> {
        let pathbuf = parent.join("stores");
        let path = pathbuf.as_path();

        fs::create_dir_all(&pathbuf)?;

        let database = match brk_store::open_database(path) {
            Ok(database) => database,
            Err(err) if can_retry => {
                info!("Failed to open stores at {path:?}: {err:?}, deleting and retrying");
                fs::remove_dir_all(path)?;
                return Self::forced_import_inner(parent, version, false);
            }
            Err(err) => return Err(err.into()),
        };

        let database_ref = &database;

        let create_addr_hash_to_addr_index_store = |index| {
            Store::import(
                database_ref,
                path,
                &format!("h2i{}", index),
                version,
                Mode::PushOnly,
                Kind::Random,
            )
        };

        let create_addr_index_to_tx_index_store = |index| {
            Store::import(
                database_ref,
                path,
                &format!("a2t{}", index),
                version,
                Mode::PushOnly,
                Kind::Vec,
            )
        };

        let create_addr_index_to_unspent_outpoint_store = |index| {
            Store::import(
                database_ref,
                path,
                &format!("a2u{}", index),
                version,
                Mode::Any,
                Kind::Vec,
            )
        };

        let stores = Self {
            db: database.clone(),

            addr_type_to_addr_hash_to_addr_index: ByAddrType::new_with_index(
                create_addr_hash_to_addr_index_store,
            )?,
            addr_type_to_addr_index_and_tx_index: ByAddrType::new_with_index(
                create_addr_index_to_tx_index_store,
            )?,
            addr_type_to_addr_index_and_unspent_outpoint: ByAddrType::new_with_index(
                create_addr_index_to_unspent_outpoint_store,
            )?,
            blockhash_prefix_to_height: Store::import(
                database_ref,
                path,
                "blockhash_prefix_to_height",
                version,
                Mode::PushOnly,
                Kind::Random,
            )?,
            txid_prefix_to_tx_index: Store::import_cached(
                database_ref,
                path,
                "txid_prefix_to_tx_index",
                version,
                Mode::PushOnly,
                Kind::Recent,
                5,
            )?,
        };

        Ok(stores)
    }

    pub fn next_height(&self) -> Height {
        self.iter_any()
            .map(|store| store.height().map(Height::incremented).unwrap_or_default())
            .min()
            .unwrap()
    }

    fn iter_any(&self) -> impl Iterator<Item = &dyn AnyStore> {
        [
            &self.blockhash_prefix_to_height as &dyn AnyStore,
            &self.txid_prefix_to_tx_index,
        ]
        .into_iter()
        .chain(
            self.addr_type_to_addr_hash_to_addr_index
                .values()
                .map(|s| s as &dyn AnyStore),
        )
        .chain(
            self.addr_type_to_addr_index_and_tx_index
                .values()
                .map(|s| s as &dyn AnyStore),
        )
        .chain(
            self.addr_type_to_addr_index_and_unspent_outpoint
                .values()
                .map(|s| s as &dyn AnyStore),
        )
    }

    fn par_iter_any_mut(&mut self) -> impl ParallelIterator<Item = &mut dyn AnyStore> {
        [
            &mut self.blockhash_prefix_to_height as &mut dyn AnyStore,
            &mut self.txid_prefix_to_tx_index,
        ]
        .into_par_iter()
        .chain(
            self.addr_type_to_addr_hash_to_addr_index
                .par_values_mut()
                .map(|s| s as &mut dyn AnyStore),
        )
        .chain(
            self.addr_type_to_addr_index_and_tx_index
                .par_values_mut()
                .map(|s| s as &mut dyn AnyStore),
        )
        .chain(
            self.addr_type_to_addr_index_and_unspent_outpoint
                .par_values_mut()
                .map(|s| s as &mut dyn AnyStore),
        )
    }

    pub fn commit(&mut self, height: Height) -> Result<()> {
        let i = Instant::now();
        self.par_iter_any_mut()
            .try_for_each(|store| store.commit(height))?;
        debug!("Stores committed in {:?}", i.elapsed());

        let i = Instant::now();
        self.db.persist(PersistMode::SyncData)?;
        debug!("Stores persisted in {:?}", i.elapsed());

        Ok(())
    }

    /// Takes all pending puts/dels from every store and returns closures
    /// that can ingest them on a background thread.
    #[allow(clippy::type_complexity)]
    pub fn take_all_pending_ingests(
        &mut self,
        height: Height,
    ) -> Result<Vec<Box<dyn FnOnce() -> Result<()> + Send>>> {
        let h = height;
        let mut tasks = Vec::new();

        macro_rules! take {
            ($store:expr) => {
                tasks.extend($store.take_pending_ingest(h)?);
            };
        }

        take!(self.blockhash_prefix_to_height);
        take!(self.txid_prefix_to_tx_index);

        for store in self.addr_type_to_addr_hash_to_addr_index.values_mut() {
            take!(store);
        }
        for store in self.addr_type_to_addr_index_and_tx_index.values_mut() {
            take!(store);
        }
        for store in self
            .addr_type_to_addr_index_and_unspent_outpoint
            .values_mut()
        {
            take!(store);
        }

        Ok(tasks)
    }

    /// Rewrites reverse-key entries below the lowered bound. In-flight
    /// readers may briefly see torn state.
    pub fn rollback_if_needed(
        &mut self,
        vecs: &mut Vecs,
        starting_lengths: &Lengths,
    ) -> Result<()> {
        if self.is_empty()? {
            return Ok(());
        }

        debug_assert!(starting_lengths.height != Height::ZERO);
        debug_assert!(starting_lengths.tx_index != TxIndex::ZERO);
        debug_assert!(starting_lengths.txout_index != TxOutIndex::ZERO);

        self.rollback_block_metadata(vecs, starting_lengths)?;
        self.rollback_txids(vecs, starting_lengths);
        self.rollback_outputs_and_inputs(vecs, starting_lengths)?;

        let rollback_height = starting_lengths.height.decremented().unwrap_or_default();
        self.par_iter_any_mut()
            .try_for_each(|store| store.export_meta(rollback_height))?;
        self.commit(rollback_height)?;

        Ok(())
    }

    fn is_empty(&self) -> Result<bool> {
        Ok(self.blockhash_prefix_to_height.is_empty()?
            && self.txid_prefix_to_tx_index.is_empty()?
            && self
                .addr_type_to_addr_hash_to_addr_index
                .values()
                .try_fold(true, |acc, s| s.is_empty().map(|empty| acc && empty))?
            && self
                .addr_type_to_addr_index_and_tx_index
                .values()
                .try_fold(true, |acc, s| s.is_empty().map(|empty| acc && empty))?
            && self
                .addr_type_to_addr_index_and_unspent_outpoint
                .values()
                .try_fold(true, |acc, s| s.is_empty().map(|empty| acc && empty))?)
    }

    fn rollback_block_metadata(
        &mut self,
        vecs: &mut Vecs,
        starting_lengths: &Lengths,
    ) -> Result<()> {
        vecs.blocks.blockhash.for_each_range_at(
            starting_lengths.height.to_usize(),
            vecs.blocks.blockhash.len(),
            |blockhash| {
                self.blockhash_prefix_to_height
                    .remove(BlockHashPrefix::from(blockhash));
            },
        );

        for addr_type in OutputType::ADDR_TYPES {
            for hash in vecs.iter_addr_hashes_from(addr_type, starting_lengths.height)? {
                self.addr_type_to_addr_hash_to_addr_index
                    .get_mut_unwrap(addr_type)
                    .remove(hash);
            }
        }

        Ok(())
    }

    fn rollback_txids(&mut self, vecs: &mut Vecs, starting_lengths: &Lengths) {
        let start = starting_lengths.tx_index.to_usize();
        let end = vecs.transactions.txid.len();
        let mut current_index = start;
        vecs.transactions
            .txid
            .for_each_range_at(start, end, |txid| {
                let tx_index = TxIndex::from(current_index);
                let txid_prefix = TxidPrefix::from(&txid);

                let is_known_dup =
                    DUPLICATE_TXID_PREFIXES
                        .iter()
                        .any(|(dup_prefix, dup_tx_index)| {
                            tx_index == *dup_tx_index && txid_prefix == *dup_prefix
                        });

                if !is_known_dup {
                    self.txid_prefix_to_tx_index.remove(txid_prefix);
                }
                current_index += 1;
            });

        self.txid_prefix_to_tx_index.clear_caches();
    }

    fn rollback_outputs_and_inputs(
        &mut self,
        vecs: &mut Vecs,
        starting_lengths: &Lengths,
    ) -> Result<()> {
        let tx_index_to_first_txout_index_reader = vecs.transactions.first_txout_index.reader();
        let txout_index_to_output_type_reader = vecs.outputs.output_type.reader();
        let txout_index_to_type_index_reader = vecs.outputs.type_index.reader();

        let mut addr_index_tx_index_to_remove: FxHashSet<(OutputType, TypeIndex, TxIndex)> =
            FxHashSet::default();

        let rollback_start = starting_lengths.txout_index.to_usize();
        let rollback_end = vecs.outputs.output_type.len();

        let starting_tx_index = starting_lengths.tx_index;
        let first_txout_indexes = vecs.transactions.first_txout_index.collect_range_at(
            starting_tx_index.to_usize(),
            vecs.transactions.first_txout_index.len(),
        );

        if !valid_rollback_boundaries(&first_txout_indexes, rollback_start, rollback_end) {
            return Err(Error::Internal("Invalid rollback output boundaries"));
        }

        for (tx_index, txout_range) in txout_ranges(
            starting_tx_index,
            &first_txout_indexes,
            TxOutIndex::from(rollback_end),
        ) {
            for (vout, txout_index) in txout_range.enumerate() {
                let output_type = txout_index_to_output_type_reader.get_at(txout_index);
                if !output_type.is_addr() {
                    continue;
                }

                let addr_type = output_type;
                let addr_index = txout_index_to_type_index_reader.get_at(txout_index);

                addr_index_tx_index_to_remove.insert((addr_type, addr_index, tx_index));

                let outpoint = OutPoint::new(tx_index, Vout::from(vout));

                self.addr_type_to_addr_index_and_unspent_outpoint
                    .get_mut_unwrap(addr_type)
                    .remove(AddrIndexOutPoint::from((addr_index, outpoint)));
            }
        }

        let start = starting_lengths.txin_index.to_usize();
        let end = vecs.inputs.outpoint.len();
        let outpoints: Vec<OutPoint> = vecs.inputs.outpoint.collect_range_at(start, end);
        let spending_tx_indexes: Vec<TxIndex> = vecs.inputs.tx_index.collect_range_at(start, end);

        let outputs_to_unspend: Vec<_> = outpoints
            .into_iter()
            .zip(spending_tx_indexes)
            .filter_map(|(outpoint, spending_tx_index)| {
                if outpoint.is_coinbase() {
                    return None;
                }

                let output_tx_index = outpoint.tx_index();
                let vout = outpoint.vout();
                let txout_index = tx_index_to_first_txout_index_reader.get(output_tx_index) + vout;

                if txout_index < starting_lengths.txout_index {
                    let output_type = txout_index_to_output_type_reader.get(txout_index);
                    let type_index = txout_index_to_type_index_reader.get(txout_index);
                    Some((outpoint, output_type, type_index, spending_tx_index))
                } else {
                    None
                }
            })
            .collect();

        for (outpoint, output_type, type_index, spending_tx_index) in outputs_to_unspend {
            if output_type.is_addr() {
                let addr_type = output_type;
                let addr_index = type_index;

                addr_index_tx_index_to_remove.insert((addr_type, addr_index, spending_tx_index));

                self.addr_type_to_addr_index_and_unspent_outpoint
                    .get_mut_unwrap(addr_type)
                    .insert(AddrIndexOutPoint::from((addr_index, outpoint)), Unit);
            }
        }

        for (addr_type, addr_index, tx_index) in addr_index_tx_index_to_remove {
            self.addr_type_to_addr_index_and_tx_index
                .get_mut_unwrap(addr_type)
                .remove(AddrIndexTxIndex::from((addr_index, tx_index)));
        }

        Ok(())
    }
}

fn valid_rollback_boundaries(
    first_txout_indexes: &[TxOutIndex],
    rollback_start: usize,
    rollback_end: usize,
) -> bool {
    if rollback_start > rollback_end {
        return false;
    }

    let Some(first) = first_txout_indexes.first() else {
        return rollback_start == rollback_end;
    };

    first.to_usize() == rollback_start
        && first_txout_indexes
            .windows(2)
            .all(|pair| pair[0] <= pair[1])
        && first_txout_indexes
            .last()
            .is_some_and(|last| last.to_usize() <= rollback_end)
}

fn txout_ranges(
    starting_tx_index: TxIndex,
    first_txout_indexes: &[TxOutIndex],
    rollback_end: TxOutIndex,
) -> impl Iterator<Item = (TxIndex, std::ops::Range<usize>)> + '_ {
    first_txout_indexes
        .iter()
        .copied()
        .enumerate()
        .map(move |(offset, first)| {
            let end = first_txout_indexes
                .get(offset + 1)
                .copied()
                .unwrap_or(rollback_end);
            (starting_tx_index + offset, first.to_usize()..end.to_usize())
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollback_output_ranges_reconstruct_tx_indexes_and_vouts() {
        let first_txout_indexes = [100_usize, 103, 103, 105].map(TxOutIndex::from);
        let ranges: Vec<_> = txout_ranges(
            TxIndex::from(40_usize),
            &first_txout_indexes,
            TxOutIndex::from(108_usize),
        )
        .collect();

        assert_eq!(
            ranges,
            [
                (TxIndex::from(40_usize), 100..103),
                (TxIndex::from(41_usize), 103..103),
                (TxIndex::from(42_usize), 103..105),
                (TxIndex::from(43_usize), 105..108),
            ]
        );

        let reconstructed: Vec<_> = ranges
            .into_iter()
            .flat_map(|(tx_index, range)| {
                range
                    .enumerate()
                    .map(move |(vout, txout_index)| (txout_index, tx_index, Vout::from(vout)))
            })
            .collect();

        assert_eq!(
            reconstructed,
            [
                (100, TxIndex::from(40_usize), Vout::from(0_usize)),
                (101, TxIndex::from(40_usize), Vout::from(1_usize)),
                (102, TxIndex::from(40_usize), Vout::from(2_usize)),
                (103, TxIndex::from(42_usize), Vout::from(0_usize)),
                (104, TxIndex::from(42_usize), Vout::from(1_usize)),
                (105, TxIndex::from(43_usize), Vout::from(0_usize)),
                (106, TxIndex::from(43_usize), Vout::from(1_usize)),
                (107, TxIndex::from(43_usize), Vout::from(2_usize)),
            ]
        );
    }

    #[test]
    fn rollback_output_boundaries_are_validated() {
        assert!(valid_rollback_boundaries(
            &[TxOutIndex::from(100_usize), TxOutIndex::from(103_usize)],
            100,
            105,
        ));
        assert!(valid_rollback_boundaries(&[], 100, 100));

        assert!(!valid_rollback_boundaries(
            &[TxOutIndex::from(99_usize)],
            100,
            105,
        ));
        assert!(!valid_rollback_boundaries(
            &[TxOutIndex::from(100_usize), TxOutIndex::from(99_usize)],
            100,
            105,
        ));
        assert!(!valid_rollback_boundaries(
            &[TxOutIndex::from(100_usize), TxOutIndex::from(106_usize)],
            100,
            105,
        ));
        assert!(!valid_rollback_boundaries(&[], 100, 101));
    }
}
