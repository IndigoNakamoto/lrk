pub mod count;
pub mod features;
pub mod fees;
pub mod hogex;
pub mod patterns;
pub mod policy;
pub mod sigops;
pub mod size;
pub mod versions;
pub mod volume;

mod compute;
mod import;

use brk_traversable::Traversable;
use vecdb::{Database, Rw, StorageMode};

pub use count::Vecs as CountVecs;
pub use features::Vecs as FeaturesVecs;
pub use fees::Vecs as FeesVecs;
pub use hogex::Vecs as HogexVecs;
pub use patterns::Vecs as PatternsVecs;
pub use policy::Vecs as PolicyVecs;
pub use sigops::Vecs as SigopsVecs;
pub use size::Vecs as SizeVecs;
pub use versions::Vecs as VersionsVecs;
pub use volume::Vecs as VolumeVecs;

pub const DB_NAME: &str = "transactions";

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    #[traversable(skip)]
    pub(crate) db: Database,

    pub count: CountVecs<M>,
    pub features: FeaturesVecs<M>,
    pub size: SizeVecs<M>,
    pub fees: FeesVecs<M>,
    pub hogex: HogexVecs<M>,
    pub patterns: PatternsVecs<M>,
    pub policy: PolicyVecs<M>,
    pub sigops: SigopsVecs<M>,
    pub versions: VersionsVecs<M>,
    pub volume: VolumeVecs<M>,
}
