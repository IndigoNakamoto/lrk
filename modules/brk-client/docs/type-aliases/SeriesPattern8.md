[**brk-client**](../README.md)

***

[brk-client](../globals.md) / SeriesPattern8

# Type Alias: SeriesPattern8\<T\>

> **SeriesPattern8**\<`T`\> = `object`

Defined in: [Developer/brk/modules/brk-client/index.js:2420](https://github.com/bitcoinresearchkit/brk/blob/f363f65275d140f73b1a471070edcc576fd24a17/modules/brk-client/index.js#L2420)

## Type Parameters

### T

`T`

## Type Declaration

### by

> **by**: `object`

#### by.day1

> `readonly` **day1**: [`DateSeriesEndpoint`](../interfaces/DateSeriesEndpoint.md)\<`T`\>

### get

> **get**: (`index`) => [`SeriesEndpoint`](../interfaces/SeriesEndpoint.md)\<`T`\> \| `undefined`

#### Parameters

##### index

[`Index`](Index.md)

#### Returns

[`SeriesEndpoint`](../interfaces/SeriesEndpoint.md)\<`T`\> \| `undefined`

### indexes

> **indexes**: () => readonly [`Index`](Index.md)[]

#### Returns

readonly [`Index`](Index.md)[]

### name

> **name**: `string`
