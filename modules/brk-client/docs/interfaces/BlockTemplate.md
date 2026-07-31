[**brk-client**](../README.md)

***

[brk-client](../globals.md) / BlockTemplate

# Interface: BlockTemplate

Defined in: [Developer/brk/modules/brk-client/index.js:273](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L273)

## Properties

### hash

> **hash**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:274](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L274)

Pass back as `<hash>` on
`/api/v1/mempool/block-template/diff/{hash}` to fetch deltas.

***

### stats

> **stats**: [`MempoolBlock`](MempoolBlock.md)

Defined in: [Developer/brk/modules/brk-client/index.js:276](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L276)

Aggregate stats for this block (size, vsize, fee range, ...).

***

### transactions

> **transactions**: [`Transaction`](Transaction.md)[]

Defined in: [Developer/brk/modules/brk-client/index.js:277](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L277)

Full transaction bodies in `getblocktemplate` order.
