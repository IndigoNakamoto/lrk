use std::sync::Arc;

use brk_traversable::{Traversable, TreeNode, make_leaf};
use brk_types::StoredU64;
use schemars::JsonSchema;
use serde::Serialize;
use vecdb::{
    AnyExportableVec, AnyVec, CheckedSub, Formattable, ReadableBoxedVec, ReadableVec, TypedVec,
    VecIndex, VecValue, Version, short_type_name,
};

#[derive(Clone)]
struct TerminalLen {
    get: Arc<dyn Fn() -> usize + Send + Sync>,
    version: Version,
}

impl TerminalLen {
    fn new<I, T>(source: ReadableBoxedVec<I, T>) -> Self
    where
        I: VecIndex,
        T: VecValue,
    {
        let version = source.version();
        Self {
            get: Arc::new(move || source.len()),
            version,
        }
    }

    #[inline(always)]
    fn get(&self) -> usize {
        (self.get)()
    }
}

/// Inclusive cumulative count derived from the next value in one first-index
/// source. The final value uses live terminal-length metadata.
#[derive(Clone)]
pub struct LazyCumulativeIndexVec<I, S>
where
    I: VecIndex,
    S: VecIndex,
{
    name: Arc<str>,
    base_version: Version,
    first_indexes: ReadableBoxedVec<I, S>,
    terminal_len: TerminalLen,
}

impl<I, S> LazyCumulativeIndexVec<I, S>
where
    I: VecIndex,
    S: VecIndex,
{
    pub fn new<TI, TT>(
        name: &str,
        version: Version,
        first_indexes: ReadableBoxedVec<I, S>,
        terminal: ReadableBoxedVec<TI, TT>,
    ) -> Self
    where
        TI: VecIndex,
        TT: VecValue,
    {
        Self {
            name: Arc::from(name),
            base_version: version,
            first_indexes,
            terminal_len: TerminalLen::new(terminal),
        }
    }

    fn for_each_value(&self, from: usize, to: usize, mut each: impl FnMut(StoredU64))
    where
        StoredU64: From<S>,
    {
        let len = self.len();
        let to = to.min(len);
        if from >= to {
            return;
        }

        self.first_indexes
            .for_each_range_dyn_at(from + 1, (to + 1).min(len), &mut |next| {
                each(StoredU64::from(next))
            });

        if to == len {
            each(StoredU64::from(S::from(self.terminal_len.get())));
        }
    }
}

impl<I, S> AnyVec for LazyCumulativeIndexVec<I, S>
where
    I: VecIndex,
    S: VecIndex,
{
    fn version(&self) -> Version {
        self.base_version + self.first_indexes.version() + self.terminal_len.version
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn len(&self) -> usize {
        self.first_indexes.len()
    }

    fn index_type_to_string(&self) -> &'static str {
        I::to_string()
    }

    fn region_names(&self) -> Vec<String> {
        Vec::new()
    }

    fn value_type_to_size_of(&self) -> usize {
        size_of::<StoredU64>()
    }

    fn value_type_to_string(&self) -> &'static str {
        short_type_name::<StoredU64>()
    }
}

impl<I, S> TypedVec for LazyCumulativeIndexVec<I, S>
where
    I: VecIndex,
    S: VecIndex,
{
    type I = I;
    type T = StoredU64;
}

impl<I, S> ReadableVec<I, StoredU64> for LazyCumulativeIndexVec<I, S>
where
    I: VecIndex,
    S: VecIndex,
    StoredU64: From<S>,
{
    fn cursor_chunk_size(&self) -> usize {
        self.first_indexes.cursor_chunk_size()
    }

    fn read_into_at(&self, from: usize, to: usize, buf: &mut Vec<StoredU64>) {
        buf.reserve(to.saturating_sub(from));
        self.for_each_value(from, to, |value| buf.push(value));
    }

    fn for_each_range_dyn_at(&self, from: usize, to: usize, each: &mut dyn FnMut(StoredU64)) {
        self.for_each_value(from, to, each);
    }

    fn fold_range_at<B, F: FnMut(B, StoredU64) -> B>(
        &self,
        from: usize,
        to: usize,
        init: B,
        fold: F,
    ) -> B {
        let mut values = Vec::with_capacity(to.saturating_sub(from));
        self.read_into_at(from, to, &mut values);
        values.into_iter().fold(init, fold)
    }

    fn try_fold_range_at<B, E, F: FnMut(B, StoredU64) -> Result<B, E>>(
        &self,
        from: usize,
        to: usize,
        init: B,
        fold: F,
    ) -> Result<B, E> {
        let mut values = Vec::with_capacity(to.saturating_sub(from));
        self.read_into_at(from, to, &mut values);
        values.into_iter().try_fold(init, fold)
    }

    fn collect_one_at(&self, index: usize) -> Option<StoredU64> {
        if index >= self.len() {
            return None;
        }
        let next = if index + 1 < self.len() {
            self.first_indexes.collect_one_at(index + 1)?
        } else {
            S::from(self.terminal_len.get())
        };
        Some(StoredU64::from(next))
    }

    fn read_sorted_into_at(&self, indices: &[usize], out: &mut Vec<StoredU64>) {
        out.reserve(indices.len());
        indices
            .iter()
            .filter_map(|&index| self.collect_one_at(index))
            .for_each(|value| out.push(value));
    }
}

