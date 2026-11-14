use crate::{AddressOutputType, Cpfp, OutputType};

/// Each of mempool.space's CPFP shapes decodes to its own variant (untagged: first match wins).
#[test]
fn cpfp_shapes() {
    let decode = |json: &str| serde_json::from_str::<Cpfp>(json).unwrap();
    let txid = "\"965f866bf8623bbf956c1b2aeec1efc1ad162fd428ab7fb89f128a0754ebbc32\"";
    let entry = format!("{{\"txid\":{txid},\"weight\":616,\"fee\":155500}}");
    let cluster = format!(
        "{{\"txs\":[{{\"txid\":{txid},\"weight\":694,\"fee\":174000,\"parents\":[]}}],\"chunks\":[{{\"txs\":[0],\"feerate\":1000.0}}],\"chunkIndex\":0}}"
    );
    let mempool = format!(
        "{{\"ancestors\":[{entry}],\"bestDescendant\":null,\"descendants\":[],\"effectiveFeePerVsize\":1000.0,\"sigops\":0,\"fee\":174000,\"vsize\":174,\"adjustedVsize\":174,\"cluster\":{cluster}}}"
    );
    let confirmed = format!(
        "{{\"ancestors\":[{entry}],\"descendants\":[],\"effectiveFeePerVsize\":1000.0,\"cluster\":{cluster}}}"
    );
    assert!(matches!(decode(&mempool), Cpfp::Mempool(_)));
    assert!(matches!(decode(&confirmed), Cpfp::Confirmed(_)));
    assert!(matches!(decode("{\"ancestors\":[]}"), Cpfp::Empty(_)));
    assert_eq!(
        serde_json::to_string(&Cpfp::EMPTY).unwrap(),
        "{\"ancestors\":[]}"
    );
}

/// The address types are exactly the output types that have addresses.
#[test]
fn address_output_types() {
    let names = [
        "p2pk65",
        "p2pk33",
        "p2pkh",
        "p2ms",
        "p2sh",
        "op_return",
        "p2wpkh",
        "p2wsh",
        "p2tr",
        "p2a",
        "empty_script",
        "unknown_script",
        #[cfg(feature = "litecoin")]
        "mweb_peg_pool",
        #[cfg(feature = "litecoin")]
        "mweb_peg_in",
    ];
    assert_eq!(names.len(), OutputType::COUNT);
    for name in names {
        let json = format!("\"{name}\"");
        let output_type = serde_json::from_str::<OutputType>(&json).unwrap();
        let parsed = serde_json::from_str::<AddressOutputType>(&json);
        assert_eq!(parsed.is_ok(), output_type.is_addr(), "{name}");
        if let Ok(address_type) = parsed {
            assert_eq!(serde_json::to_string(&address_type).unwrap(), json);
        }
    }
}

#[cfg(feature = "litecoin")]
#[test]
fn mweb_scripts_classify() {
    let program = |version: u8| {
        let mut bytes = vec![version, 0x20];
        bytes.extend([7u8; 32]);
        bitcoin::ScriptBuf::from_bytes(bytes)
    };
    assert_eq!(OutputType::from(&program(0x58)), OutputType::MWEBPegPool);
    assert_eq!(OutputType::from(&program(0x59)), OutputType::MWEBPegIn);
    assert_eq!(OutputType::from(&program(0x5a)), OutputType::UnknownScript);
    for output_type in [OutputType::MWEBPegPool, OutputType::MWEBPegIn] {
        assert!(output_type.is_mweb() && !output_type.is_addr());
        assert_eq!(output_type.storage_type(), OutputType::UnknownScript);
    }
}
