use crate::{TypedVec, VecIndex, VecValue};

use super::LazyVec;

impl<I, T, S1I, S1T> TypedVec for LazyVec<I, T, S1I, S1T>
where
    I: VecIndex,
    T: VecValue,
    S1I: VecIndex,
    S1T: VecValue,
{
    type I = I;
    type T = T;
}
