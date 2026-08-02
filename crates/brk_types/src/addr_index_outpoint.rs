use std::hash::{Hash, Hasher};

use byteview::ByteView;
use serde::Serialize;

use crate::{AddrIndexTxIndex, Vout};

use super::{OutPoint, TxIndex, TypeIndex};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy, Serialize)]
#[repr(C)]
pub struct AddrIndexOutPoint {
    addr_index_tx_index: AddrIndexTxIndex, // u64
    vout: Vout,                            // u16
}

impl AddrIndexOutPoint {
    #[inline]
    pub(crate) fn to_be_bytes(self) -> [u8; 10] {
        let mut bytes = [0; 10];
        bytes[..8].copy_from_slice(&self.addr_index_tx_index.to_be_bytes());
        bytes[8..].copy_from_slice(&self.vout.to_be_bytes());
        bytes
    }

    #[inline]
    pub fn tx_index(&self) -> TxIndex {
        self.addr_index_tx_index.tx_index()
    }

    #[inline]
    pub fn vout(&self) -> Vout {
        self.vout
    }
}

impl Hash for AddrIndexOutPoint {
    #[inline]
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.addr_index_tx_index.hash(state);
        self.vout.hash(state);
    }
}

impl From<(TypeIndex, OutPoint)> for AddrIndexOutPoint {
    #[inline]
    fn from((addr_index, outpoint): (TypeIndex, OutPoint)) -> Self {
        Self {
            addr_index_tx_index: AddrIndexTxIndex::from((addr_index, outpoint.tx_index())),
            vout: outpoint.vout(),
        }
    }
}

impl From<ByteView> for AddrIndexOutPoint {
    #[inline]
    fn from(value: ByteView) -> Self {
        Self {
            addr_index_tx_index: AddrIndexTxIndex::from(ByteView::new(&value[..8])),
            vout: Vout::from(u16::from_be_bytes([value[8], value[9]])),
        }
    }
}

impl From<AddrIndexOutPoint> for ByteView {
    #[inline]
    fn from(value: AddrIndexOutPoint) -> Self {
        ByteView::from(&value)
    }
}
impl From<&AddrIndexOutPoint> for ByteView {
    #[inline]
    fn from(value: &AddrIndexOutPoint) -> Self {
        ByteView::from(value.to_be_bytes())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn byte_encoding_is_stable_and_roundtrips() {
        let value = AddrIndexOutPoint::from((
            TypeIndex::new(0x0102_0304),
            OutPoint::new(TxIndex::new(0x0506_0708), Vout::from(0x090a_u16)),
        ));
        let bytes = ByteView::from(value);

        assert_eq!(
            &*bytes,
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10],
            "the LSM key encoding is part of the persisted format",
        );
        assert_eq!(AddrIndexOutPoint::from(bytes), value);
    }
}
