use bitview_cohort::{AmountRange, SpendableType};
use bitview_primitives::SupplyState;
use brk_types::{OutputType, Sats};
use vecdb::unlikely;

#[derive(Default, Debug)]
pub struct ReceivedOutputs {
    pub by_type: SpendableType<SupplyState>,
    pub by_amount: AmountRange<SupplyState>,
}

impl ReceivedOutputs {
    pub fn add(&mut self, value: Sats, ty: OutputType) {
        // MWEB's protocol outputs hold MWEB's coins, not transparent holders': the distribution
        // is transparent-only (see the MWEB plugin).
        if unlikely(ty.is_unspendable() || ty.is_mweb()) {
            return;
        }
        let supply = SupplyState {
            utxo_count: 1,
            value,
        };
        *self.by_type.get_mut(ty) += &supply;
        *self.by_amount.get_mut(value) += &supply;
    }
}
