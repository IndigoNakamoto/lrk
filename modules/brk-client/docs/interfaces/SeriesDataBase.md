[**brk-client**](../README.md)

***

[brk-client](../globals.md) / SeriesDataBase

# Interface: SeriesDataBase\<T\>

Defined in: [Developer/brk/modules/brk-client/index.js:1682](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1682)

## Type Parameters

### T

`T`

## Properties

### data

> **data**: `T`[]

Defined in: [Developer/brk/modules/brk-client/index.js:1689](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1689)

The series data

***

### end

> **end**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:1687](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1687)

End index (exclusive)

***

### entries

> **entries**: () => \[`number`, `T`\][]

Defined in: [Developer/brk/modules/brk-client/index.js:1693](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1693)

Get [index, value] pairs

#### Returns

\[`number`, `T`\][]

***

### index

> **index**: [`Index`](../type-aliases/Index.md)

Defined in: [Developer/brk/modules/brk-client/index.js:1684](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1684)

The index type used for this query

***

### indexes

> **indexes**: () => `number`[]

Defined in: [Developer/brk/modules/brk-client/index.js:1691](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1691)

Get index numbers

#### Returns

`number`[]

***

### isDateBased

> **isDateBased**: `boolean`

Defined in: [Developer/brk/modules/brk-client/index.js:1690](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1690)

Whether this series uses a date-based index

***

### keys

> **keys**: () => `number`[]

Defined in: [Developer/brk/modules/brk-client/index.js:1692](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1692)

Get keys as index numbers (alias for indexes)

#### Returns

`number`[]

***

### stamp

> **stamp**: `string`

Defined in: [Developer/brk/modules/brk-client/index.js:1688](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1688)

ISO 8601 timestamp of when the response was generated

***

### start

> **start**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:1686](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1686)

Start index (inclusive)

***

### toMap

> **toMap**: () => `Map`\<`number`, `T`\>

Defined in: [Developer/brk/modules/brk-client/index.js:1694](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1694)

Convert to Map<index, value>

#### Returns

`Map`\<`number`, `T`\>

***

### type

> **type**: `string`

Defined in: [Developer/brk/modules/brk-client/index.js:1685](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1685)

Value type (e.g. "f32", "u64", "Sats")

***

### version

> **version**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:1683](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1683)

Version of the series data
