use std::ops::{Add, AddAssign};

use brk_traversable::Traversable;

#[derive(Default, Clone, Debug, Traversable)]
pub struct UnspendableType<T> {
    pub op_return: T,
    /// Litecoin MWEB peg-pool (witness v8 / HogAddr macro balance).
    pub mweb_peg_pool: T,
    /// Litecoin MWEB peg-in outputs (witness v9).
    pub mweb_pegin: T,
}

impl<T> UnspendableType<T> {
    pub fn as_vec(&self) -> [&T; 3] {
        [
            &self.op_return,
            &self.mweb_peg_pool,
            &self.mweb_pegin,
        ]
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
            mweb_peg_pool: self.mweb_peg_pool + rhs.mweb_peg_pool,
            mweb_pegin: self.mweb_pegin + rhs.mweb_pegin,
        }
    }
}

impl<T> AddAssign for UnspendableType<T>
where
    T: AddAssign,
{
    fn add_assign(&mut self, rhs: Self) {
        self.op_return += rhs.op_return;
        self.mweb_peg_pool += rhs.mweb_peg_pool;
        self.mweb_pegin += rhs.mweb_pegin;
    }
}
