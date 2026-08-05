[**brk-client**](../README.md)

***

[brk-client](../globals.md) / DataRangeFormat

# Interface: DataRangeFormat

Defined in: [Developer/brk/modules/brk-client/index.js:479](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L479)

## Properties

### end?

> `optional` **end?**: `number` \| `null`

Defined in: [Developer/brk/modules/brk-client/index.js:481](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L481)

Exclusive end: integer index, date (YYYY-MM-DD), or timestamp (ISO 8601). Negative integers count from end. Aliases: `to`, `t`, `e`

***

### format?

> `optional` **format?**: [`Format`](../type-aliases/Format.md)

Defined in: [Developer/brk/modules/brk-client/index.js:483](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L483)

Format of the output

***

### limit?

> `optional` **limit?**: `number` \| `null`

Defined in: [Developer/brk/modules/brk-client/index.js:482](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L482)

Maximum number of values to return (ignored if `end` is set). Aliases: `count`, `c`, `l`

***

### start?

> `optional` **start?**: `number` \| `null`

Defined in: [Developer/brk/modules/brk-client/index.js:480](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L480)

Inclusive start: integer index, date (YYYY-MM-DD), or timestamp (ISO 8601). Negative integers count from end. Aliases: `from`, `f`, `s`
