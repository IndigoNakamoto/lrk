use bitview_plugin::ImportContext;
use bitview_plugin_mappings::Vecs as MappingsVecs;
use bitview_vecs::{
    LazyPerBlock, LazyWindowStartVec, PerBlock, PerBlockCumulativeRolling,
    ValuePerBlockCumulativeRolling, shapes::Windows,
};
use brk_error::Result;
use vecdb::Convert;

use super::{STORAGE, Vecs};

impl Vecs {
    pub fn import(
        context: ImportContext<'_>,
        mappings: &MappingsVecs,
        window_starts: &Windows<&LazyWindowStartVec>,
    ) -> Result<Self> {
        let db = STORAGE.open_database(context, 1_000_000)?;
        let v = STORAGE.schema_version();

        let supply_sats = PerBlock::import(&db, "mweb_supply_sats", v, mappings)?;
        let supply = LazyPerBlock::from_resolutions::<Convert>("mweb_supply", v, &supply_sats);
        let flow =
            |name| ValuePerBlockCumulativeRolling::import(&db, name, v, mappings, window_starts);

        let this = Self {
            peg_in: flow("mweb_peg_in")?,
            peg_out: flow("mweb_peg_out")?,
            fee: flow("mweb_fee")?,
            kernels: PerBlockCumulativeRolling::import(
                &db,
                "mweb_kernels",
                v,
                mappings,
                window_starts,
            )?,
            net_flow: PerBlock::import(&db, "mweb_net_flow", v, mappings)?,
            pegin_count: PerBlock::import(&db, "mweb_pegin_count", v, mappings)?,
            pegout_count: PerBlock::import(&db, "mweb_pegout_count", v, mappings)?,
            active: PerBlock::import(&db, "mweb_active", v, mappings)?,
            accounting_mismatch: PerBlock::import(&db, "mweb_accounting_mismatch", v, mappings)?,
            link_mismatch: PerBlock::import(&db, "mweb_link_mismatch", v, mappings)?,
            supply_sats,
            supply,
            db,
        };
        STORAGE.finalize_database(&this.db)?;
        Ok(this)
    }
}
