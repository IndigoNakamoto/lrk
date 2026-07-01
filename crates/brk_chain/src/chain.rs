use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

/// The blockchain this BRK instance is connected to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
pub enum Chain {
    #[default]
    Bitcoin,
    Litecoin,
}

impl Chain {
    pub fn constants(self) -> ChainConstants {
        match self {
            Chain::Bitcoin => ChainConstants::BITCOIN,
            Chain::Litecoin => ChainConstants::LITECOIN,
        }
    }

    /// Returns false when the on-chain price oracle is not yet calibrated for
    /// this chain. Exchange-rate data from `brk_fetcher` is still available.
    pub fn supports_oracle(self) -> bool {
        matches!(self, Chain::Bitcoin)
    }
}

impl std::fmt::Display for Chain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Chain::Bitcoin => f.write_str("bitcoin"),
            Chain::Litecoin => f.write_str("litecoin"),
        }
    }
}

impl std::str::FromStr for Chain {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "bitcoin" | "btc" => Ok(Chain::Bitcoin),
            "litecoin" | "ltc" => Ok(Chain::Litecoin),
            other => Err(format!("unknown chain '{other}'; expected 'bitcoin' or 'litecoin'")),
        }
    }
}

/// Protocol and deployment constants that differ between chains.
#[derive(Debug, Clone, Copy)]
pub struct ChainConstants {
    /// Number of blocks between reward halvings (BTC: 210,000 · LTC: 840,000).
    pub blocks_per_halving: u32,
    /// Number of blocks per difficulty-retarget epoch (both chains: 2,016).
    pub blocks_per_diff_epoch: u32,
    /// Target seconds per block (BTC: 600 · LTC: 150).
    pub seconds_per_block: u64,
    /// Initial block subsidy in smallest coin units (both chains: 50 × 10⁸).
    pub initial_subsidy: u64,
    /// Approximate blocks mined per year derived from `seconds_per_block`.
    pub blocks_per_year: u64,
    /// Unix timestamp of the genesis block.
    pub genesis_timestamp: u32,
    /// Unix timestamp of the index epoch start (the calendar day of genesis).
    pub index_epoch: u32,
    /// Calendar year of the genesis block (used for cohort start year).
    pub genesis_year: u16,
    /// Default JSON-RPC port for the node daemon.
    pub default_rpc_port: u16,
    /// Default data directory name segment on Linux (`~/<segment>`).
    pub default_datadir_linux: &'static str,
    /// Default data directory path segment on macOS
    /// (`~/Library/Application Support/<segment>`).
    pub default_datadir_mac: &'static str,
    /// Short ticker symbol.
    pub ticker: &'static str,
    /// Coin name for display.
    pub coin_name: &'static str,
    /// BIP30 duplicate coinbase `(duplicate_height, original_height)` pairs.
    /// The duplicate block contains a coinbase txid that collides with the one
    /// at `original_height`. Empty for chains that have no BIP30 duplicates.
    pub bip30_duplicate_heights: &'static [(u32, u32)],
    /// Binance spot symbol (e.g. `"BTCUSDT"`, `"LTCUSDT"`).
    pub binance_symbol: &'static str,
    /// Kraken OHLC endpoint pair name (e.g. `"XBTUSD"`, `"LTCUSD"`).
    pub kraken_pair: &'static str,
    /// Kraken result-object key in the OHLC response (e.g. `"XXBTZUSD"`, `"XLTCZUSD"`).
    pub kraken_result_key: &'static str,
    /// Coinbase Exchange product id for daily-history backfill (e.g. `"BTC-USD"`,
    /// `"LTC-USD"`). Unlike Binance/Kraken, Coinbase serves full paginated daily
    /// history, so it is the primary source for chains without an on-chain oracle.
    pub coinbase_product: &'static str,
    /// Calendar month (1 = Jan) of the `index_epoch` date. Used by client generators.
    pub index_epoch_month: u8,
    /// Calendar day of the `index_epoch` date. Used by client generators.
    pub index_epoch_day: u8,
}

impl ChainConstants {
    pub const BITCOIN: Self = Self {
        blocks_per_halving: 210_000,
        blocks_per_diff_epoch: 2_016,
        seconds_per_block: 600,
        initial_subsidy: 50 * 100_000_000,
        blocks_per_year: 52_560,
        genesis_timestamp: 1_231_006_505,
        index_epoch: 1_230_768_000, // 2009-01-01 00:00:00 UTC
        genesis_year: 2009,
        default_rpc_port: 8332,
        default_datadir_linux: ".bitcoin",
        default_datadir_mac: "Bitcoin",
        ticker: "BTC",
        coin_name: "Bitcoin",
        bip30_duplicate_heights: &[
            (91_842, 91_812), // block 91_842 coinbase duplicates 91_812
            (91_880, 91_722), // block 91_880 coinbase duplicates 91_722
        ],
        binance_symbol: "BTCUSDT",
        kraken_pair: "XBTUSD",
        kraken_result_key: "XXBTZUSD",
        coinbase_product: "BTC-USD",
        index_epoch_month: 1, // 2009-01-01
        index_epoch_day: 1,
    };

    pub const LITECOIN: Self = Self {
        blocks_per_halving: 840_000,
        blocks_per_diff_epoch: 2_016,
        seconds_per_block: 150,
        initial_subsidy: 50 * 100_000_000,
        blocks_per_year: 210_240,
        genesis_timestamp: 1_317_972_665,
        index_epoch: 1_317_600_000, // 2011-10-03 00:00:00 UTC
        genesis_year: 2011,
        default_rpc_port: 9332,
        default_datadir_linux: ".litecoin",
        default_datadir_mac: "Litecoin",
        ticker: "LTC",
        coin_name: "Litecoin",
        bip30_duplicate_heights: &[],
        binance_symbol: "LTCUSDT",
        kraken_pair: "LTCUSD",
        kraken_result_key: "XLTCZUSD",
        coinbase_product: "LTC-USD",
        index_epoch_month: 10, // 2011-10-03
        index_epoch_day: 3,
    };
}
