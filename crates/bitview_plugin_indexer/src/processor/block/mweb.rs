use bitview_primitives::{Boolean, Count, Count32};
use brk_types::{Sats, litecoin::MWEB_ACTIVATION_HEIGHT};
use vecdb::WritableVec;

use super::super::BlockProcessor;

impl BlockProcessor<'_> {
    /// Pushes the block's MWEB extension summary, zeros where there is none.
    pub fn push_mweb(&mut self) {
        let h = self.height;
        let block = self.block;
        let txdata = &block.txdata;
        let missing_hogex = h >= MWEB_ACTIVATION_HEIGHT
            && !(txdata.len() >= 2 && txdata.last().is_some_and(|tx| tx.is_hog_ex));

        let sats = |amount: i64| Sats::from(u64::try_from(amount).unwrap_or(0));
        let (mut fee, mut pegin, mut pegout) = (Sats::ZERO, Sats::ZERO, Sats::ZERO);
        let (mut pegin_kernels, mut pegout_kernels) = (0u32, 0u32);
        let (mut counts, mut mmr) = ([0u32; 3], [0u64; 2]);
        if let Some(mweb) = &block.mweb_block {
            let body = &mweb.tx_body;
            for kernel in &body.kernels {
                fee += kernel.fee.map_or(Sats::ZERO, sats);
                if let Some(amount) = kernel.pegin {
                    pegin += sats(amount);
                    pegin_kernels += 1;
                }
                if !kernel.pegouts.is_empty() {
                    pegout_kernels += 1;
                    for coin in &kernel.pegouts {
                        pegout += sats(coin.amount);
                    }
                }
            }
            counts = [
                body.kernels.len() as u32,
                body.inputs.len() as u32,
                body.outputs.len() as u32,
            ];
            mmr = [mweb.header.output_mmr_size, mweb.header.kernel_mmr_size];
        }

        let v = &mut self.vecs.mweb;
        v.kernel_count.debug_checked_push(h, Count32::from(counts[0]));
        v.input_count.debug_checked_push(h, Count32::from(counts[1]));
        v.output_count.debug_checked_push(h, Count32::from(counts[2]));
        v.output_mmr_size.debug_checked_push(h, Count::from(mmr[0]));
        v.kernel_mmr_size.debug_checked_push(h, Count::from(mmr[1]));
        v.kernel_fee.debug_checked_push(h, fee);
        v.kernel_pegin.debug_checked_push(h, pegin);
        v.kernel_pegout.debug_checked_push(h, pegout);
        v.pegin_kernel_count
            .debug_checked_push(h, Count32::from(pegin_kernels));
        v.pegout_kernel_count
            .debug_checked_push(h, Count32::from(pegout_kernels));
        v.decode_failed
            .debug_checked_push(h, Boolean::from(block.mweb_decode_failed()));
        v.missing_hogex
            .debug_checked_push(h, Boolean::from(missing_hogex));
    }
}
