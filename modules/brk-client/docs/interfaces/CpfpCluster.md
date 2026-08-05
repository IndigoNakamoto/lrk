[**brk-client**](../README.md)

***

[brk-client](../globals.md) / CpfpCluster

# Interface: CpfpCluster

Defined in: [Developer/brk/modules/brk-client/index.js:420](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L420)

## Properties

### chunkIndex

> **chunkIndex**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:423](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L423)

Index into `chunks` of the chunk containing the seed tx.

***

### chunks

> **chunks**: [`CpfpClusterChunk`](CpfpClusterChunk.md)[]

Defined in: [Developer/brk/modules/brk-client/index.js:422](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L422)

SFL-emitted chunks ordered by descending feerate.

***

### txs

> **txs**: [`CpfpClusterTx`](CpfpClusterTx.md)[]

Defined in: [Developer/brk/modules/brk-client/index.js:421](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L421)

All txs in the cluster, in topological order (parents before children).
