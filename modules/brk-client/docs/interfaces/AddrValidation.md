[**brk-client**](../README.md)

***

[brk-client](../globals.md) / AddrValidation

# Interface: AddrValidation

Defined in: [Developer/brk/modules/brk-client/index.js:77](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L77)

## Properties

### address?

> `optional` **address?**: `string` \| `null`

Defined in: [Developer/brk/modules/brk-client/index.js:79](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L79)

The validated address

***

### error?

> `optional` **error?**: `string` \| `null`

Defined in: [Developer/brk/modules/brk-client/index.js:86](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L86)

Error message for invalid addresses

***

### errorLocations?

> `optional` **errorLocations?**: `number`[] \| `null`

Defined in: [Developer/brk/modules/brk-client/index.js:85](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L85)

Error locations (empty array for most errors)

***

### isscript?

> `optional` **isscript?**: `boolean` \| `null`

Defined in: [Developer/brk/modules/brk-client/index.js:81](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L81)

Whether this is a script address (P2SH)

***

### isvalid

> **isvalid**: `boolean`

Defined in: [Developer/brk/modules/brk-client/index.js:78](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L78)

Whether the address is valid

***

### iswitness?

> `optional` **iswitness?**: `boolean` \| `null`

Defined in: [Developer/brk/modules/brk-client/index.js:82](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L82)

Whether this is a witness address

***

### scriptPubKey?

> `optional` **scriptPubKey?**: `string` \| `null`

Defined in: [Developer/brk/modules/brk-client/index.js:80](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L80)

The scriptPubKey in hex

***

### witnessProgram?

> `optional` **witnessProgram?**: `string` \| `null`

Defined in: [Developer/brk/modules/brk-client/index.js:84](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L84)

Witness program in hex

***

### witnessVersion?

> `optional` **witnessVersion?**: `number` \| `null`

Defined in: [Developer/brk/modules/brk-client/index.js:83](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L83)

Witness version (0 for P2WPKH/P2WSH, 1 for P2TR)
