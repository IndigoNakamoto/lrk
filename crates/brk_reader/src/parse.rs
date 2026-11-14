use std::io::Cursor;

use bitcoin::{Block as BitcoinBlock, Transaction, VarInt, block::Header, consensus::Decodable};
use brk_error::{Error, Result};
use brk_types::{BlkPosition, BlockHash, Height, ReadBlock};

use crate::{XORBytes, XORIndex, canonical::CanonicalRange};

pub const HEADER_LEN: usize = 80;

/// The canonical offset and header of the block in `bytes`, `None` for any other block. A block
/// of the range that does not follow its canonical predecessor is an error: the range was resolved
/// across a reorg. Decodes the header onto a stack buffer so `bytes` stays untouched: the body
/// parse later re-XORs the full block from the original phase.
pub fn peek_canonical(
    bytes: &[u8],
    mut xor_state: XORIndex,
    xor_bytes: XORBytes,
    canonical: &CanonicalRange,
) -> Result<Option<(u32, Header)>> {
    if bytes.len() < HEADER_LEN {
        return Ok(None);
    }
    let mut header_buf = [0u8; HEADER_LEN];
    header_buf.copy_from_slice(&bytes[..HEADER_LEN]);
    xor_state.bytes(&mut header_buf, xor_bytes);
    let header = Header::consensus_decode_from_finite_reader(&mut &header_buf[..])
        .expect("an 80-byte header always decodes");
    let Some(offset) = canonical.offset_of(&BlockHash::from(header.block_hash())) else {
        return Ok(None);
    };
    if !canonical.verify_prev(offset, &BlockHash::from(header.prev_blockhash)) {
        return Err(Error::Internal("canonical batch stitched across a reorg"));
    }
    Ok(Some((offset, header)))
}

pub fn parse_canonical_body(
    mut bytes: Vec<u8>,
    position: BlkPosition,
    mut xor_state: XORIndex,
    xor_bytes: XORBytes,
    height: Height,
    header: Header,
) -> Result<ReadBlock> {
    debug_assert!(bytes.len() >= HEADER_LEN);
    xor_state.bytes(&mut bytes, xor_bytes);
    let bitcoin_hash = header.block_hash();

    let mut cursor = Cursor::new(bytes);
    cursor.set_position(HEADER_LEN as u64);

    // `from_finite_reader` skips the `Take<R>` wrap that
    // `consensus_decode` applies to every nested field for memory
    // safety: our cursor is already a bounded `Vec<u8>`, so the
    // wrapping is pure overhead and compounds across ~2000 tx fields.
    let tx_count = VarInt::consensus_decode_from_finite_reader(&mut cursor)?.0 as usize;
    let mut txdata = Vec::with_capacity(tx_count);
    let mut tx_offsets = Vec::with_capacity(tx_count);
    for _ in 0..tx_count {
        tx_offsets.push(cursor.position() as u32);
        let tx = Transaction::consensus_decode_from_finite_reader(&mut cursor)?;
        txdata.push(tx);
    }

    #[cfg(not(feature = "litecoin"))]
    let block = BitcoinBlock { header, txdata };
    #[cfg(feature = "litecoin")]
    let (block, mweb_decode_failed) = {
        let (mweb_block, failed) = mweb::decode_extension(&mut cursor, height, &header, &txdata)?;
        (
            BitcoinBlock {
                header,
                txdata,
                mweb_block,
            },
            failed,
        )
    };

    let read = ReadBlock::new(
        height,
        BlockHash::from(bitcoin_hash),
        block,
        position,
        cursor.into_inner(),
        tx_offsets,
    );
    #[cfg(feature = "litecoin")]
    let read = read.with_mweb_decode_failed(mweb_decode_failed);
    Ok(read)
}

#[cfg(feature = "litecoin")]
mod mweb {
    use std::io::Cursor;

    use bitcoin::{Transaction, block::Header, block::MwebBlock, consensus::Decodable};
    use brk_error::{Error, Result};
    use brk_types::{
        Height,
        litecoin::{MWEB_ACTIVATION_HEIGHT, MWEB_VERSION_BIT, mweb_strict},
    };
    use tracing::{error, warn};

