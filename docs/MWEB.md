# MWEB in LRK

Litecoin’s Mimblewimble Extension Block (MWEB) is an opt-in side structure
anchored to the L1 chain by a special final transaction, **HogEx**. LRK indexes
the peg edge and (Phase 3) compact extension-block summaries for analytics on
[litview.space](https://litview.space). It is **not** an MWEB wallet.

Build with `--features litecoin`. A Bitcoin-shaped build fails at the first
MWEB block.

## Canonical L1 bridge model

| Piece | Meaning in LRK |
|-------|----------------|
| Witness **v8** / HogAddr | Peg-pool output (`OutputType::MWEBPegPool`). Unspendable; not an address. |
| Witness **v9** | Peg-in output (`OutputType::MWEBPegIn`). Unspendable; ~0 steady-state balance. |
| HogEx | Last L1 tx in an MWEB-carrying block (`is_hog_ex`). `vout[0]` is the v8 pool; later transparent `vout`s are peg-outs. |
| Peg-out maturity | 6 blocks before a peg-out UTXO is spendable (consensus). Balance metrics ignore maturity. |

Pegged supply is recovered from L1 unspendable MWEB outs:

`mweb_balance ≈ cumulative(mweb_outputs_value) − cumulative(mweb_inputs_value)`.

Circulating supply = transparent UTXO set + MWEB pegged balance.

## Extension block (Phase 3)

`brk_reader` decodes trailing `mweb_block` after L1 txs when the header version
bit `0x20000000` is set and the last tx is HogEx. The indexer stores per-height
summaries (not full stealth payloads):

- `mweb_input_count` / `mweb_output_count` / `mweb_kernel_count`
- `mweb_fee` — sum of kernel fee fields
- `mweb_kernel_pegin` / `mweb_kernel_pegout` — kernel-declared peg amounts
- `mweb_recon_delta` — `|L1 peg-in − kernel peg-in| + |L1 peg-out − kernel peg-out|`

Kernel/input/output counts are **aggregated body activity**, not “user payment”
counts (aggregation + cut-through collapse same-block create/spend).

## Identifier quirks (mempool / fees)

Learned from the MWEB harness and Litecoin Core behavior:

- **MWEB-only txs** can have empty L1 `vin`/`vout` with the body in `mweb_tx`.
- **txid** for MWEB-only txs is the first sorted kernel hash.
- **wtxid excludes the MWEB body** (`SERIALIZE_NO_MWEB`), so Bitcoin’s
  “wtxid covers all malleable data” assumption does not hold.
- L1 **weight/vsize can be 0**. LRK treats that as undefined sat/vB (rate `0`)
  and excludes those txs from sat/vB packing / fee percentiles.

## What LRK does not index

- Stealth addresses, scan keys, or owner rewind
- Full PMMR / leafset / light-client segment payloads
- Mempool aggregation censorship or Core crash surfaces (see the
  `litecoin-mweb-harness` audit if you operate Litecoin Core)

## Related code

- Reader: `crates/brk_reader/src/parse.rs`
- Indexer summaries: `crates/brk_indexer/src/mweb_summary.rs`, `vecs/blocks.rs`
- Peg + Phase 3 metrics: `crates/brk_computer/src/outputs/mweb/`
- HogEx metrics: `crates/brk_computer/src/transactions/hogex/`
- Charts: `website/scripts/options/network.js` (MWEB section)
- API list: `website/llms-full.txt` (MWEB series)

## Source of model truth

Operational MWEB consensus/P2P findings that informed this model live in the
sibling harness repo (`litecoin-mweb-harness`): `ledger/hypotheses.md`,
`audit/gist-mweb-findings-for-core.md`, and `findings/`.
