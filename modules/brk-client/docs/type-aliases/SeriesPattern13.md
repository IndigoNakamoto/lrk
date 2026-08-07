[**brk-client**](../README.md)

***

[brk-client](../globals.md) / SeriesPattern13

# Type Alias: SeriesPattern13\<T\>

> **SeriesPattern13**\<`T`\> = `object`

Defined in: [Developer/brk/modules/brk-client/index.js:2435](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L2435)

## Type Parameters

### T

`T`

## Type Declaration

### by

> **by**: `object`

#### by.month6

> `readonly` **month6**: [`DateSeriesEndpoint`](../interfaces/DateSeriesEndpoint.md)\<`T`\>

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
