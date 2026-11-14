use std::iter;

use brk_types::OutputType;

use super::{CohortId, SpendableType, SpendableTypeId, UnspendableType};
#[cfg(feature = "litecoin")]
use super::{
    MwebType,
    mweb_type::{MWEB_PEG_IN, MWEB_PEG_POOL},
};

#[cfg(feature = "storage")]
use bitview_traversable::Traversable;

pub(crate) const OP_RETURN: &str = "op_return";

/// An output type as analytics count it: a spendable type, OP_RETURN, or (Litecoin) one of
/// MWEB's protocol outputs.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputTypeId {
    Spendable(SpendableTypeId),
    OpReturn,
    #[cfg(feature = "litecoin")]
    MwebPegPool,
    #[cfg(feature = "litecoin")]
    MwebPegIn,
}

#[cfg(feature = "litecoin")]
const MWEB_MEMBERS: [OutputTypeId; 2] = [OutputTypeId::MwebPegPool, OutputTypeId::MwebPegIn];
#[cfg(not(feature = "litecoin"))]
const MWEB_MEMBERS: [OutputTypeId; 0] = [];

impl OutputTypeId {
    /// Members of [`ByType`]: the spendable types, OP_RETURN, then MWEB's.
    pub const COUNT: usize = SpendableTypeId::ALL.len() + 1 + MWEB_MEMBERS.len();

    pub fn all() -> impl Iterator<Item = Self> {
        SpendableTypeId::ALL
            .iter()
            .map(|&kind| Self::Spendable(kind))
            .chain(iter::once(Self::OpReturn))
            .chain(MWEB_MEMBERS)
    }

    #[inline]
    pub const fn from_output_type(output_type: OutputType) -> Self {
        #[cfg(feature = "litecoin")]
        match output_type {
            OutputType::MWEBPegPool => return Self::MwebPegPool,
            OutputType::MWEBPegIn => return Self::MwebPegIn,
            _ => {}
        }
        match SpendableTypeId::from_output_type(output_type) {
            Some(kind) => Self::Spendable(kind),
            None => Self::OpReturn,
        }
    }

    /// Position among [`OutputTypeId::all`], for per-member counters.
    #[inline]
    pub const fn index(self) -> usize {
        match self {
            Self::Spendable(kind) => kind.index(),
            Self::OpReturn => SpendableTypeId::ALL.len(),
            #[cfg(feature = "litecoin")]
            Self::MwebPegPool => SpendableTypeId::ALL.len() + 1,
            #[cfg(feature = "litecoin")]
            Self::MwebPegIn => SpendableTypeId::ALL.len() + 2,
        }
    }

    /// The member key, the word per-entry series ids build on (`p2pk_output_count`).
    pub const fn key(self) -> &'static str {
        match self {
            Self::Spendable(kind) => kind.key(),
            Self::OpReturn => OP_RETURN,
            #[cfg(feature = "litecoin")]
            Self::MwebPegPool => MWEB_PEG_POOL,
            #[cfg(feature = "litecoin")]
            Self::MwebPegIn => MWEB_PEG_IN,
        }
    }

    pub const fn cohort(self) -> CohortId {
        match self {
            Self::Spendable(kind) => kind.cohort(),
            Self::OpReturn => CohortId::OpReturn,
            #[cfg(feature = "litecoin")]
            Self::MwebPegPool => CohortId::MwebPegPool,
            #[cfg(feature = "litecoin")]
            Self::MwebPegIn => CohortId::MwebPegIn,
        }
    }

    pub fn select<T>(self, values: &ByType<T>) -> &T {
        match self {
            Self::Spendable(kind) => kind.select(&values.spendable),
            Self::OpReturn => &values.unspendable.op_return,
            #[cfg(feature = "litecoin")]
            Self::MwebPegPool => &values.mweb.mweb_peg_pool,
            #[cfg(feature = "litecoin")]
            Self::MwebPegIn => &values.mweb.mweb_peg_in,
        }
    }
}

#[derive(Default, Clone, Debug)]
#[cfg_attr(feature = "storage", derive(Traversable))]
pub struct ByType<T> {
    #[cfg_attr(feature = "storage", traversable(flatten))]
    pub spendable: SpendableType<T>,
    #[cfg_attr(feature = "storage", traversable(flatten))]
    pub unspendable: UnspendableType<T>,
    #[cfg(feature = "litecoin")]
    #[cfg_attr(feature = "storage", traversable(flatten))]
    pub mweb: MwebType<T>,
}

impl<T> ByType<T> {
    pub fn from_fn(mut create: impl FnMut(OutputTypeId) -> T) -> Self {
        Self {
            spendable: SpendableType::from_fn(|kind| create(OutputTypeId::Spendable(kind))),
            unspendable: UnspendableType {
                op_return: create(OutputTypeId::OpReturn),
            },
            #[cfg(feature = "litecoin")]
            mweb: MwebType {
                mweb_peg_pool: create(OutputTypeId::MwebPegPool),
                mweb_peg_in: create(OutputTypeId::MwebPegIn),
            },
        }
    }

    pub fn try_from_fn<E>(mut create: impl FnMut(OutputTypeId) -> Result<T, E>) -> Result<Self, E> {
        Ok(Self {
            spendable: SpendableType::try_from_fn(|kind| create(OutputTypeId::Spendable(kind)))?,
            unspendable: UnspendableType {
                op_return: create(OutputTypeId::OpReturn)?,
            },
            #[cfg(feature = "litecoin")]
            mweb: MwebType {
                mweb_peg_pool: create(OutputTypeId::MwebPegPool)?,
                mweb_peg_in: create(OutputTypeId::MwebPegIn)?,
            },
        })
    }

    /// The member holding `output_type`; P2PK's 33- and 65-byte keys share one.
    pub fn get(&self, output_type: OutputType) -> &T {
        OutputTypeId::from_output_type(output_type).select(self)
    }

    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        #[cfg(feature = "litecoin")]
        let mweb = [&mut self.mweb.mweb_peg_pool, &mut self.mweb.mweb_peg_in];
        #[cfg(not(feature = "litecoin"))]
        let mweb: [&mut T; 0] = [];
        self.spendable
            .iter_mut()
            .chain(iter::once(&mut self.unspendable.op_return))
            .chain(mweb)
    }

    pub fn iter_typed_mut(&mut self) -> impl Iterator<Item = (OutputTypeId, &mut T)> {
        OutputTypeId::all().zip(self.iter_mut())
    }
}
