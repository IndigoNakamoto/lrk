use std::ops::{Add, AddAssign};

use brk_traversable::Traversable;

#[derive(Default, Clone, Debug, Traversable)]
pub struct UnspendableType<T> {
    pub op_return: T,
    /// Litecoin MWEB outputs (peg-pool witness v8 + peg-in witness v9). Kept
    /// separate from `op_return` so MWEB is never counted as burned supply and
    /// gets its own per-type series.
    pub mweb: T,
}

impl<T> UnspendableType<T> {
    pub fn as_vec(&self) -> [&T; 2] {
        [&self.op_return, &self.mweb]
    }
}

impl<T> Add for UnspendableType<T>
where
    T: Add<Output = T>,
{
    type Output = Self;
    fn add(self, rhs: Self) -> Self::Output {
        Self {
            op_return: self.op_return + rhs.op_return,
            mweb: self.mweb + rhs.mweb,
        }
    }
}

impl<T> AddAssign for UnspendableType<T>
where
    T: AddAssign,
{
    fn add_assign(&mut self, rhs: Self) {
        self.op_return += rhs.op_return;
        self.mweb += rhs.mweb;
    }
}
