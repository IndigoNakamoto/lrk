macro_rules! with_transaction_features {
    ($macro:ident) => {
        $macro! {
            has_p2pk: P2PK = 0, count: p2pk;
            has_p2ms: P2MS = 1, count: p2ms;
            has_p2pkh: P2PKH = 2, count: p2pkh;
            has_p2sh: P2SH = 3, count: p2sh;
            has_p2wpkh: P2WPKH = 4, count: p2wpkh;
            has_p2wsh: P2WSH = 5, count: p2wsh;
            has_p2tr: P2TR = 6, count: p2tr;
            has_p2a: P2A = 7, count: p2a;
            has_op_return: OP_RETURN = 8, count: op_return;
            has_empty: EMPTY = 9, count: empty;
            has_unknown: UNKNOWN = 10, count: unknown;
            has_fake_pubkey: FAKE_PUBKEY = 11, count: fake_pubkey;
            has_fake_scripthash: FAKE_SCRIPTHASH = 12, count: fake_scripthash;
            has_inscription: INSCRIPTION = 13, count: inscription, count_attr: traversable(hidden);
            has_annex: ANNEX = 14, count: annex, count_attr: traversable(hidden);
            has_sighash_all: SIGHASH_ALL = 15, count: sighash_all, count_attr: traversable(hidden);
            has_sighash_none: SIGHASH_NONE = 16, count: sighash_none, count_attr: traversable(hidden);
            has_sighash_single: SIGHASH_SINGLE = 17, count: sighash_single, count_attr: traversable(hidden);
            has_sighash_default: SIGHASH_DEFAULT = 18, count: sighash_default, count_attr: traversable(hidden);
            has_sighash_anyone_can_pay: SIGHASH_ANYONE_CAN_PAY = 19, count: sighash_anyone_can_pay, count_attr: traversable(hidden);
            #[traversable(hidden)]
            is_unconditionally_nonstandard: UNCONDITIONALLY_NONSTANDARD = 20;
            has_dust_output: DUST_OUTPUT = 21, count: dust_output, count_attr: traversable(hidden);
        }
    };
}

pub(super) use with_transaction_features;
