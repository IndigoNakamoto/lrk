use std::sync::OnceLock;

use serde::Deserialize;

use crate::{Chain, PoolSlug};

use super::Pool;

const JSON_BTC: &str = include_str!("../pools-v2.json");
const JSON_LTC: &str = include_str!("../pools-ltc-v1.json");
const POOL_COUNT_BTC: usize = 171;
/// LTC pool count: entries 1-15 plus the Unknown slot at 0.
const POOL_COUNT_LTC: usize = 16;
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

    let mut pools: Vec<Pool> = (0..pool_count).map(empty_pool).collect();

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
