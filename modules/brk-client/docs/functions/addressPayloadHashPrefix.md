[**brk-client**](../README.md)

***

[brk-client](../globals.md) / addressPayloadHashPrefix

# Function: addressPayloadHashPrefix()

> **addressPayloadHashPrefix**(`payload`, `nibbles`): `string`

Defined in: [Developer/brk/modules/brk-client/index.js:2330](https://github.com/bitcoinresearchkit/brk/blob/75e961b6adced8a53e31527679ee5e4b5be69b12/modules/brk-client/index.js#L2330)

Compute the RapidHash v3 hash-prefix used by `/api/address/hash-prefix/{addr_type}/{prefix}`.

## Parameters

### payload

`number`[] \| `ArrayBuffer` \| `Uint8Array`\<`ArrayBufferLike`\> \| `ArrayBufferView`\<`ArrayBufferLike`\>

Raw address payload bytes

### nibbles

`number`

Prefix length from 1 to 16 hex nibbles

## Returns

`string`
