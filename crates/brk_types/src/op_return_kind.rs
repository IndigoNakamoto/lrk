use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display};
use vecdb::{Bytes, Formattable, Pco, TransparentPco};

#[derive(
    Debug,
    Clone,
    Copy,
    AsRefStr,
    Display,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    Serialize,
    Deserialize,
    JsonSchema,
    Hash,
)]
#[serde(rename_all = "snake_case")]
#[strum(serialize_all = "snake_case")]
#[repr(u8)]
pub enum OpReturnKind {
    Runes,
    VeriBlock,
    Omni,
    Stacks,
    Blockstack,
    Colu,
    OpenAssets,
    Komodo,
    CoinSpark,
    Poet,
    Docproof,
    OpenTimestamps,
    Factom,
    EternityWall,
    Memo,
    Bitproof,
    Ascribe,
    Stampery,
    Epobc,
    BareHash,
    Text,
    Empty,
    Unknown,
}

impl OpReturnKind {
    fn is_valid(value: u8) -> bool {
        value <= Self::Unknown as u8
    }
}

impl Formattable for OpReturnKind {
    #[inline(always)]
    fn write_to(&self, buf: &mut Vec<u8>) {
        buf.extend_from_slice(self.as_ref().as_bytes());
    }

    fn fmt_json(&self, buf: &mut Vec<u8>) {
        buf.push(b'"');
        self.write_to(buf);
        buf.push(b'"');
    }
}

impl Bytes for OpReturnKind {
    type Array = [u8; size_of::<Self>()];

    #[inline]
    fn to_bytes(&self) -> Self::Array {
        [*self as u8]
    }

    #[inline]
    fn from_bytes(bytes: &[u8]) -> vecdb::Result<Self> {
        if bytes.len() != size_of::<Self>() {
            return Err(vecdb::Error::WrongLength {
                expected: size_of::<Self>(),
                received: bytes.len(),
            });
        }
        let value = bytes[0];
        if !Self::is_valid(value) {
            return Err(vecdb::Error::InvalidArgument("invalid OpReturnKind"));
        }
        // SAFETY: We validated that value is a valid variant.
        Ok(unsafe { std::mem::transmute::<u8, Self>(value) })
    }
}

impl Pco for OpReturnKind {
    type NumberType = u8;
}

impl TransparentPco<u8> for OpReturnKind {}