    /// Decodes the MWEB extension block that follows the transactions of an MWEB block, returning
    /// it and whether one was signalled but failed to decode. Structural violations (a HogEx before
    /// activation or not last) are errors; a failed extension decode only is under `mweb_strict`,
    /// since every transparent-side series (HogAddr supply, peg flows) survives without it.
    pub fn decode_extension(
        cursor: &mut Cursor<Vec<u8>>,
        height: Height,
        header: &Header,
        txdata: &[Transaction],
    ) -> Result<(Option<MwebBlock>, bool)> {
        let last = txdata.len().saturating_sub(1);
        if txdata[..last].iter().any(|tx| tx.is_hog_ex) {
            return Err(Error::Internal("HogEx is not the last transaction"));
        }
        let has_hogex = txdata.len() >= 2 && txdata[last].is_hog_ex;
        if height < MWEB_ACTIVATION_HEIGHT {
            if has_hogex {
                return Err(Error::Internal("HogEx before MWEB activation"));
            }
            return Ok((None, false));
        }
        if !has_hogex {
            error!("block {height}: no HogEx after MWEB activation");
            if mweb_strict() {
                return Err(Error::Internal("missing HogEx after MWEB activation"));
            }
            return Ok((None, false));
        }
        if header.version.to_consensus() & MWEB_VERSION_BIT == 0 {
            warn!("block {height}: HogEx without the MWEB version bit");
            return Ok((None, false));
        }

        let start = cursor.position();
        let decoded = u8::consensus_decode_from_finite_reader(cursor).and_then(|present| {
            if present == 0 {
                Ok(None)
            } else {
                MwebBlock::consensus_decode_from_finite_reader(cursor).map(Some)
            }
        });
        match decoded {
            Ok(Some(mweb)) => Ok((Some(mweb), false)),
            Ok(None) => {
                error!("block {height}: MWEB extension block marked absent");
                if mweb_strict() {
                    return Err(Error::Internal("MWEB extension block absent"));
                }
                Ok((None, true))
            }
            Err(e) => {
                error!("block {height}: MWEB extension block failed to decode: {e}");
                if mweb_strict() {
                    return Err(Error::Internal("MWEB extension block failed to decode"));
                }
                cursor.set_position(start);
                Ok((None, true))
            }
        }
    }

    #[cfg(test)]
    mod tests {
        use std::{fs, path::PathBuf};

        use bitcoin::{WitnessVersion, block::Header, consensus::Decodable};
        use brk_types::{BlkPosition, Height};

        use crate::{XORBytes, XORIndex, parse::parse_canonical_body};

        fn read(height: u32) -> brk_types::ReadBlock {
            let dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
            let hex = fs::read_to_string(dir.join(format!("{height}.hex"))).unwrap();
            let bytes: Vec<u8> = (0..hex.trim().len())
                .step_by(2)
                .map(|i| u8::from_str_radix(&hex[i..i + 2], 16).unwrap())
                .collect();
            let header = Header::consensus_decode(&mut &bytes[..80]).unwrap();
            parse_canonical_body(
                bytes,
                BlkPosition::new(0, 0),
                XORIndex::default(),
                XORBytes::default(),
                Height::new(height),
                header,
            )
            .unwrap()
        }

        #[test]
        fn no_mweb_before_activation() {
            let block = read(2_265_983);
            assert!(block.mweb_block.is_none());
            assert!(!block.mweb_decode_failed());
            assert!(block.txdata.iter().all(|tx| !tx.is_hog_ex));
        }

        #[test]
        fn decodes_extension_from_activation() {
            for height in [2_265_984, 3_213_000] {
                let block = read(height);
                assert!(!block.mweb_decode_failed(), "{height}");
                let mweb = block.mweb_block.as_ref().expect("extension block");
                assert_eq!(mweb.header.height, height);
                let hogex = block.txdata.last().unwrap();
                assert!(hogex.is_hog_ex);
                assert_eq!(hogex.output[0].script_pubkey.witness_version(), Some(WitnessVersion::V8));
                let pegin: i64 = mweb.tx_body.kernels.iter().filter_map(|k| k.pegin).sum();
                let v9: i64 = block
                    .txdata
                    .iter()
                    .flat_map(|tx| &tx.output)
                    .filter(|o| o.script_pubkey.witness_version() == Some(WitnessVersion::V9))
                    .map(|o| o.value.to_sat() as i64)
                    .sum();
                assert_eq!(pegin, v9, "{height}");

                let output_type = |o: &bitcoin::TxOut| brk_types::OutputType::from(&o.script_pubkey);
                assert_eq!(output_type(&hogex.output[0]), brk_types::OutputType::MWEBPegPool);
                let peg_ins: i64 = block
                    .txdata
                    .iter()
                    .flat_map(|tx| &tx.output)
                    .filter(|o| output_type(o) == brk_types::OutputType::MWEBPegIn)
                    .map(|o| o.value.to_sat() as i64)
                    .sum();
                assert_eq!(peg_ins, v9, "{height}");
            }
            let json = fs::read_to_string(
                PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/3213000.json"),
            )
            .unwrap();
            let expected: serde_json::Value = serde_json::from_str(&json).unwrap();
            let hogaddr = read(3_213_000).txdata.last().unwrap().output[0].value.to_sat();
            assert_eq!(hogaddr, expected["mweb_amount"].as_u64().unwrap());
        }

        #[test]
        fn mweb_txids_match_decoder() {
            let block = read(3_213_000);
            for i in 0..block.txdata.len() {
                let (txid, _, _) = block.compute_txid_and_sizes(i);
                assert_eq!(txid, block.txdata[i].compute_txid(), "tx {i}");
            }
        }
    }
}
