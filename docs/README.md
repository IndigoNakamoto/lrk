# Litecoin Research Kit

[![MIT Licensed](https://img.shields.io/badge/license-MIT-blue.svg)](https://github.com/IndigoNakamoto/lrk/blob/main/docs/LICENSE.md)

> A Litecoin fork of [Bitcoin Research Kit](https://github.com/bitcoinresearchkit/brk) (BRK).

Open-source Litecoin data toolkit that can parse blocks, index the chain, compute metrics, serve data, and render it — all from a Litecoin Core node. It combines what on-chain analytics providers and block explorers do separately into a single self-hostable package, with historical USD pricing from exchange data (Bitfinex and Coinbase) and live price from your mempool.

[litview](https://litview.space) is the official free hosted instance of LRK.

## Data

**Zero external dependencies.** LRK needs only a Litecoin Core node. 8,000+ metrics across 15 time resolutions, all computed locally from your own copy of the blockchain. Historical LTC/USD prices are built in (Bitfinex back to 2013, Coinbase from 2016), with live price from your mempool. Your node, your data.

**Blockchain:** Blocks, transactions, addresses, UTXOs — including MWEB (MimbleWimble Extension Blocks).

**Metrics:** Supply distributions, holder cohorts, network activity, fee markets, mining, and market indicators.

**Indexes:** Date, height, halving epoch, address type, UTXO age.

**Mempool:** Fee estimation, projected blocks, unconfirmed transactions.

**Mining pools:** Pool attribution from the [litecoinspace](https://litecoinspace.org) mining-pool list.

## Usage

### Website

Browse metrics and charts at [litview.space](https://litview.space), no signup required.

### API

```bash
curl https://litview.space/api/mempool/price
```

Query metrics and blockchain data in JSON or CSV. No rate limit.

[Documentation](https://litview.space/api) · [JavaScript](https://www.npmjs.com/package/brk-client) · [Python](https://pypi.org/project/brk-client) · [Rust](https://crates.io/crates/brk_client) · [llms.txt](https://litview.space/llms.txt) · [LLM-friendly schema](https://litview.space/api.json)

### Self-host

LRK must be built with the `litecoin` Cargo feature so the MWEB-aware block/transaction decoder is selected at compile time. A default (Bitcoin) build will fail to index once it reaches the first MWEB block.

```bash
git clone https://github.com/IndigoNakamoto/lrk.git && cd lrk
cargo install --locked --path crates/brk_cli --features litecoin
brk --chain litecoin
```

Run your own website and API. All you need is Litecoin Core with `server=1`.

> **Note:** LRK uses [sparse files](https://en.wikipedia.org/wiki/Sparse_file). Tools like `ls -l` or Finder report the logical file size, not actual disk usage (~100 GB for Litecoin). Use `du -sh` to see real usage.

[Guide](https://github.com/IndigoNakamoto/lrk/blob/main/crates/brk_cli/README.md) · [Professional hosting](./PROFESSIONAL_HOSTING.md)

### Library

```bash
cargo add brk --features litecoin
```

Build custom applications in Rust. Use the full stack or individual components (parser, indexer, computer, server).

[Reference](https://docs.rs/brk) · [Architecture](./ARCHITECTURE.md)

## Links

- [Changelog](./CHANGELOG.md)
- [Contributing](https://github.com/IndigoNakamoto/lrk/issues)
- [Upstream BRK](https://github.com/bitcoinresearchkit/brk)

## License

[MIT](./LICENSE.md)
