// Copyright (c) 2024-present, fjall-rs
// This source code is licensed under both the Apache 2.0 and MIT License
// (found in the LICENSE-* files in the repository)

use crate::{Checksum, SeqNo, TableId, file::CURRENT_VERSION_FILE, version::VersionId};
use byteorder::{LittleEndian, ReadBytesExt};
use std::path::Path;

pub fn get_current_version(folder: &std::path::Path) -> crate::Result<VersionId> {
    use byteorder::{LittleEndian, ReadBytesExt};

    std::fs::File::open(folder.join(CURRENT_VERSION_FILE))
        .and_then(|mut f| f.read_u64::<LittleEndian>())
        .map_err(Into::into)
}

pub struct RecoveredTable {
    pub id: TableId,
    pub checksum: Checksum,
    pub global_seqno: SeqNo,
}

pub struct Recovery {
    pub curr_version_id: VersionId,
    pub table_ids: Vec<Vec<Vec<RecoveredTable>>>,
}

pub fn recover(folder: &Path) -> crate::Result<Recovery> {
    let curr_version_id = get_current_version(folder)?;
    let version_file_path = folder.join(format!("v{curr_version_id}"));

    // TODO: maybe validate current version using the checksum in "current"

    log::info!(
        "Recovering current manifest at {}",
        version_file_path.display(),
    );

    let reader = sfa::Reader::new(&version_file_path)?;
    let toc = reader.toc();

    // // TODO: vvv move into Version::decode vvv
    let mut levels = vec![];

    {
        let mut reader = toc
            .section(b"tables")
            .ok_or(crate::Error::Unrecoverable)
            .inspect_err(|_| {
                log::error!("tables section not found in version #{curr_version_id} - maybe the file is corrupted?");
            })?
            .buf_reader(&version_file_path)?;

        let level_count = reader.read_u8()?;

        for _ in 0..level_count {
            let mut level = vec![];
            let run_count = reader.read_u8()?;

            for _ in 0..run_count {
                let mut run = vec![];
                let table_count = reader.read_u32::<LittleEndian>()?;

                for _ in 0..table_count {
                    let id = reader.read_u32::<LittleEndian>()?;
                    let checksum_type = reader.read_u8()?;

                    if checksum_type != 0 {
                        return Err(crate::Error::InvalidTag(("ChecksumType", checksum_type)));
                    }

                    let checksum = reader.read_u128::<LittleEndian>()?;
                    let checksum = Checksum::from_raw(checksum);

                    let global_seqno = reader.read_u64::<LittleEndian>()?;

                    run.push(RecoveredTable {
                        id,
                        checksum,
                        global_seqno,
                    });
                }

                level.push(run);
            }

            levels.push(level);
        }
    }

    let tree_type = {
        let byte = toc
            .section(b"tree_type")
            .ok_or(crate::Error::Unrecoverable)
            .inspect_err(|_| {
                log::error!("tree_type section not found in version #{curr_version_id} - maybe the file is corrupted?");
            })?
            .buf_reader(&version_file_path)?
            .read_u8()?;

        byte
    };

    if tree_type != 0 {
        log::error!("Blob trees are not supported by this build");
        return Err(crate::Error::Unrecoverable);
    }

    Ok(Recovery {
        curr_version_id,
        table_ids: levels,
    })
}
