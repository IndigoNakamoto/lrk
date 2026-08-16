mod sighash;

use crate::TxFeatureFlags;

pub(super) fn scan_ecdsa_signature(bytes: &[u8], flags: &mut TxFeatureFlags) {
    sighash::scan_ecdsa_signature(bytes, flags);
}

pub(super) fn scan_taproot_signature(bytes: &[u8], flags: &mut TxFeatureFlags) {
    sighash::scan_taproot_signature(bytes, flags);
}

pub(super) fn record_validated_ecdsa_sighash(bytes: &[u8], flags: &mut TxFeatureFlags) {
    sighash::record_validated_ecdsa_sighash(bytes, flags);
}

pub(super) fn record_validated_taproot_sighash(bytes: &[u8], flags: &mut TxFeatureFlags) {
    sighash::record_validated_taproot_sighash(bytes, flags);
}
