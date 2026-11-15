//! Litecoin's whole supply: the transparent UTXO set plus the coins inside MWEB.

use bitview_compute::{prepare_computed, write_locked};
use bitview_plugin_holders::Vecs as HoldersVecs;
use bitview_plugin_mappings::Vecs as MappingsVecs;
use bitview_plugin_mweb::Vecs as MwebVecs;
use bitview_traversable::Traversable;
use bitview_vecs::{LazyPerBlock, PerBlock, ValuePerBlockCumulativeRolling};
use brk_error::{OptionData, Result};
use brk_exit::Exit;
use brk_types::{BLOCKS_PER_HALVING, Bitcoin, Height, Sats, SatsSigned, Version};
use vecdb::{AnyStoredVec, AnyVec, Convert, Database, ReadableVec, Rw, StorageMode, WritableVec};

#[derive(Traversable)]
pub struct Vecs<M: StorageMode = Rw> {
    #[traversable(hidden)]
    pub total_sats: PerBlock<Sats, M>,
    /// Circulating supply plus the coins inside MWEB: every coin in existence, unspent. The
    /// circulating supply counts the transparent UTXO set only, which leaves out MWEB's
    /// protocol outputs (the HogAddr and peg-ins).
    #[traversable(rename = "total_supply")]
    #[traversable(lead = "Total supply: circulating plus MWEB, every coin in existence.")]
    pub total: LazyPerBlock<Bitcoin, Sats>,
    /// Total supply minus every block subsidy so far, less what was burned: zero while the
    /// UTXO set, the MWEB pool and the burned coins account for each coin issued.
    #[traversable(hidden)]
    pub invariant_mismatch: PerBlock<SatsSigned, M>,
}

impl Vecs {
    pub fn import(db: &Database, version: Version, mappings: &MappingsVecs) -> Result<Self> {
        let total_sats = PerBlock::import(db, "total_supply_sats", version, mappings)?;
        let total = LazyPerBlock::from_resolutions::<Convert>("total_supply", version, &total_sats);
        Ok(Self {
            total_sats,
            total,
            invariant_mismatch: PerBlock::import(
                db,
                "supply_invariant_mismatch",
                version,
                mappings,
            )?,
        })
    }

    pub fn compute(
        &mut self,
        from: Height,
        holders: &HoldersVecs,
        mweb: &MwebVecs,
        burned: &ValuePerBlockCumulativeRolling,
        exit: &Exit,
    ) -> Result<()> {
        let circulating = &holders.cohorts.all.supply.total.sats.height;
        let pool = &mweb.supply_sats.height;
        let burned = &burned.cumulative.sats.height;
        let version = circulating.version() + pool.version() + burned.version();
        let start = prepare_computed(
            [
                &mut self.total_sats.height as &mut dyn AnyStoredVec,
                &mut self.invariant_mismatch.height,
            ],
            version,
            usize::from(from),
            exit,
        )?;
        let end = circulating.len().min(pool.len()).min(burned.len());
        if start < end {
            let mut circulating = circulating.bounded_cursor_at(start, end);
            let mut pool = pool.bounded_cursor_at(start, end);
            let mut burned = burned.bounded_cursor_at(start, end);
            let mut issued = issued_through(start) - u64::from(Height::from(start).subsidy());
            for h in start..end {
                issued += u64::from(Height::from(h).subsidy());
                let total = circulating.next().data()? + pool.next().data()?;
                let expected = issued - u64::from(burned.next().data()?);
                self.total_sats.height.push(total);
                self.invariant_mismatch
                    .height
                    .push(SatsSigned::from(u64::from(total) as i64 - expected as i64));
            }
        }
        write_locked(
            [
                &mut self.total_sats.height as &mut dyn AnyStoredVec,
                &mut self.invariant_mismatch.height,
            ],
            exit,
        )
    }
}

/// Every block subsidy scheduled through block `height`, the genesis one included.
fn issued_through(height: usize) -> u64 {
    let halving = BLOCKS_PER_HALVING as usize;
    let mut issued = 0;
    let mut epoch_start = 0;
    while epoch_start <= height {
        let epoch_end = (epoch_start + halving).min(height + 1);
        issued += (epoch_end - epoch_start) as u64 * u64::from(Height::from(epoch_start).subsidy());
        epoch_start += halving;
    }
    issued
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn issued_through_spans_halvings() {
        let halving = BLOCKS_PER_HALVING as usize;
        let fifty = u64::from(Sats::FIFTY_BTC);
        assert_eq!(issued_through(0), fifty);
        assert_eq!(issued_through(halving - 1), halving as u64 * fifty);
        assert_eq!(issued_through(halving), halving as u64 * fifty + fifty / 2);
        let direct: u64 = (0..=halving + 9)
            .map(|h| u64::from(Height::from(h).subsidy()))
            .sum();
        assert_eq!(issued_through(halving + 9), direct);
    }
}
