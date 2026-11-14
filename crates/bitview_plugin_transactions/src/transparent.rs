use bitview_compute::{prepare_computed, write_locked};
use bitview_plugin_indexer::Indexer;
use bitview_plugin_mappings::Vecs as MappingsVecs;
use bitview_primitives::Count;
use bitview_vecs::{CachedSeries, import_cached};
use brk_error::{OptionData, Result};
use brk_exit::Exit;
use brk_types::{Height, Version};
use vecdb::{AnyStoredVec, AnyVec, Database, ReadableVec, WritableVec};

/// Cumulative transactions without the HogEx, Litecoin's per-block MWEB settlement: a protocol
/// transaction, not a payment, so transaction counts and rates leave it out.
pub(crate) fn import(db: &Database, version: Version) -> Result<CachedSeries<Height, Count>> {
    import_cached(db, "transparent_tx_count_cumulative", version)
}

pub(crate) fn compute(
    target: &mut CachedSeries<Height, Count>,
    indexer: &Indexer,
    mappings: &MappingsVecs,
    exit: &Exit,
) -> Result<()> {
    let tx_counts = &mappings.height.tx_index_count;
    let hogex = &indexer.vecs().transactions.features.hogex.block;
    let version = tx_counts.version() + hogex.version();
    let start = prepare_computed(
        [target as &mut dyn AnyStoredVec],
        version,
        usize::from(indexer.compute_start().height),
        exit,
    )?;
    let end = hogex.len();
    if start < end {
        let mut cumulative = target.collect_last().unwrap_or_default();
        let mut tx_counts = tx_counts.bounded_cursor_at(start, end);
        let mut hogex = hogex.bounded_cursor_at(start, end);
        for _ in start..end {
            let txs = u64::from(tx_counts.next().data()?);
            let hogex = u64::from(Count::from(hogex.next().data()?));
            cumulative += Count::from(txs - hogex);
            target.push(cumulative);
        }
    }
    write_locked([target as &mut dyn AnyStoredVec], exit)
}
