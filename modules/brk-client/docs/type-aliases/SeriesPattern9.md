[**brk-client**](../README.md)

***

[brk-client](../globals.md) / SeriesPattern9

# Type Alias: SeriesPattern9\<T\>

> **SeriesPattern9**\<`T`\> = `object`

Defined in: [Developer/brk/modules/brk-client/index.js:2423](https://github.com/bitcoinresearchkit/brk/blob/7ab57b5341cb1b8e45327c9a1bba060f0ddd106d/modules/brk-client/index.js#L2423)

## Type Parameters

### T

`T`

## Type Declaration

### by

> **by**: `object`

#### by.day3

> `readonly` **day3**: [`DateSeriesEndpoint`](../interfaces/DateSeriesEndpoint.md)\<`T`\>

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
