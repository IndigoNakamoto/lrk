#[cfg(feature = "litecoin")]
use bitview_plugin_holders::Vecs as HoldersVecs;
use bitview_plugin_indexer::Indexer;
use bitview_plugin_mining::Vecs as MiningVecs;
#[cfg(feature = "litecoin")]
use bitview_plugin_mweb::Vecs as MwebVecs;
use bitview_plugin_outputs::Vecs as OutputsVecs;
use bitview_plugin_price::Vecs as PriceVecs;

pub struct Dependencies<'a> {
    pub indexer: &'a Indexer,
    pub outputs: &'a OutputsVecs,
    pub mining: &'a MiningVecs,
    pub price: &'a PriceVecs,
    #[cfg(feature = "litecoin")]
    pub holders: &'a HoldersVecs,
    #[cfg(feature = "litecoin")]
    pub mweb: &'a MwebVecs,
}
