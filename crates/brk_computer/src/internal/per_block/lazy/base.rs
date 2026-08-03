use brk_traversable::Traversable;
use brk_types::{Height, Version};
use derive_more::{Deref, DerefMut};
use schemars::JsonSchema;
use vecdb::{
    LazyVecFrom1, ReadOnlyClone, ReadableBoxedVec, ReadableCloneableVec, ReadableVec, TypedVec,
    UnaryTransform, VecValue,
};

use crate::{
    indexes,
    internal::{
        CachedPerBlock, ComputedVecValue, DerivedResolutions, Identity, NumericValue, PerBlock,
        Resolutions,
    },
};
#[derive(Clone, Deref, DerefMut, Traversable)]
#[traversable(merge)]
pub struct LazyPerBlock<T, S1T = T>
where
    T: VecValue + PartialOrd + JsonSchema,
    S1T: VecValue,
{
    pub height: LazyVecFrom1<Height, T, Height, S1T>,
    #[deref]
    #[deref_mut]
    #[traversable(flatten)]
    pub resolutions: Box<DerivedResolutions<T, S1T>>,
}

impl<T, S1T> LazyPerBlock<T, S1T>
where
    T: VecValue + PartialOrd + JsonSchema + 'static,
    S1T: VecValue + PartialOrd + JsonSchema,
{
    pub(crate) fn from_resolutions<F: UnaryTransform<S1T, T>>(
        name: &str,
        version: Version,
        height_source: ReadableBoxedVec<Height, S1T>,
        resolutions: &Resolutions<S1T>,
    ) -> Self
    where
        S1T: NumericValue,
    {
        Self {
            height: LazyVecFrom1::transformed::<F>(name, version, height_source),
            resolutions: Box::new(DerivedResolutions::from_derived_computed::<F>(
                name,
                version,
                resolutions,
            )),
        }
    }

    pub(crate) fn from_computed<F: UnaryTransform<S1T, T>>(
        name: &str,
        version: Version,
        height_source: ReadableBoxedVec<Height, S1T>,
        source: &PerBlock<S1T>,
    ) -> Self
    where
        S1T: NumericValue,
    {
        Self::from_resolutions::<F>(name, version, height_source, &source.resolutions)
    }

    pub(crate) fn from_cached_computed<F: UnaryTransform<S1T, T>>(
        name: &str,
        version: Version,
        height_source: ReadableBoxedVec<Height, S1T>,
        source: &CachedPerBlock<S1T>,
    ) -> Self
    where
        S1T: NumericValue,
    {
        Self::from_resolutions::<F>(name, version, height_source, &source.resolutions)
    }

    pub(crate) fn from_height_source<F: UnaryTransform<S1T, T>, V>(
        name: &str,
        version: Version,
        height_source: V,
        indexes: &indexes::Vecs,
    ) -> Self
    where
        S1T: NumericValue,
        V: TypedVec<I = Height, T = S1T> + ReadableVec<Height, S1T> + Clone + 'static,
    {
        Self {
            height: LazyVecFrom1::transformed::<F>(
                name,
                version,
                height_source.read_only_boxed_clone(),
            ),
            resolutions: Box::new(DerivedResolutions::from_height_source::<F, V>(
                name,
                version,
                height_source,
                indexes,
            )),
        }
    }

    /// Create by unary-transforming a LazyPerBlock source (chaining lazy vecs).
    pub(crate) fn from_lazy<F, S2T>(
        name: &str,
        version: Version,
        source: &LazyPerBlock<S1T, S2T>,
    ) -> Self
    where
        F: UnaryTransform<S1T, T>,
        S2T: ComputedVecValue + JsonSchema,
    {
        Self {
            height: LazyVecFrom1::transformed::<F>(
                name,
                version,
                source.height.read_only_boxed_clone(),
            ),
            resolutions: Box::new(DerivedResolutions::from_lazy::<F, S2T>(
                name,
                version,
                &source.resolutions,
            )),
        }
    }
}

impl<T> LazyPerBlock<T>
where
    T: NumericValue + JsonSchema + 'static,
{
    /// Derive a per-block metric from one height-indexed source and the height itself.
    pub(crate) fn from_indexed_source<S>(
        name: &str,
        version: Version,
        source: &(impl ReadableCloneableVec<Height, S> + 'static),
        compute: fn(Height, S) -> T,
        indexes: &indexes::Vecs,
    ) -> Self
    where
        S: VecValue,
    {
        let indexed = LazyVecFrom1::init(
            &format!("{name}_source"),
            version,
            source.read_only_boxed_clone(),
            compute,
        );
        Self::from_height_source::<Identity<T>, _>(name, version, indexed, indexes)
    }
}

impl<T, S1T> ReadOnlyClone for LazyPerBlock<T, S1T>
where
    T: VecValue + PartialOrd + JsonSchema,
    S1T: VecValue,
{
    type ReadOnly = Self;
    fn read_only_clone(&self) -> Self {
        self.clone()
    }
}