impl<I, S> Traversable for LazyCumulativeIndexVec<I, S>
where
    I: VecIndex,
    S: VecIndex,
    StoredU64: From<S> + Formattable + Serialize + JsonSchema,
{
    fn iter_any_exportable(&self) -> impl Iterator<Item = &dyn AnyExportableVec> {
        std::iter::once(self as &dyn AnyExportableVec)
    }

    fn to_tree_node(&self) -> TreeNode {
        make_leaf::<I, StoredU64, _>(self)
    }
}

/// Per-item count derived by subtracting adjacent values in one first-index
/// source. Range reads fetch all required boundaries in one operation.
#[derive(Clone)]
pub struct LazyIndexCountVec<I, S>
where
    I: VecIndex,
    S: VecIndex,
{
    name: Arc<str>,
    base_version: Version,
    first_indexes: ReadableBoxedVec<I, S>,
    terminal_len: TerminalLen,
}

impl<I, S> LazyIndexCountVec<I, S>
where
    I: VecIndex,
    S: VecIndex + CheckedSub + Default,
    StoredU64: From<S>,
{
    pub fn new<TI, TT>(
        name: &str,
        version: Version,
        first_indexes: ReadableBoxedVec<I, S>,
        terminal: ReadableBoxedVec<TI, TT>,
    ) -> Self
    where
        TI: VecIndex,
        TT: VecValue,
    {
        Self {
            name: Arc::from(name),
            base_version: version,
            first_indexes,
            terminal_len: TerminalLen::new(terminal),
        }
    }

    #[inline(always)]
    fn count(current: S, next: S) -> StoredU64 {
        StoredU64::from(next.checked_sub(current).unwrap_or_default())
    }

    fn for_each_value(&self, from: usize, to: usize, mut each: impl FnMut(StoredU64)) {
        let len = self.len();
        let to = to.min(len);
        if from >= to {
            return;
        }

        let mut boundaries = self
            .first_indexes
            .collect_range_dyn(from, (to + 1).min(len));
        if to == len {
            boundaries.push(S::from(self.terminal_len.get()));
        }
        boundaries
            .windows(2)
            .for_each(|pair| each(Self::count(pair[0], pair[1])));
    }
}

impl<I, S> AnyVec for LazyIndexCountVec<I, S>
where
    I: VecIndex,
    S: VecIndex,
{
    fn version(&self) -> Version {
        self.base_version + self.first_indexes.version() + self.terminal_len.version
    }

    fn name(&self) -> &str {
        &self.name
    }

    fn len(&self) -> usize {
        self.first_indexes.len()
    }

    fn index_type_to_string(&self) -> &'static str {
        I::to_string()
    }

    fn region_names(&self) -> Vec<String> {
        Vec::new()
    }

    fn value_type_to_size_of(&self) -> usize {
        size_of::<StoredU64>()
    }

    fn value_type_to_string(&self) -> &'static str {
        short_type_name::<StoredU64>()
    }
}

impl<I, S> TypedVec for LazyIndexCountVec<I, S>
where
    I: VecIndex,
    S: VecIndex,
{
    type I = I;
    type T = StoredU64;
}

