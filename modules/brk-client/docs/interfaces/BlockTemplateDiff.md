[**brk-client**](../README.md)

***

[brk-client](../globals.md) / BlockTemplateDiff

# Interface: BlockTemplateDiff

Defined in: [Developer/brk/modules/brk-client/index.js:293](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L293)

## Properties

### hash

> **hash**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:294](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L294)

Current next-block hash. Use as `since` on the next diff call.

***

### order

> **order**: [`BlockTemplateDiffEntry`](../type-aliases/BlockTemplateDiffEntry.md)[]

Defined in: [Developer/brk/modules/brk-client/index.js:296](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L296)

New template in order. Each entry is either an index into the
prior template's transactions or a full transaction body.

***

### removed

> **removed**: `string`[]

Defined in: [Developer/brk/modules/brk-client/index.js:298](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L298)

Txids that left the projected next block since `since`
(confirmed, evicted, replaced, or pushed past block 0).

***

### since

> **since**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:295](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L295)

Echoed prior hash the diff was computed against.
