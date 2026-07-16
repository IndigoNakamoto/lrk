use brk_traversable::Traversable;
use brk_types::OpReturnKind;

macro_rules! define_by_kind {
    ($($field:ident => $kind:ident),+ $(,)?) => {
        #[derive(Traversable)]
        pub struct ByKind<T> {
            $(pub $field: T),+
        }

        impl<T> ByKind<T> {
            pub fn try_new<E>(
                mut create: impl FnMut(OpReturnKind, &'static str) -> Result<T, E>,
            ) -> Result<Self, E> {
                Ok(Self {
                    $($field: create(OpReturnKind::$kind, stringify!($field))?),+
                })
            }

            pub fn iter(&self) -> impl Iterator<Item = &T> {
                [$( &self.$field ),+].into_iter()
            }

            pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
                let Self { $($field),+ } = self;
                [$($field),+].into_iter()
            }

            pub fn iter_typed(&self) -> impl Iterator<Item = (OpReturnKind, &T)> {
                [$( (OpReturnKind::$kind, &self.$field) ),+].into_iter()
            }

            pub fn iter_typed_mut(
                &mut self,
            ) -> impl Iterator<Item = (OpReturnKind, &mut T)> {
                let Self { $($field),+ } = self;
                [$( (OpReturnKind::$kind, $field) ),+].into_iter()
            }
        }
    };
}

define_by_kind! {
    runes => Runes,
    veri_block => VeriBlock,
    omni => Omni,
    stacks => Stacks,
    blockstack => Blockstack,
    colu => Colu,
    open_assets => OpenAssets,
    komodo => Komodo,
    coin_spark => CoinSpark,
    poet => Poet,
    docproof => Docproof,
    open_timestamps => OpenTimestamps,
    factom => Factom,
    eternity_wall => EternityWall,
    memo => Memo,
    bitproof => Bitproof,
    ascribe => Ascribe,
    stampery => Stampery,
    epobc => Epobc,
    bare_hash => BareHash,
    text => Text,
    empty => Empty,
    unknown => Unknown,
}

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use brk_types::OpReturnKind;

    use super::ByKind;

    #[test]
    fn covers_every_kind_in_discriminant_order() {
        let by_kind =
            ByKind::try_new(|kind, _| Ok::<_, Infallible>(kind)).expect("infallible constructor");
        let kinds: Vec<_> = by_kind.iter_typed().collect();

        assert_eq!(kinds.len(), OpReturnKind::Unknown as usize + 1);
        for (index, (kind, value)) in kinds.into_iter().enumerate() {
            assert_eq!(kind as usize, index);
            assert_eq!(kind, *value);
        }
    }
}
