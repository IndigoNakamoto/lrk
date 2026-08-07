[**brk-client**](../README.md)

***

[brk-client](../globals.md) / BlockTemplate

# Interface: BlockTemplate

Defined in: [Developer/brk/modules/brk-client/index.js:274](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L274)

## Properties

### hash

> **hash**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:275](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L275)

Pass to `GET /api/v1/mempool/block-template/diff/{hash}` to fetch deltas.

***

### stats

> **stats**: [`MempoolBlock`](MempoolBlock.md)

Defined in: [Developer/brk/modules/brk-client/index.js:276](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L276)

Aggregate stats for this block (size, vsize, fee range, ...).

***

### transactions

> **transactions**: [`Transaction`](Transaction.md)[]

Defined in: [Developer/brk/modules/brk-client/index.js:277](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L277)

Full transaction bodies in `getblocktemplate` order.
