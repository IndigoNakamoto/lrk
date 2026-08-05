[**brk-client**](../README.md)

***

[brk-client](../globals.md) / CpfpInfo

# Interface: CpfpInfo

Defined in: [Developer/brk/modules/brk-client/index.js:460](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L460)

## Properties

### adjustedVsize

> **adjustedVsize**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:471](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L471)

Policy-adjusted virtual size: `max(vsize, sigops * 5)`.

***

### ancestors

> **ancestors**: [`CpfpEntry`](CpfpEntry.md)[]

Defined in: [Developer/brk/modules/brk-client/index.js:461](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L461)

Ancestor transactions in the CPFP chain.

***

### bestDescendant?

> `optional` **bestDescendant?**: [`CpfpEntry`](CpfpEntry.md) \| `null`

Defined in: [Developer/brk/modules/brk-client/index.js:462](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L462)

Best (highest fee rate) descendant, if any.

***

### cluster?

> `optional` **cluster?**: [`CpfpCluster`](CpfpCluster.md) \| `null`

Defined in: [Developer/brk/modules/brk-client/index.js:472](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L472)

Cluster the seed belongs to: full tx list, SFL-linearized chunks,
and the seed's chunk index. Omitted when the seed has no
ancestors and no descendants (matches mempool.space).

***

### descendants

> **descendants**: [`CpfpEntry`](CpfpEntry.md)[]

Defined in: [Developer/brk/modules/brk-client/index.js:463](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L463)

Descendant transactions in the CPFP chain.

***

### effectiveFeePerVsize

> **effectiveFeePerVsize**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:464](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L464)

Effective fee rate considering CPFP relationships (sat/vB).
This is the seed's chunk feerate after lift-merging, i.e. the
rate Core/mempool.space would surface for this tx.

***

### fee

> **fee**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:469](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L469)

Transaction fee (sats).

***

### sigops

> **sigops**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:467](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L467)

BIP-141 sigop cost for the seed tx (witness sigops count as 1,
legacy and P2SH-redeem sigops count as 4).

***

### vsize

> **vsize**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:470](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L470)

Virtual size of the seed tx (vbytes).
