[**brk-client**](../README.md)

***

[brk-client](../globals.md) / SeriesPattern17

# Type Alias: SeriesPattern17\<T\>

> **SeriesPattern17**\<`T`\> = `object`

Defined in: [Developer/brk/modules/brk-client/index.js:2447](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L2447)

## Type Parameters

### T

`T`

## Type Declaration

### by

> **by**: `object`

#### by.epoch

> `readonly` **epoch**: [`SeriesEndpoint`](../interfaces/SeriesEndpoint.md)\<`T`\>

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
