[**brk-client**](../README.md)

***

[brk-client](../globals.md) / DateSeriesDataExtras

# Interface: DateSeriesDataExtras\<T\>

Defined in: [Developer/brk/modules/brk-client/index.js:1701](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L1701)

## Type Parameters

### T

`T`

## Properties

### dateEntries

> **dateEntries**: () => \[`Date`, `T`\][]

Defined in: [Developer/brk/modules/brk-client/index.js:1703](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L1703)

Get [date, value] pairs

#### Returns

\[`Date`, `T`\][]

***

### dates

> **dates**: () => `Date`[]

Defined in: [Developer/brk/modules/brk-client/index.js:1702](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L1702)

Get dates for each data point

#### Returns

`Date`[]

***

### toDateMap

> **toDateMap**: () => `Map`\<`Date`, `T`\>

Defined in: [Developer/brk/modules/brk-client/index.js:1704](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L1704)

Convert to Map<date, value>

#### Returns

`Map`\<`Date`, `T`\>
