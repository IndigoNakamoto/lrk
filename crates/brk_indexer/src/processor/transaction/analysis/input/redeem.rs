use bitcoin::{Script, WitnessVersion};
use brk_types::OutputType;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    None,
    P2WPKH,
    P2WSH,
    Witness,
    Other,
}

#[derive(Clone, Copy)]
pub(super) struct Facts {
    kind: Kind,
    sigops: usize,
}

impl Facts {
    pub(super) fn analyze(last_push: Option<&[u8]>, output_type: OutputType) -> Self {
        if output_type != OutputType::P2SH {
            return Self::NONE;
        }

        let Some(bytes) = last_push else {
            return Self::NONE;
        };
        let script = Script::from_bytes(bytes);
        let kind = match script.witness_version() {
            Some(WitnessVersion::V0) if script.len() == 22 => Kind::P2WPKH,
            Some(WitnessVersion::V0) if script.len() == 34 => Kind::P2WSH,
            Some(_) => Kind::Witness,
            None => Kind::Other,
        };

        Self {
            kind,
            sigops: script.count_sigops(),
        }
    }

    pub(super) fn effective_output_type(
        self,
        output_type: OutputType,
        witness_items: usize,
    ) -> OutputType {
        match self.kind {
            Kind::P2WPKH if witness_items == 2 => OutputType::P2WPKH,
            _ => output_type,
        }
    }

    pub(super) fn is_present(self) -> bool {
        self.kind != Kind::None
    }

    pub(super) fn is_p2wpkh(self) -> bool {
        self.kind == Kind::P2WPKH
    }

    pub(super) fn is_p2wsh(self) -> bool {
        self.kind == Kind::P2WSH
    }

    pub(super) fn is_witness_program(self) -> bool {
        matches!(self.kind, Kind::P2WPKH | Kind::P2WSH | Kind::Witness)
    }

    pub(super) fn sigops(self) -> Option<usize> {
        self.is_present().then_some(self.sigops)
    }

    const NONE: Self = Self {
        kind: Kind::None,
        sigops: 0,
    };
}

#[cfg(test)]
mod tests {
    use bitcoin::ScriptBuf;
    use brk_types::OutputType;

    use super::{Facts, Kind};

    fn classify(hex: &str) -> Facts {
        let script = ScriptBuf::from_hex(hex).unwrap();
        Facts::analyze(Some(script.as_bytes()), OutputType::P2SH)
    }

    #[test]
    fn classifies_redeem_scripts_once() {
        assert_eq!(
            classify("00140000000000000000000000000000000000000000").kind,
            Kind::P2WPKH
        );
        assert_eq!(
            classify("00200000000000000000000000000000000000000000000000000000000000000000").kind,
            Kind::P2WSH
        );
        assert_eq!(
            classify("51200000000000000000000000000000000000000000000000000000000000000000").kind,
            Kind::Witness
        );
        assert_eq!(classify("ac").kind, Kind::Other);
        assert_eq!(classify("ac").sigops(), Some(1));
        assert_eq!(Facts::analyze(None, OutputType::P2SH).kind, Kind::None);
        assert_eq!(
            Facts::analyze(Some(&[0xac]), OutputType::P2PKH).kind,
            Kind::None
        );
    }
}
