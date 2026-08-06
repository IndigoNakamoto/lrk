[**brk-client**](../README.md)

***

[brk-client](../globals.md) / SeriesPattern3

# Type Alias: SeriesPattern3\<T\>

> **SeriesPattern3**\<`T`\> = `object`

Defined in: [Developer/brk/modules/brk-client/index.js:2405](https://github.com/bitcoinresearchkit/brk/blob/f363f65275d140f73b1a471070edcc576fd24a17/modules/brk-client/index.js#L2405)

## Type Parameters

### T

`T`

## Type Declaration

### by

> **by**: `object`

#### by.minute10

> `readonly` **minute10**: [`DateSeriesEndpoint`](../interfaces/DateSeriesEndpoint.md)\<`T`\>

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
