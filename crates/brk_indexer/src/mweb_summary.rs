//! Compact per-block MWEB extension-block summaries for analytics.

use brk_types::{Sats, StoredU32};

/// Aggregated extension-block activity for one L1 height.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct MwebSummary {
    pub input_count: StoredU32,
    pub output_count: StoredU32,
    pub kernel_count: StoredU32,
    pub fee_sats: Sats,
    pub kernel_pegin_sats: Sats,
    pub kernel_pegout_sats: Sats,
}

impl MwebSummary {
    /// Summarize a Litecoin block's `mweb_block` body (zeros when absent).
    #[cfg(feature = "litecoin")]
    pub fn from_block(block: &brk_chain::primitives::Block) -> Self {
        let Some(mw) = block.mweb_block.as_ref() else {
            return Self::default();
        };
        let body = &mw.tx_body;
        let mut fee: i64 = 0;
        let mut pegin: i64 = 0;
        let mut pegout: i64 = 0;
        for k in &body.kernels {
            if let Some(f) = k.fee {
                fee = fee.saturating_add(f);
            }
            if let Some(p) = k.pegin {
                pegin = pegin.saturating_add(p);
            }
            for po in &k.pegouts {
                pegout = pegout.saturating_add(po.amount);
            }
        }
        Self {
            input_count: StoredU32::from(body.inputs.len() as u32),
            output_count: StoredU32::from(body.outputs.len() as u32),
            kernel_count: StoredU32::from(body.kernels.len() as u32),
            fee_sats: sats_from_i64(fee),
            kernel_pegin_sats: sats_from_i64(pegin),
            kernel_pegout_sats: sats_from_i64(pegout),
        }
    }

    #[cfg(not(feature = "litecoin"))]
    pub fn from_block(_block: &brk_chain::primitives::Block) -> Self {
        Self::default()
    }
}

#[inline]
fn sats_from_i64(v: i64) -> Sats {
    Sats::from(v.max(0) as u64)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn negative_kernel_amounts_clamp_to_zero() {
        assert_eq!(sats_from_i64(-1), Sats::ZERO);
        assert_eq!(sats_from_i64(42), Sats::from(42u64));
    }
}
