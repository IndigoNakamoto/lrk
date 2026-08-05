[**brk-client**](../README.md)

***

[brk-client](../globals.md) / MempoolBlock

# Interface: MempoolBlock

Defined in: [Developer/brk/modules/brk-client/index.js:726](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L726)

## Properties

### blockSize

> **blockSize**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:727](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L727)

Total serialized block size in bytes (witness + non-witness).

***

### blockVSize

> **blockVSize**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:728](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L728)

Total block virtual size in vbytes

***

### feeRange

> **feeRange**: `number`[]

Defined in: [Developer/brk/modules/brk-client/index.js:732](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L732)

Fee rate range: [min, 10%, 25%, 50%, 75%, 90%, max]

***

### medianFee

> **medianFee**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:731](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L731)

Median fee rate in sat/vB

***

### nTx

> **nTx**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:729](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L729)

Number of transactions in the projected block

***

### totalFees

> **totalFees**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:730](https://github.com/bitcoinresearchkit/brk/blob/76e08a33afe7d878b09f4359c21ffd6183727e32/modules/brk-client/index.js#L730)

Total fees in satoshis
