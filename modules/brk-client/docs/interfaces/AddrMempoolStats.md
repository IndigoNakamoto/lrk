[**brk-client**](../README.md)

***

[brk-client](../globals.md) / AddrMempoolStats

# Interface: AddrMempoolStats

Defined in: [Developer/brk/modules/brk-client/index.js:50](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L50)

## Properties

### balanceDelta

> **balanceDelta**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:51](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L51)

Net pending (unconfirmed) balance change in satoshis; negative when pending spends exceed receipts

***

### fundedTxoCount

> **fundedTxoCount**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:52](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L52)

Number of unconfirmed transaction outputs funding this address

***

### fundedTxoSum

> **fundedTxoSum**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:53](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L53)

Total amount in satoshis being received in unconfirmed transactions

***

### spentTxoCount

> **spentTxoCount**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:54](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L54)

Number of unconfirmed transaction inputs spending from this address

***

### spentTxoSum

> **spentTxoSum**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:55](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L55)

Total amount in satoshis being spent in unconfirmed transactions

***

### txCount

> **txCount**: `number`

Defined in: [Developer/brk/modules/brk-client/index.js:56](https://github.com/bitcoinresearchkit/brk/blob/cc5b6da341b469859cb457c9c0c93a547eb6ee00/modules/brk-client/index.js#L56)

Number of unconfirmed transactions involving this address
