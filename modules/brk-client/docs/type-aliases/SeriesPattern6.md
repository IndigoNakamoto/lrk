[**brk-client**](../README.md)

***

[brk-client](../globals.md) / SeriesPattern6

# Type Alias: SeriesPattern6\<T\>

> **SeriesPattern6**\<`T`\> = `object`

Defined in: [Developer/brk/modules/brk-client/index.js:2414](https://github.com/bitcoinresearchkit/brk/blob/f363f65275d140f73b1a471070edcc576fd24a17/modules/brk-client/index.js#L2414)

## Type Parameters

### T

`T`

## Type Declaration

### by

> **by**: `object`

#### by.hour4

> `readonly` **hour4**: [`DateSeriesEndpoint`](../interfaces/DateSeriesEndpoint.md)\<`T`\>

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
