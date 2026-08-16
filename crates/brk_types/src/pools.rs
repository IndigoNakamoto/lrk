use std::sync::OnceLock;

use serde::Deserialize;

use crate::{Chain, PoolSlug, Version};

use super::Pool;

/// Increment when pool IDs, payout addresses, or coinbase tags change.
pub const POOL_ATTRIBUTION_VERSION: Version = Version::ONE;

const JSON_BTC: &str = include_str!("../pools-v2.json");
const JSON_LTC: &str = include_str!("../pools-ltc-v1.json");
const POOL_COUNT_BTC: usize = 171;
/// Array capacity for LTC pools. Entries (from `pools-ltc-v1.json`) map to
/// `PoolSlug` discriminants: shared pools reuse existing variants and
/// Litecoin-specific pools occupy ids 171-189, so the backing array must span
/// the full `PoolSlug` range. `Pools::len()` reports the actual pool count.
const POOL_COUNT_LTC: usize = 256;
const TESTNET_IDS: &[u16] = &[145, 146, 149, 150, 156, 163];

#[derive(Deserialize)]
struct JsonPoolEntry {
    id: u16,
    name: String,
    #[serde(rename = "addresses")]
    addrs: Vec<String>,
    tags: Vec<String>,
    link: String,
}

fn leak_str(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

fn empty_pool(id: usize) -> Pool {
    Pool {
        slug: PoolSlug::from(id as u8),
        name: "",
        addrs: Box::new([]),
        tags: Box::new([]),
        tags_lowercase: Box::new([]),
        link: "",
    }
}

#[derive(Debug)]
pub struct Pools {
    entries: Vec<Pool>,
    /// Number of non-testnet entries (used by `len()`).
    count: usize,
}

impl Pools {
    pub fn find_from_coinbase_tag(&self, coinbase_tag: &str) -> Option<&Pool> {
        let coinbase_tag = coinbase_tag.to_lowercase();
        self.iter().find(|pool| {
            pool.tags_lowercase
                .iter()
                .any(|pool_tag| coinbase_tag.contains(pool_tag))
        })
    }

    pub fn find_from_addr(&self, addr: &str) -> Option<&Pool> {
        self.iter().find(|pool| pool.addrs.contains(&addr))
    }

    pub fn get_unknown(&self) -> &Pool {
        &self.entries[0]
    }

    pub fn get(&self, slug: PoolSlug) -> &Pool {
        let i: u8 = slug.into();
        &self.entries[i as usize]
    }

    pub fn iter(&self) -> impl Iterator<Item = &Pool> + '_ {
        self.entries.iter().filter(|p| !p.name.is_empty())
    }

    #[allow(clippy::len_without_is_empty)]
    pub fn len(&self) -> usize {
        self.count
    }
}

fn parse_pools(json: &str, pool_count: usize, skip_ids: &[u16]) -> Pools {
    let entries: Vec<JsonPoolEntry> =
        serde_json::from_str(json).expect("Failed to parse pools JSON");

    let max_id = entries.iter().map(|entry| entry.id).max().unwrap_or(0);
    assert!(
        max_id <= u8::MAX as u16,
        "pool ID {max_id} exceeds PoolSlug's u8 range"
    );
    let mut pools: Vec<Pool> = (0..pool_count.max(usize::from(max_id) + 1))
        .map(empty_pool)
        .collect();

    pools[0] = Pool {
        slug: PoolSlug::Unknown,
        name: "Unknown",
        addrs: Box::new([]),
        tags: Box::new([]),
        tags_lowercase: Box::new([]),
        link: "",
    };

    let mut count = 1; // Unknown counts
    for entry in entries {
        if skip_ids.contains(&entry.id) {
            continue;
        }
        let id = entry.id as usize;
        let slug = PoolSlug::from(id as u8);
        let tags_lowercase = entry
            .tags
            .iter()
            .map(|t| t.to_lowercase())
            .collect::<Vec<_>>()
            .into_boxed_slice();
        pools[id] = Pool {
            slug,
            name: leak_str(entry.name),
            link: leak_str(entry.link),
            addrs: entry
                .addrs
                .into_iter()
                .map(leak_str)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            tags: entry
                .tags
                .into_iter()
                .map(leak_str)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            tags_lowercase,
        };
        count += 1;
    }

    Pools { entries: pools, count }
}

pub fn pools() -> &'static Pools {
    pools_for_chain(Chain::Bitcoin)
}

pub fn pools_for_chain(chain: Chain) -> &'static Pools {
    match chain {
        Chain::Bitcoin => {
            static POOLS_BTC: OnceLock<Pools> = OnceLock::new();
            POOLS_BTC.get_or_init(|| {
                parse_pools(JSON_BTC, POOL_COUNT_BTC, TESTNET_IDS)
            })
        }
        Chain::Litecoin => {
            static POOLS_LTC: OnceLock<Pools> = OnceLock::new();
            POOLS_LTC.get_or_init(|| {
                parse_pools(JSON_LTC, POOL_COUNT_LTC, &[])
            })
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_json_entries_have_named_slugs() {
        let entries: Vec<JsonPoolEntry> =
            serde_json::from_str(JSON_BTC).expect("valid pools-v2.json");

        for entry in entries {
            if TESTNET_IDS.contains(&entry.id) {
                continue;
            }
            let id = u8::try_from(entry.id).expect("pool ID fits PoolSlug");
            let slug = PoolSlug::from(id);
            assert!(
                serde_json::to_string(&slug).is_ok(),
                "pool ID {} ({}) still maps to {slug:?}",
                entry.id,
                entry.name
            );
        }
    }

    #[test]
    fn dmnd_uses_upstream_id_171() {
        let dmnd = pools().get(PoolSlug::Dmnd);
        assert_eq!(dmnd.name, "DMND");
        assert_eq!(dmnd.mempool_unique_id(), 171);
    }
}
