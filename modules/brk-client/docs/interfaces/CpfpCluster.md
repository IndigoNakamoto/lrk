[**brk-client**](../README.md)

***

[brk-client](../globals.md) / CpfpCluster

# Interface: CpfpCluster

Defined in: [Developer/brk/modules/brk-client/index.js:412](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L412)

## Properties

### chunkIndex

> **chunkIndex**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:415](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L415)

Index into `chunks` of the chunk containing the seed tx.

***

### chunks

> **chunks**: [`CpfpClusterChunk`](CpfpClusterChunk.md)[]

Defined in: [Developer/brk/modules/brk-client/index.js:414](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L414)

SFL-emitted chunks ordered by descending feerate.

***

### txs

> **txs**: [`CpfpClusterTx`](CpfpClusterTx.md)[]

Defined in: [Developer/brk/modules/brk-client/index.js:413](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L413)

All txs in the cluster, in topological order (parents before children).
