[**brk-client**](../README.md)

***

[brk-client](../globals.md) / DetailedSeriesCount

# Interface: DetailedSeriesCount

Defined in: [Developer/brk/modules/brk-client/index.js:495](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L495)

## Properties

### byDb

> **byDb**: `object`

Defined in: [Developer/brk/modules/brk-client/index.js:500](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L500)

Per-database breakdown of counts

#### Index Signature

\[`key`: `string`\]: [`SeriesCount`](SeriesCount.md)

***

### distinct

> **distinct**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:496](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L496)

Number of unique series available (e.g., realized_price, market_cap)

***

### lazy

> **lazy**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:498](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L498)

Number of lazy (computed on-the-fly) series-index combinations

***

### stored

> **stored**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:499](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L499)

Number of eager (stored on disk) series-index combinations

***

### total

> **total**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:497](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L497)

Total number of series-index combinations across all timeframes
