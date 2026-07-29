use std::marker::PhantomData;

use brk_types::{
    Bitcoin, Cents, Dollars, PartsPerMillion32, Sats, StoredF32, StoredF64, StoredI8, StoredU16,
    StoredU32, StoredU64, VSize, Weight,
};
use vecdb::{BinaryTransform, UnaryTransform, VecValue};

pub struct Identity<T>(PhantomData<T>);

impl<T: VecValue> UnaryTransform<T, T> for Identity<T> {
    #[inline(always)]
    fn apply(v: T) -> T {
        v
    }
}

pub struct HalveSats;

impl UnaryTransform<Sats, Sats> for HalveSats {
    #[inline(always)]
    fn apply(sats: Sats) -> Sats {
        sats / 2
    }
}

pub struct HalveSatsToBitcoin;

impl UnaryTransform<Sats, Bitcoin> for HalveSatsToBitcoin {
    #[inline(always)]
    fn apply(sats: Sats) -> Bitcoin {
        Bitcoin::from(sats / 2)
    }
}

pub struct HalveCents;

impl UnaryTransform<Cents, Cents> for HalveCents {
    #[inline(always)]
    fn apply(cents: Cents) -> Cents {
        cents / 2u64
    }
}

pub struct HalveDollars;

impl UnaryTransform<Dollars, Dollars> for HalveDollars {
    #[inline(always)]
    fn apply(dollars: Dollars) -> Dollars {
        dollars.halved()
    }
}

pub struct MaskSats;

impl BinaryTransform<StoredU32, Sats, Sats> for MaskSats {
    #[inline(always)]
    fn apply(mask: StoredU32, value: Sats) -> Sats {
        if mask == StoredU32::ONE {
            value
        } else {
            Sats::ZERO
        }
    }
}

impl BinaryTransform<StoredU64, Sats, Sats> for MaskSats {
    #[inline]
    fn apply(mask: StoredU64, value: Sats) -> Sats {
        if u64::from(mask) != 0 {
            value
        } else {
            Sats::ZERO
        }
    }
}

pub struct ReturnF32Tenths<const V: u16>;

impl<S, const V: u16> UnaryTransform<S, StoredF32> for ReturnF32Tenths<V> {
    #[inline(always)]
    fn apply(_: S) -> StoredF32 {
        StoredF32::from(V as f32 / 10.0)
    }
}

pub struct ReturnU16<const V: u16>;

impl<S, const V: u16> UnaryTransform<S, StoredU16> for ReturnU16<V> {
    #[inline(always)]
    fn apply(_: S) -> StoredU16 {
        StoredU16::new(V)
    }
}

pub struct ReturnI8<const V: i8>;

impl<S, const V: i8> UnaryTransform<S, StoredI8> for ReturnI8<V> {
    #[inline(always)]
    fn apply(_: S) -> StoredI8 {
        StoredI8::new(V)
    }
}

pub struct ThsToPhsF32;

impl UnaryTransform<StoredF32, StoredF32> for ThsToPhsF32 {
    #[inline(always)]
    fn apply(ths: StoredF32) -> StoredF32 {
        (*ths * 1000.0).into()
    }
}

pub struct BlocksToDaysF32;

impl UnaryTransform<StoredU32, StoredF32> for BlocksToDaysF32 {
    #[inline(always)]
    fn apply(blocks: StoredU32) -> StoredF32 {
        (*blocks as f32 / crate::blocks::TARGET_BLOCKS_PER_DAY_F32).into()
    }
}

pub struct OneMinusF64;

impl UnaryTransform<StoredF64, StoredF64> for OneMinusF64 {
    #[inline(always)]
    fn apply(v: StoredF64) -> StoredF64 {
        StoredF64::from(1.0 - *v)
    }
}

pub struct DifficultyToHashF64;

impl UnaryTransform<StoredF64, StoredF64> for DifficultyToHashF64 {
    #[inline(always)]
    fn apply(difficulty: StoredF64) -> StoredF64 {
        const MULTIPLIER: f64 = 4_294_967_296.0 / 600.0; // 2^32 / 600
        StoredF64::from(*difficulty * MULTIPLIER)
    }
}

pub struct OneMinusPpm;

impl UnaryTransform<PartsPerMillion32, PartsPerMillion32> for OneMinusPpm {
    #[inline(always)]
    fn apply(value: PartsPerMillion32) -> PartsPerMillion32 {
        PartsPerMillion32::ONE - value
    }
}

pub struct VBytesToWeight;

impl UnaryTransform<StoredU64, Weight> for VBytesToWeight {
    #[inline(always)]
    fn apply(vbytes: StoredU64) -> Weight {
        Weight::from(VSize::new(*vbytes))
    }
}

pub struct WeightToVSize;

impl UnaryTransform<Weight, VSize> for WeightToVSize {
    #[inline(always)]
    fn apply(weight: Weight) -> VSize {
        VSize::from(weight)
    }
}
