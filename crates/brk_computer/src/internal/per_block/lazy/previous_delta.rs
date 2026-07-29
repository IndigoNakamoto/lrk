use std::sync::Arc;

use brk_traversable::{Traversable, TreeNode, make_leaf};
use schemars::JsonSchema;
use serde::Serialize;
use vecdb::{
    AnyExportableVec, AnyVec, CheckedSub, Formattable, ReadableBoxedVec, ReadableVec, TypedVec,
    VecIndex, VecValue, Version, short_type_name,
};

/// Lazy `source[index] - source[index - 1]`, with zero before the first value.
///
/// This is a single-source view: it follows source growth automatically and
/// stores nothing on disk.
pub struct LazyPreviousDeltaVec<I, T>
where
    I: VecIndex,
    T: VecValue,
{
    name: Arc<str>,
    base_version: Version,
    source: ReadableBoxedVec<I, T>,
}

impl<I, T> LazyPreviousDeltaVec<I, T>
where
    I: VecIndex,
    T: VecValue,
{
    pub fn new(name: &str, version: Version, source: ReadableBoxedVec<I, T>) -> Self {
        Self {
            name: Arc::from(name),
            base_version: version,
            source,
        }
    }
}

impl<I, T> Clone for LazyPreviousDeltaVec<I, T>
where
    I: VecIndex,
    T: VecValue,
{
    fn clone(&self) -> Self {
        Self {
            name: Arc::clone(&self.name),
            base_version: self.base_version,
            source: self.source.clone(),
        }
    }
}

impl<I, T> AnyVec for LazyPreviousDeltaVec<I, T>
where
    I: VecIndex,
    T: VecValue,
{
    fn version(&self) -> Version {
        self.base_version + self.source.version()
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn len(&self) -> usize {
        self.source.len()
    }

    fn index_type_to_string(&self) -> &'static str {
        I::to_string()
    }

    fn region_names(&self) -> Vec<String> {
        Vec::new()
    }

    fn value_type_to_size_of(&self) -> usize {
        size_of::<T>()
    }

    fn value_type_to_string(&self) -> &'static str {
        short_type_name::<T>()
    }
}

impl<I, T> TypedVec for LazyPreviousDeltaVec<I, T>
where
    I: VecIndex,
    T: VecValue,
{
    type I = I;
    type T = T;
}

impl<I, T> LazyPreviousDeltaVec<I, T>
where
    I: VecIndex,
    T: VecValue + CheckedSub + Default,
{
    fn for_each_delta(&self, from: usize, to: usize, mut each: impl FnMut(T)) {
        let to = to.min(self.len());
        if from >= to {
            return;
        }

        let read_from = from.saturating_sub(1);
        let values = self.source.collect_range_dyn(read_from, to);
        let mut values = values.into_iter();
        let mut previous = if from == 0 {
            T::default()
        } else {
            values.next().unwrap()
        };

        for current in values {
            each(current.clone().checked_sub(previous).unwrap_or_default());
            previous = current;
        }
    }
}

impl<I, T> ReadableVec<I, T> for LazyPreviousDeltaVec<I, T>
where
    I: VecIndex,
    T: VecValue + CheckedSub + Default,
{
    fn read_into_at(&self, from: usize, to: usize, buf: &mut Vec<T>) {
        buf.reserve(to.saturating_sub(from));
        self.for_each_delta(from, to, |value| buf.push(value));
    }

    fn for_each_range_dyn_at(&self, from: usize, to: usize, f: &mut dyn FnMut(T)) {
        self.for_each_delta(from, to, f);
    }

    fn fold_range_at<B, F: FnMut(B, T) -> B>(&self, from: usize, to: usize, init: B, f: F) -> B {
        let mut values = Vec::with_capacity(to.saturating_sub(from));
        self.read_into_at(from, to, &mut values);
        values.into_iter().fold(init, f)
    }

    fn try_fold_range_at<B, E, F: FnMut(B, T) -> Result<B, E>>(
        &self,
        from: usize,
        to: usize,
        init: B,
        f: F,
    ) -> Result<B, E> {
        let mut values = Vec::with_capacity(to.saturating_sub(from));
        self.read_into_at(from, to, &mut values);
        values.into_iter().try_fold(init, f)
    }

    fn collect_one_at(&self, index: usize) -> Option<T> {
        let current = self.source.collect_one_at(index)?;
        let previous = index
            .checked_sub(1)
            .and_then(|index| self.source.collect_one_at(index))
            .unwrap_or_default();
        Some(current.checked_sub(previous).unwrap_or_default())
    }

    fn read_sorted_into_at(&self, indices: &[usize], out: &mut Vec<T>) {
        out.reserve(indices.len());
        indices
            .iter()
            .filter_map(|&index| self.collect_one_at(index))
            .for_each(|value| out.push(value));
    }
}

impl<I, T> Traversable for LazyPreviousDeltaVec<I, T>
where
    I: VecIndex,
    T: VecValue + CheckedSub + Default + Formattable + Serialize + JsonSchema,
{
    fn iter_any_exportable(&self) -> impl Iterator<Item = &dyn AnyExportableVec> {
        std::iter::once(self as &dyn AnyExportableVec)
    }

    fn to_tree_node(&self) -> TreeNode {
        make_leaf::<I, T, _>(self)
    }
}
