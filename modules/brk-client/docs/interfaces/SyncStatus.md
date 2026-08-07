[**brk-client**](../README.md)

***

[brk-client](../globals.md) / SyncStatus

# Interface: SyncStatus

Defined in: [Developer/brk/modules/brk-client/index.js:1209](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1209)

## Properties

### blocksBehind

> **blocksBehind**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:1213](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1213)

Number of blocks behind the tip

***

### computedHeight

> **computedHeight**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:1211](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1211)

Height of the last computed block (series)

***

### indexedHeight

> **indexedHeight**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:1210](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1210)

Height of the last indexed block

***

### lastIndexedAt

> **lastIndexedAt**: `string`

Defined in: [Developer/brk/modules/brk-client/index.js:1214](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1214)

Human-readable timestamp of the last indexed block (ISO 8601)

***

### lastIndexedAtUnix

> **lastIndexedAtUnix**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:1215](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1215)

Unix timestamp of the last indexed block

***

### tipHeight

> **tipHeight**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:1212](https://github.com/bitcoinresearchkit/brk/blob/51e058e08ee90a54012facf825ef4a1944101a66/modules/brk-client/index.js#L1212)

Height of the chain tip (from Bitcoin node)