impl<I, S> ReadableVec<I, StoredU64> for LazyIndexCountVec<I, S>
where
    I: VecIndex,
    S: VecIndex + CheckedSub + Default,
    StoredU64: From<S>,
{
    fn cursor_chunk_size(&self) -> usize {
        self.first_indexes.cursor_chunk_size()
    }

    fn read_into_at(&self, from: usize, to: usize, buf: &mut Vec<StoredU64>) {
        buf.reserve(to.saturating_sub(from));
        self.for_each_value(from, to, |value| buf.push(value));
    }

    fn for_each_range_dyn_at(&self, from: usize, to: usize, each: &mut dyn FnMut(StoredU64)) {
        self.for_each_value(from, to, each);
    }

    fn fold_range_at<B, F: FnMut(B, StoredU64) -> B>(
        &self,
        from: usize,
        to: usize,
        init: B,
        fold: F,
    ) -> B {
        let mut values = Vec::with_capacity(to.saturating_sub(from));
        self.read_into_at(from, to, &mut values);
        values.into_iter().fold(init, fold)
    }

    fn try_fold_range_at<B, E, F: FnMut(B, StoredU64) -> Result<B, E>>(
        &self,
        from: usize,
        to: usize,
        init: B,
        fold: F,
    ) -> Result<B, E> {
        let mut values = Vec::with_capacity(to.saturating_sub(from));
        self.read_into_at(from, to, &mut values);
        values.into_iter().try_fold(init, fold)
    }

    fn collect_one_at(&self, index: usize) -> Option<StoredU64> {
        if index >= self.len() {
            return None;
        }

        let (current, next) = if index + 1 < self.len() {
            let boundaries = self.first_indexes.collect_range_dyn(index, index + 2);
            (*boundaries.first()?, *boundaries.get(1)?)
        } else {
            (
                self.first_indexes.collect_one_at(index)?,
                S::from(self.terminal_len.get()),
            )
        };
        Some(Self::count(current, next))
    }

    fn read_sorted_into_at(&self, indices: &[usize], out: &mut Vec<StoredU64>) {
        out.reserve(indices.len());
        indices
            .iter()
            .filter_map(|&index| self.collect_one_at(index))
            .for_each(|value| out.push(value));
    }
}

impl<I, S> Traversable for LazyIndexCountVec<I, S>
where
    I: VecIndex,
    S: VecIndex + CheckedSub + Default,
    StoredU64: From<S> + Formattable + Serialize + JsonSchema,
{
    fn iter_any_exportable(&self) -> impl Iterator<Item = &dyn AnyExportableVec> {
        std::iter::once(self as &dyn AnyExportableVec)
    }

    fn to_tree_node(&self) -> TreeNode {
        make_leaf::<I, StoredU64, _>(self)
    }
}

#[cfg(test)]
mod tests {
    use brk_types::Height;
    use vecdb::{
        AnyStoredVec, Database, EagerVec, ImportableVec, PcoVec, ReadableCloneableVec, WritableVec,
    };

    use super::*;

    #[test]
    fn next_boundaries_produce_cumulative_and_per_item_counts() {
        let suffix = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("brk-next-index-{}-{suffix}", std::process::id()));
        let db = Database::open(&path).unwrap();
        let mut first: EagerVec<PcoVec<Height, Height>> =
            EagerVec::forced_import(&db, "first", Version::ONE).unwrap();
        let mut terminal: EagerVec<PcoVec<Height, StoredU64>> =
            EagerVec::forced_import(&db, "terminal", Version::ONE).unwrap();

        for value in [0, 2, 5] {
            first.push(Height::new(value));
        }
        for value in 0_u64..6 {
            terminal.push(StoredU64::from(value));
        }
        first.write().unwrap();
        terminal.write().unwrap();

        let cumulative = LazyCumulativeIndexVec::new(
            "cumulative",
            Version::ONE,
            first.read_only_boxed_clone(),
            terminal.read_only_boxed_clone(),
        );
        let count = LazyIndexCountVec::new(
            "count",
            Version::ONE,
            first.read_only_boxed_clone(),
            terminal.read_only_boxed_clone(),
        );

        assert_eq!(
            cumulative.collect_range(Height::ZERO, Height::new(3)),
            [2_u64, 5, 6].map(StoredU64::from)
        );
        assert_eq!(
            count.collect_range(Height::ZERO, Height::new(3)),
            [2_u64, 3, 1].map(StoredU64::from)
        );
        assert_eq!(
            count.collect_range(Height::new(1), Height::new(3)),
            [3_u64, 1].map(StoredU64::from)
        );
        assert_eq!(
            cumulative.collect_one(Height::new(2)),
            Some(StoredU64::new(6))
        );
        assert_eq!(count.collect_one(Height::new(2)), Some(StoredU64::new(1)));

        drop(count);
        drop(cumulative);
        drop(first);
        drop(terminal);
        drop(db);
        std::fs::remove_dir_all(path).unwrap();
    }
}
