[**brk-client**](../README.md)

***

[brk-client](../globals.md) / DetailedSeriesCount

# Interface: DetailedSeriesCount

Defined in: [Developer/brk/modules/brk-client/index.js:487](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L487)

## Properties

### byDb

> **byDb**: `object`

Defined in: [Developer/brk/modules/brk-client/index.js:492](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L492)

Per-database breakdown of counts

#### Index Signature

\[`key`: `string`\]: [`SeriesCount`](SeriesCount.md)

***

### distinctSeries

> **distinctSeries**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:488](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L488)

Number of unique series available (e.g., realized_price, market_cap)

***

### lazyEndpoints

> **lazyEndpoints**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:490](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L490)

Number of lazy (computed on-the-fly) series-index combinations

***

### storedEndpoints

> **storedEndpoints**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:491](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L491)

Number of eager (stored on disk) series-index combinations

***

### totalEndpoints

> **totalEndpoints**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:489](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L489)

Total number of series-index combinations across all timeframes
