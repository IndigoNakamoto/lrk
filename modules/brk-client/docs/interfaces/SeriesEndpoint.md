[**brk-client**](../README.md)

***

[brk-client](../globals.md) / SeriesEndpoint

# Interface: SeriesEndpoint\<T\>

Defined in: [Developer/brk/modules/brk-client/index.js:1692](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L1692)

## Type Parameters

### T

`T`

## Properties

### fetch

> **fetch**: (`arg?`, `options?`) => `Promise`\<[`SeriesData`](../type-aliases/SeriesData.md)\<`T`\>\>

Defined in: [Developer/brk/modules/brk-client/index.js:1698](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L1698)

Fetch all data

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

Defined in: [Developer/brk/modules/brk-client/index.js:1699](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L1699)

Fetch all data as CSV

#### Parameters

##### options?

[`ClientFetchOptions`](ClientFetchOptions.md)\<`string`\>

#### Returns

`Promise`\<`string`\>

***

### first

> **first**: (`n`) => [`RangeBuilder`](RangeBuilder.md)\<`T`\>

Defined in: [Developer/brk/modules/brk-client/index.js:1695](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L1695)

Get first n items

#### Parameters

##### n

`number`

#### Returns

[`RangeBuilder`](RangeBuilder.md)\<`T`\>

***

### get

> **get**: (`index`) => [`SingleItemBuilder`](SingleItemBuilder.md)\<`T`\>

Defined in: [Developer/brk/modules/brk-client/index.js:1693](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L1693)

Get single item at index

#### Parameters

##### index

`number`

#### Returns

[`SingleItemBuilder`](SingleItemBuilder.md)\<`T`\>

***

### last

> **last**: (`n`) => [`RangeBuilder`](RangeBuilder.md)\<`T`\>

Defined in: [Developer/brk/modules/brk-client/index.js:1696](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L1696)

Get last n items

#### Parameters

##### n

`number`

#### Returns

[`RangeBuilder`](RangeBuilder.md)\<`T`\>

***

### len

> **len**: () => `Promise`\<`number`\>

Defined in: [Developer/brk/modules/brk-client/index.js:1700](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L1700)

Get total number of data points

#### Returns

`Promise`\<`number`\>

***

### path

> **path**: `string`

Defined in: [Developer/brk/modules/brk-client/index.js:1703](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L1703)

The endpoint path

***

### skip

> **skip**: (`n`) => [`SkippedBuilder`](SkippedBuilder.md)\<`T`\>

Defined in: [Developer/brk/modules/brk-client/index.js:1697](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L1697)

Skip first n items, chain with take()

#### Parameters

##### n

`number`

#### Returns

[`SkippedBuilder`](SkippedBuilder.md)\<`T`\>

***

### slice

> **slice**: (`start?`, `end?`) => [`RangeBuilder`](RangeBuilder.md)\<`T`\>

Defined in: [Developer/brk/modules/brk-client/index.js:1694](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L1694)

Slice by index

#### Parameters

##### start?

`number`

##### end?

`number`

#### Returns

[`RangeBuilder`](RangeBuilder.md)\<`T`\>

***

### then

> **then**: [`Thenable`](../type-aliases/Thenable.md)\<`T`\>

Defined in: [Developer/brk/modules/brk-client/index.js:1702](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L1702)

Thenable (await endpoint)

***

### version

> **version**: () => `Promise`\<`number`\>

Defined in: [Developer/brk/modules/brk-client/index.js:1701](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L1701)

Get the current version of the series

#### Returns

`Promise`\<`number`\>
