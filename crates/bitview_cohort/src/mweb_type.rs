#[cfg(feature = "storage")]
use bitview_traversable::Traversable;

pub(crate) const MWEB_PEG_POOL: &str = "mweb_peg_pool";
pub(crate) const MWEB_PEG_IN: &str = "mweb_peg_in";

/// Litecoin MWEB's protocol outputs: counted apart from the spendable types, and never part of
/// the transparent UTXO supply or its holders (the MWEB plugin accounts for their coins).
#[derive(Default, Clone, Debug)]
#[cfg_attr(feature = "storage", derive(Traversable))]
pub struct MwebType<T> {
    /// Uses the MWEB pool's HogAddr (witness v8): output 0 of each block's HogEx.
    #[cfg_attr(feature = "storage", traversable(lead = "The MWEB pool's HogAddr output, one per block."))]
    pub mweb_peg_pool: T,
    /// Uses peg-in outputs (witness v9), which their block's HogEx sweeps into MWEB.
    #[cfg_attr(feature = "storage", traversable(lead = "Peg-in outputs: coins entering MWEB."))]
    pub mweb_peg_in: T,
}
