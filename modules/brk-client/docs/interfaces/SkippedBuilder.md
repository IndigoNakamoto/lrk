[**brk-client**](../README.md)

***

[brk-client](../globals.md) / SkippedBuilder

# Interface: SkippedBuilder\<T\>

Defined in: [Developer/brk/modules/brk-client/index.js:1771](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L1771)

## Type Parameters

### T

`T`

## Properties

### fetch

> **fetch**: (`arg?`, `options?`) => `Promise`\<[`SeriesData`](../type-aliases/SeriesData.md)\<`T`\>\>

Defined in: [Developer/brk/modules/brk-client/index.js:1773](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L1773)

Fetch from skipped position to end

#### Parameters

##### arg?

[`SeriesFetchArg`](../type-aliases/SeriesFetchArg.md)\<`T`\>

##### options?

[`ClientFetchOptions`](ClientFetchOptions.md)\<[`SeriesData`](../type-aliases/SeriesData.md)\<`T`\>\>

#### Returns

`Promise`\<[`SeriesData`](../type-aliases/SeriesData.md)\<`T`\>\>

***

### fetchCsv

> **fetchCsv**: (`options?`) => `Promise`\<`string`\>

Defined in: [Developer/brk/modules/brk-client/index.js:1774](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L1774)

Fetch as CSV

#### Parameters

##### options?

[`ClientFetchOptions`](ClientFetchOptions.md)\<`string`\>

#### Returns

`Promise`\<`string`\>

***

### take

> **take**: (`n`) => [`RangeBuilder`](RangeBuilder.md)\<`T`\>

Defined in: [Developer/brk/modules/brk-client/index.js:1772](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L1772)

Take n items after skipped position

#### Parameters

##### n

`number`

#### Returns

[`RangeBuilder`](RangeBuilder.md)\<`T`\>

***

### then

> **then**: [`Thenable`](../type-aliases/Thenable.md)\<`T`\>

Defined in: [Developer/brk/modules/brk-client/index.js:1775](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L1775)

Thenable
