[**brk-client**](../README.md)

***

[brk-client](../globals.md) / SeriesPattern32

# Type Alias: SeriesPattern32\<T\>

> **SeriesPattern32**\<`T`\> = `object`

Defined in: [Developer/brk/modules/brk-client/index.js:2492](https://github.com/bitcoinresearchkit/brk/blob/7ab57b5341cb1b8e45327c9a1bba060f0ddd106d/modules/brk-client/index.js#L2492)

## Type Parameters

### T

`T`

## Type Declaration

### by

> **by**: `object`

#### by.p2wsh\_addr\_index

> `readonly` **p2wsh\_addr\_index**: [`SeriesEndpoint`](../interfaces/SeriesEndpoint.md)\<`T`\>

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
