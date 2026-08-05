[**brk-client**](../README.md)

***

[brk-client](../globals.md) / SingleItemBuilder

# Interface: SingleItemBuilder\<T\>

Defined in: [Developer/brk/modules/brk-client/index.js:1759](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L1759)

## Type Parameters

### T

`T`

## Properties

### fetch

> **fetch**: (`arg?`, `options?`) => `Promise`\<[`SeriesData`](../type-aliases/SeriesData.md)\<`T`\>\>

Defined in: [Developer/brk/modules/brk-client/index.js:1760](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L1760)

Fetch the item

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

Defined in: [Developer/brk/modules/brk-client/index.js:1761](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L1761)

Fetch as CSV

#### Parameters

##### options?

[`ClientFetchOptions`](ClientFetchOptions.md)\<`string`\>

#### Returns

`Promise`\<`string`\>

***

### then

> **then**: [`Thenable`](../type-aliases/Thenable.md)\<`T`\>

Defined in: [Developer/brk/modules/brk-client/index.js:1762](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L1762)

Thenable
