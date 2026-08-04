use brk_traversable::Traversable;
use brk_types::{
    Day1, Day3, Epoch, Halving, Hour1, Hour4, Hour12, Minute10, Minute30, Month1, Month3, Month6,
    Version, Week1, Year1, Year10,
};
use derive_more::{Deref, DerefMut};
use schemars::JsonSchema;
use vecdb::{LazyVecFrom1, ReadableCloneableVec, UnaryTransform, VecValue};

use crate::internal::{ComputedVecValue, PerResolution};

#[derive(Clone, Deref, DerefMut, Traversable)]
#[traversable(transparent)]
pub struct LazyIndexes<T, S>(
    #[allow(clippy::type_complexity)]
    pub  PerResolution<
        LazyVecFrom1<Minute10, T, Minute10, S>,
        LazyVecFrom1<Minute30, T, Minute30, S>,
        LazyVecFrom1<Hour1, T, Hour1, S>,
        LazyVecFrom1<Hour4, T, Hour4, S>,
        LazyVecFrom1<Hour12, T, Hour12, S>,
        LazyVecFrom1<Day1, T, Day1, S>,
        LazyVecFrom1<Day3, T, Day3, S>,
        LazyVecFrom1<Week1, T, Week1, S>,
        LazyVecFrom1<Month1, T, Month1, S>,
        LazyVecFrom1<Month3, T, Month3, S>,
        LazyVecFrom1<Month6, T, Month6, S>,
        LazyVecFrom1<Year1, T, Year1, S>,
        LazyVecFrom1<Year10, T, Year10, S>,
        LazyVecFrom1<Halving, T, Halving, S>,
        LazyVecFrom1<Epoch, T, Epoch, S>,
    >,
)
where
    T: ComputedVecValue + PartialOrd + JsonSchema,
    S: VecValue;

impl<T, S> LazyIndexes<T, S>
where
    T: ComputedVecValue + PartialOrd + JsonSchema,
    S: VecValue,
{
    pub(crate) fn from_lazy_indexes<Transform, U>(
        name: &str,
        version: Version,
        source: &LazyIndexes<S, U>,
    ) -> Self
    where
        Transform: UnaryTransform<S, T>,
        S: ComputedVecValue + PartialOrd + JsonSchema,
        U: VecValue,
    {
        macro_rules! period {
            ($idx:ident) => {
                LazyVecFrom1::transformed::<Transform>(
                    name,
                    version,
                    source.$idx.read_only_boxed_clone(),
                )
            };
        }

        Self(PerResolution {
            minute10: period!(minute10),
            minute30: period!(minute30),
            hour1: period!(hour1),
            hour4: period!(hour4),
            hour12: period!(hour12),
            day1: period!(day1),
            day3: period!(day3),
            week1: period!(week1),
            month1: period!(month1),
            month3: period!(month3),
            month6: period!(month6),
            year1: period!(year1),
            year10: period!(year10),
            halving: period!(halving),
            epoch: period!(epoch),
        })
    }
}
