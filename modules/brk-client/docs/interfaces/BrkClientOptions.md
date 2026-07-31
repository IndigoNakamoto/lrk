[**brk-client**](../README.md)

***

[brk-client](../globals.md) / BrkClientOptions

# Interface: BrkClientOptions

Defined in: [Developer/brk/modules/brk-client/index.js:1468](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L1468)

## Properties

### baseUrl

> **baseUrl**: `string`

Defined in: [Developer/brk/modules/brk-client/index.js:1469](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L1469)

Base URL for the API

***

### browserCache?

> `optional` **browserCache?**: `string` \| `boolean`

Defined in: [Developer/brk/modules/brk-client/index.js:1471](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L1471)

Enable browser Cache API with default name (true), custom name (string), or disable (false). No effect in Node.js. Default: true

***

### memCache?

> `optional` **memCache?**: `number` \| `boolean`

Defined in: [Developer/brk/modules/brk-client/index.js:1472](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L1472)

In-memory parsed-response cache size (LRU). true/undefined → 1000, false/0 → disabled. Lets 304 responses skip the JSON parse entirely. Default: 1000

***

### timeout?

> `optional` **timeout?**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:1470](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L1470)

Request timeout in milliseconds
