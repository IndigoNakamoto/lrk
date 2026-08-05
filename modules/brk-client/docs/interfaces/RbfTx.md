[**brk-client**](../README.md)

***

[brk-client](../globals.md) / RbfTx

# Interface: RbfTx

Defined in: [Developer/brk/modules/brk-client/index.js:1020](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L1020)

## Properties

### fee

> **fee**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:1022](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L1022)

***

### fullRbf?

> `optional` **fullRbf?**: `boolean` \| `null`

Defined in: [Developer/brk/modules/brk-client/index.js:1028](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L1028)

Only populated on the root `tx` of an RBF response. `true` iff
this tx displaced at least one non-signaling predecessor.

***

### rate

> **rate**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:1025](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L1025)

***

### rbf

> **rbf**: `boolean`

Defined in: [Developer/brk/modules/brk-client/index.js:1027](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L1027)

BIP-125 signaling: at least one input has sequence < 0xffffffff-1.

***

### time

> **time**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:1026](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L1026)

***

### txid

> **txid**: `string`

Defined in: [Developer/brk/modules/brk-client/index.js:1021](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L1021)

***

### value

> **value**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:1024](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L1024)

Sum of output amounts.

***

### vsize

> **vsize**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:1023](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L1023)
