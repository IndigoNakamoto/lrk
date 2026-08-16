// Copyright (c) 2024-present, fjall-rs
// This source code is licensed under both the Apache 2.0 and MIT License
// (found in the LICENSE-* files in the repository)

use crate::{
    InternalValue, Table,
    compaction::{Input as CompactionPayload, worker::Options},
    file::TABLES_FOLDER,
    table::multi_writer::MultiWriter,
    version::{SuperVersions, Version},
};
use std::time::Instant;

pub(super) fn prepare_table_writer(
    version: &Version,
    opts: &Options,
    payload: &CompactionPayload,
) -> crate::Result<MultiWriter> {
    let table_base_folder = opts.config.path.join(TABLES_FOLDER);
    let dst_lvl = payload.canonical_level.into();
    let data_block_size = opts.config.data_block_size_policy.get(dst_lvl);
    let data_block_restart_interval = opts.config.data_block_restart_interval_policy.get(dst_lvl);
    let index_block_restart_interval = opts.config.index_block_restart_interval_policy.get(dst_lvl);
    let data_block_compression = opts.config.data_block_compression_policy.get(dst_lvl);
    let index_block_compression = opts.config.index_block_compression_policy.get(dst_lvl);
    let data_block_hash_ratio = opts.config.data_block_hash_ratio_policy.get(dst_lvl);
    let index_partitioning = opts.config.index_block_partitioning_policy.get(dst_lvl);
    let filter_partitioning = opts.config.filter_block_partitioning_policy.get(dst_lvl);

    log::debug!(
        "Compacting tables {:?} into L{} (canonical L{}), target_size={}, data_block_restart_interval={data_block_restart_interval}, index_block_restart_interval={index_block_restart_interval}, data_block_size={data_block_size}, data_block_compression={data_block_compression:?}, index_block_compression={index_block_compression:?}, mvcc_gc_watermark={}",
        payload.table_ids,
        payload.dest_level,
        payload.canonical_level,
        payload.target_size,
        opts.mvcc_gc_watermark,
    );

    let mut table_writer = MultiWriter::new(
        table_base_folder,
        opts.table_id_generator.clone(),
        payload.target_size,
        payload.dest_level,
    )?;

    if index_partitioning {
        table_writer = table_writer.use_partitioned_index();
    }
    if filter_partitioning {
        table_writer = table_writer.use_partitioned_filter();
    }

    #[expect(clippy::cast_possible_truncation, reason = "max key size = u16")]
    let last_level = (version.level_count() - 1) as u8;
    let is_last_level = payload.dest_level == last_level;

    Ok(table_writer
        .use_data_block_restart_interval(data_block_restart_interval)
        .use_index_block_restart_interval(index_block_restart_interval)
        .use_data_block_compression(data_block_compression)
        .use_data_block_size(data_block_size)
        .use_data_block_hash_ratio(data_block_hash_ratio)
        .use_index_block_compression(index_block_compression)
        .use_bloom_policy({
            use crate::config::FilterPolicyEntry::{Bloom, None};
            use crate::table::filter::BloomConstructionPolicy;

            if is_last_level && opts.config.expect_point_read_hits {
                BloomConstructionPolicy::BitsPerKey(0.0)
            } else {
                match opts
                    .config
                    .filter_policy
                    .get(usize::from(payload.dest_level))
                {
                    Bloom(policy) => policy,
                    None => BloomConstructionPolicy::BitsPerKey(0.0),
                }
            }
        }))
}

pub(super) struct StandardCompaction {
    start: Instant,
    table_writer: MultiWriter,
    tables_to_rewrite: Vec<Table>,
}

impl StandardCompaction {
    pub fn new(table_writer: MultiWriter, tables_to_rewrite: Vec<Table>) -> Self {
        Self {
            start: Instant::now(),
            table_writer,
            tables_to_rewrite,
        }
    }

    pub fn write(&mut self, item: InternalValue) -> crate::Result<()> {
        self.table_writer.write(item)
    }

    fn consume_writer(self, opts: &Options, dst_lvl: usize) -> crate::Result<Vec<Table>> {
        let pin_filter = opts.config.filter_block_pinning_policy.get(dst_lvl);
        let pin_index = opts.config.index_block_pinning_policy.get(dst_lvl);
        let (table_base_folder, results) = self.table_writer.finish()?;

        results
            .into_iter()
            .map(|(table_id, checksum)| {
                Table::recover(
                    table_base_folder.join(table_id.to_string()),
                    checksum,
                    0,
                    opts.tree_id,
                    opts.config.cache.clone(),
                    opts.config.descriptor_table.clone(),
                    pin_filter,
                    pin_index,
                )
            })
            .collect()
    }

    pub fn finish(
        mut self,
        super_version: &mut SuperVersions,
        opts: &Options,
        payload: &CompactionPayload,
        dst_lvl: usize,
    ) -> crate::Result<()> {
        log::debug!("Compaction done in {:?}", self.start.elapsed());

        let tables_to_delete = std::mem::take(&mut self.tables_to_rewrite);
        let created_tables = self.consume_writer(opts, dst_lvl)?;

        super_version.upgrade_version(
            &opts.config.path,
            |current| {
                let mut copy = current.clone();
                copy.version = copy.version.with_merge(
                    &payload.table_ids.iter().copied().collect::<Vec<_>>(),
                    &created_tables,
                    payload.dest_level as usize,
                );
                Ok(copy)
            },
            &opts.global_seqno,
            &opts.visible_seqno,
        )?;

        for table in tables_to_delete {
            table.mark_as_deleted();
        }

        Ok(())
    }
}
