use std::{fmt::Debug, ops::Add};

use crate::PrintableIndex;

/// Trait for types that can be used as vector indices.
pub trait VecIndex
where
    Self: Debug
        + Default
        + Copy
        + Clone
        + PartialEq
        + Eq
        + PartialOrd
        + Ord
        + From<usize>
        + Into<usize>
        + Add<usize, Output = Self>
        + Send
        + Sync
        + PrintableIndex
        + 'static,
{
    /// Initial element capacity for newly created fixed-width raw vectors.
    const INITIAL_CAPACITY: usize = 0;

    /// Converts this index to a `usize`.
    #[inline]
    fn to_usize(self) -> usize {
        self.into()
    }

    /// Returns the previous index, or `None` if this is zero.
    #[inline]
    fn decremented(self) -> Option<Self> {
        self.to_usize().checked_sub(1).map(Self::from)
    }
}

impl VecIndex for usize {}
