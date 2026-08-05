[**brk-client**](../README.md)

***

[brk-client](../globals.md) / RangeBuilder

# Interface: RangeBuilder\<T\>

Defined in: [Developer/brk/modules/brk-client/index.js:1785](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L1785)

## Type Parameters

### T

`T`

## Properties

### fetch

> **fetch**: (`arg?`, `options?`) => `Promise`\<[`SeriesData`](../type-aliases/SeriesData.md)\<`T`\>\>

Defined in: [Developer/brk/modules/brk-client/index.js:1786](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L1786)

Fetch the range

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

Defined in: [Developer/brk/modules/brk-client/index.js:1787](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L1787)

Fetch as CSV

#### Parameters

##### options?

[`ClientFetchOptions`](ClientFetchOptions.md)\<`string`\>

#### Returns

`Promise`\<`string`\>

***

### then

> **then**: [`Thenable`](../type-aliases/Thenable.md)\<`T`\>

Defined in: [Developer/brk/modules/brk-client/index.js:1788](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L1788)

Thenable
