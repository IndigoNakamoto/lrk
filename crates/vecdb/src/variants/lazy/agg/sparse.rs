use crate::{AggFold, ReadableVec, VecIndex, VecValue};

/// Sparse aggregation: emits `Option<T>` per output index.
///
/// `Some(last_value)` when the range contains source elements,
/// `None` when the range is empty.
pub struct Sparse;

impl<T: VecValue, SI: VecIndex> AggFold<Option<T>, SI, SI, T> for Sparse {
    #[inline]
    fn try_fold<S: ReadableVec<SI, T> + ?Sized, B, E, F: FnMut(B, Option<T>) -> Result<B, E>>(
        source: &S,
        mapping: &[SI],
        from: usize,
        to: usize,
        init: B,
        mut f: F,
    ) -> Result<B, E> {
        let source_len = source.len();

        let mut indices: Vec<usize> = Vec::with_capacity(to - from);
        let mut slot_map: Vec<Option<u32>> = Vec::with_capacity(to - from);

        (from..to).for_each(|idx| {
            let current_first = mapping[idx].to_usize();
            let next_first = mapping
                .get(idx + 1)
                .map(|h| h.to_usize())
                .unwrap_or(source_len)
                .min(source_len);

            if current_first >= next_first {
                slot_map.push(None);
            } else {
                slot_map.push(Some(indices.len() as u32));
                indices.push(next_first - 1);
            }
        });

        let values = source.read_sorted_at(&indices);

        slot_map.iter().try_fold(init, |acc, slot| match slot {
            None => f(acc, None),
            &Some(vi) => f(acc, Some(values[vi as usize].clone())),
        })
    }

    #[inline]
    fn collect_one<S: ReadableVec<SI, T> + ?Sized>(
        source: &S,
        mapping: &[SI],
        index: usize,
    ) -> Option<Option<T>> {
        let source_len = source.len();
        let current_first = mapping[index].to_usize();
        let next_first = mapping
            .get(index + 1)
            .map(|h| h.to_usize())
            .unwrap_or(source_len)
            .min(source_len);

        if current_first >= next_first {
            return Some(None);
        }
        Some(source.collect_one_at(next_first - 1))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AnyVec, Version};

    struct TestVec(Vec<u8>);

    impl AnyVec for TestVec {
        fn version(&self) -> Version {
            Version::ZERO
        }

        fn name(&self) -> &str {
            "test"
        }

        fn len(&self) -> usize {
            self.0.len()
        }

        fn index_type_to_string(&self) -> &'static str {
            "usize"
        }

        fn region_names(&self) -> Vec<String> {
            Vec::new()
        }

        fn value_type_to_size_of(&self) -> usize {
            size_of::<u8>()
        }

        fn value_type_to_string(&self) -> &'static str {
            "u8"
        }
    }

    impl ReadableVec<usize, u8> for TestVec {
        fn read_into_at(&self, from: usize, to: usize, buf: &mut Vec<u8>) {
            buf.extend_from_slice(&self.0[from.min(self.len())..to.min(self.len())]);
        }

        fn for_each_range_dyn_at(&self, from: usize, to: usize, f: &mut dyn FnMut(u8)) {
            self.0[from.min(self.len())..to.min(self.len())]
                .iter()
                .copied()
                .for_each(f);
        }

        fn fold_range_at<B, F: FnMut(B, u8) -> B>(
            &self,
            from: usize,
            to: usize,
            init: B,
            f: F,
        ) -> B {
            self.0[from.min(self.len())..to.min(self.len())]
                .iter()
                .copied()
                .fold(init, f)
        }

        fn try_fold_range_at<B, E, F: FnMut(B, u8) -> Result<B, E>>(
            &self,
            from: usize,
            to: usize,
            init: B,
            f: F,
        ) -> Result<B, E> {
            self.0[from.min(self.len())..to.min(self.len())]
                .iter()
                .copied()
                .try_fold(init, f)
        }
    }

    #[test]
    fn partial_source_returns_none_after_its_last_period() {
        let source = TestVec(vec![10, 20, 30]);
        let mapping = [0_usize, 2, 4, 6];

        let values = Sparse::fold(
            &source,
            &mapping,
            0,
            mapping.len(),
            Vec::new(),
            |mut v, x| {
                v.push(x);
                v
            },
        );

        assert_eq!(values, [Some(20), Some(30), None, None]);
        assert_eq!(Sparse::collect_one(&source, &mapping, 1), Some(Some(30)));
        assert_eq!(Sparse::collect_one(&source, &mapping, 2), Some(None));
    }
}
