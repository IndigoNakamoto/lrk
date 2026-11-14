//! Litecoin's MWEB extension block, seen from the transparent chain.
//!
//! Every coin inside MWEB sits in one transparent output, the HogAddr: output 0 of the block's
//! HogEx, its last transaction. Each HogEx spends the previous HogAddr and the block's peg-ins and
//! pays the new HogAddr and the block's peg-outs, so the pool's supply and its flows are read
//! from the HogEx alone. The kernels of the decoded extension block record the same flows from
//! MWEB's side; the two must agree, which `accounting_mismatch` checks block by block.
//!
//! Only built with the `litecoin` feature.
#![cfg(feature = "litecoin")]

mod compute;
mod dependencies;
mod has;
mod import;

pub use dependencies::Dependencies;
pub use has::HasMweb;

use bitview_plugin::{Plugin, PluginId, PluginStorage};
use bitview_primitives::{Boolean, Count32};
use bitview_traversable::Traversable;
use bitview_vecs::{LazyPerBlock, PerBlock, ValuePerBlockCumulativeRolling};
use brk_types::{Bitcoin, Sats, SatsSigned, Version};
use vecdb::{Database, Rw, StorageMode};

const STORAGE: PluginStorage = PluginStorage::new(PluginId::new("mweb"), Version::ONE);
pub const ID: PluginId = STORAGE.id();

#[derive(Traversable, Plugin)]
#[plugin(storage = STORAGE)]
pub struct Vecs<M: StorageMode = Rw> {
    #[traversable(skip)]
    db: Database,

    /// Satoshis in the MWEB pool after each block: the value of the HogAddr, the HogEx's
    /// output 0. Zero before activation (block 2,265,984).
    #[traversable(hidden)]
    pub supply_sats: PerBlock<Sats, M>,
    /// Coins inside MWEB after each block, in LTC: the HogAddr's value, which Litecoin Core
    /// reports as `getblockheader`'s `mweb_amount`. Not in the UTXO-based circulating supply,
    /// where the HogAddr is excluded as a protocol output.
    #[traversable(rename = "mweb_supply")]
    #[traversable(lead = "MWEB supply: the coins inside Litecoin's private extension block.")]
    supply: LazyPerBlock<Bitcoin, Sats>,
    /// Coins entering MWEB in the block: the peg-in outputs (witness v9) its HogEx sweeps.
    #[traversable(lead = "Peg-ins: coins moving from the transparent chain into MWEB.")]
    pub peg_in: ValuePerBlockCumulativeRolling<M>,
    /// Coins leaving MWEB in the block: the HogEx's outputs after the HogAddr.
    #[traversable(lead = "Peg-outs: coins moving from MWEB back to the transparent chain.")]
    pub peg_out: ValuePerBlockCumulativeRolling<M>,
    /// Fees paid by MWEB transactions in the block: the HogEx's inputs minus its outputs, which
    /// the coinbase claims with the transparent fees.
    #[traversable(lead = "MWEB fees: what MWEB transactions paid miners.")]
    pub fee: ValuePerBlockCumulativeRolling<M>,
    /// Peg-ins minus peg-outs in the block, in satoshis.
    #[traversable(lead = "MWEB net flow: peg-ins minus peg-outs.")]
    pub net_flow: PerBlock<SatsSigned, M>,
    /// Peg-in outputs the block's HogEx sweeps.
    #[traversable(lead = "Peg-in count.")]
    pub pegin_count: PerBlock<Count32, M>,
    /// Peg-out outputs the block's HogEx pays.
    #[traversable(lead = "Peg-out count.")]
    pub pegout_count: PerBlock<Count32, M>,
    /// Whether the block ends with a HogEx: true from activation on.
    #[traversable(lead = "Whether MWEB was active in the block.")]
    pub active: PerBlock<Boolean, M>,
    /// The HogAddr's change minus the kernels' peg-ins, less their peg-outs and fees: zero
    /// whenever the transparent chain and the extension block agree.
    #[traversable(hidden)]
    pub accounting_mismatch: PerBlock<SatsSigned, M>,
    /// The previous HogAddr the HogEx spends minus the pool's previous supply: zero when the
    /// HogEx's input 0 is the previous block's HogAddr.
    #[traversable(hidden)]
    pub link_mismatch: PerBlock<SatsSigned, M>,
}
