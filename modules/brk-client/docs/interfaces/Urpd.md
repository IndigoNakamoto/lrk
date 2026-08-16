[**brk-client**](../README.md)

***

[brk-client](../globals.md) / Urpd

# Interface: Urpd

Defined in: [Developer/brk/modules/brk-client/index.js:1377](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1377)

## Properties

### aggregation

> **aggregation**: [`UrpdAggregation`](../type-aliases/UrpdAggregation.md)

Defined in: [Developer/brk/modules/brk-client/index.js:1381](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1381)

Aggregation strategy applied to the buckets.

***

### buckets

> **buckets**: [`UrpdBucket`](UrpdBucket.md)[]

Defined in: [Developer/brk/modules/brk-client/index.js:1384](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1384)

***

### close

> **close**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:1382](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1382)

Close price on `date`, in USD. Anchor for `unrealized_pnl`.

***

### cohort

> **cohort**: [`Cohort`](../type-aliases/Cohort.md)

Defined in: [Developer/brk/modules/brk-client/index.js:1378](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1378)

***

### date

> **date**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:1379](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1379)

***

### totalSupply

> **totalSupply**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:1383](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1383)

Sum of `supply` across all buckets, in BTC.

***

### weight

> **weight**: [`UrpdWeight`](../type-aliases/UrpdWeight.md)

Defined in: [Developer/brk/modules/brk-client/index.js:1380](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1380)

Weighting applied to the source supply.
