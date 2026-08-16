use std::{ops::AddAssign, sync::Arc};

use crate::{AnyVec, ReadOnlyClone, ReadableVec, TypedVec, Version, short_type_name};

use super::{
    ColumnId, ReadableColumnarVec,
    read::{fold_readable, try_fold_readable},
    schema::{selection_version, validate_column},
};

/// Lazy scalar sum of selected columns from any readable columnar source.
pub struct ColumnarSumVec<S, C>
where
    C: ColumnId,
    S: ReadableColumnarVec<C>,
{
    name: Arc<str>,
    base_version: Version,
    source: S,
    columns: Box<[C]>,
    selection_version: Version,
}

impl<S, C> Clone for ColumnarSumVec<S, C>
where
    C: ColumnId,
    S: ReadableColumnarVec<C>,
{
    fn clone(&self) -> Self {
        Self {
            name: Arc::clone(&self.name),
            base_version: self.base_version,
            source: self.source.clone(),
            columns: self.columns.clone(),
            selection_version: self.selection_version,
        }
    }
}

impl<S, C> ColumnarSumVec<S, C>
where
    C: ColumnId,
    S: ReadableColumnarVec<C>,
{
    /// Creates a lazy sum from a non-empty array of distinct column IDs.
    pub fn new<const M: usize>(name: &str, version: Version, source: S, columns: [C; M]) -> Self {
        assert!(M > 0, "ColumnarSumVec requires at least one column");
        let mut columns = columns.to_vec();
        for &column in &columns {
            validate_column(column);
        }
        columns.sort_unstable_by_key(|column| column.index());
        assert!(
            columns.windows(2).all(|pair| pair[0] != pair[1]),
            "ColumnarSumVec cannot sum the same column twice",
        );
        let selection_version = selection_version(2, &columns);
        Self {
            name: Arc::from(name),
            base_version: version,
            source,
            columns: columns.into_boxed_slice(),
            selection_version,
        }
    }
}

impl<S, C> AnyVec for ColumnarSumVec<S, C>
where
    C: ColumnId,
    S: ReadableColumnarVec<C>,
{
    fn version(&self) -> Version {
        self.base_version + self.source.version() + self.selection_version
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn len(&self) -> usize {
        self.source.len()
    }

    fn index_type_to_string(&self) -> &'static str {
        self.source.index_type_to_string()
    }

    fn value_type_to_size_of(&self) -> usize {
        size_of::<S::T>()
    }

    fn value_type_to_string(&self) -> &'static str {
        short_type_name::<S::T>()
    }

    fn region_names(&self) -> Vec<String> {
        Vec::new()
    }
}

impl<S, C> TypedVec for ColumnarSumVec<S, C>
where
    C: ColumnId,
    S: ReadableColumnarVec<C>,
{
    type I = S::I;
    type T = S::T;
}

impl<S, C> ReadableVec<S::I, S::T> for ColumnarSumVec<S, C>
where
    C: ColumnId,
    S: ReadableColumnarVec<C>,
    S::T: AddAssign,
{
    fn cursor_chunk_size(&self) -> usize {
        self.source.cursor_chunk_size()
    }

    fn read_into_at(&self, from: usize, to: usize, out: &mut Vec<S::T>) {
        let from = from.min(self.len());
        let to = to.min(self.len());
        if from >= to {
            return;
        }

        let out_start = out.len();
        out.reserve(to - from);
        let first = self.columns[0];
        self.source.for_each_column_chunk_at(
            &self.columns,
            from,
            to,
            &mut |column, row_start, values| {
                let start = out_start + row_start - from;
                if column == first {
                    debug_assert_eq!(start, out.len());
                    out.extend_from_slice(values);
                } else {
                    for (sum, value) in out[start..start + values.len()].iter_mut().zip(values) {
                        *sum += value.clone();
                    }
                }
            },
        );
    }

    fn for_each_range_dyn_at(&self, from: usize, to: usize, f: &mut dyn FnMut(S::T)) {
        fold_readable(self, from, to, (), |(), value| f(value));
    }

    fn fold_range_at<B, F: FnMut(B, S::T) -> B>(&self, from: usize, to: usize, init: B, f: F) -> B {
        fold_readable(self, from, to, init, f)
    }

    fn try_fold_range_at<B, E, F: FnMut(B, S::T) -> std::result::Result<B, E>>(
        &self,
        from: usize,
        to: usize,
        init: B,
        f: F,
    ) -> std::result::Result<B, E> {
        try_fold_readable(self, from, to, init, f)
    }
}

impl<S, C> ReadOnlyClone for ColumnarSumVec<S, C>
where
    C: ColumnId,
    S: ReadableColumnarVec<C>,
{
    type ReadOnly = Self;

    fn read_only_clone(&self) -> Self {
        self.clone()
    }
}
