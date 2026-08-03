use brk_traversable::Traversable;
use vecdb::{Database, Rw, StorageMode};

use super::{bedrock, capital_sentiment, rarity_meter};

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    #[traversable(skip)]
    pub(crate) db: Database,

    pub bedrock: bedrock::Vecs<M>,
    pub capital_sentiment: capital_sentiment::Vecs<M>,
    pub rarity_meter: rarity_meter::RarityMeter<M>,
}
