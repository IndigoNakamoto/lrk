#!/usr/bin/env python3
"""Dump raw Litecoin blocks plus RPC reference values for the MWEB decoder tests.

Usage: mweb_fixtures.py OUT_DIR HEIGHT [HEIGHT ...]
       mweb_fixtures.py OUT_DIR --sample   (activation, every 10,000th block, last 100)

For each height writes OUT_DIR/<height>.hex (getblock verbosity 0) and
OUT_DIR/<height>.json with the reference values the Rust tests assert against.
Requires a synced litecoind with -txindex; set LITECOIN_CLI to override the binary.
"""

import json
import os
import subprocess
import sys

CLI = os.environ.get("LITECOIN_CLI", "litecoin-cli").split()
ACTIVATION = 2_265_984


def rpc(*args):
    out = subprocess.run(CLI + [str(a) for a in args], check=True, capture_output=True, text=True).stdout
    try:
        return json.loads(out)
    except json.JSONDecodeError:
        return out.strip()


def sats(btc):
    return round(btc * 100_000_000)


def hogex_fee(block):
    hog = block["tx"][-1]
    in_block = {tx["txid"]: tx for tx in block["tx"]}
    total_in = 0
    for vin in hog["vin"]:
        tx = in_block.get(vin["txid"]) or rpc("getrawtransaction", vin["txid"], "true")
        total_in += sats(tx["vout"][vin["vout"]]["value"])
    return total_in - sum(sats(o["value"]) for o in hog["vout"])


def dump(out_dir, height):
    block_hash = rpc("getblockhash", height)
    header = rpc("getblockheader", block_hash)
    block = rpc("getblock", block_hash, 2)
    stats = rpc("getblockstats", height, '["subsidy","totalfee"]')
    mweb = height >= ACTIVATION
    ref = {
        "height": height,
        "hash": block_hash,
        "mweb_active": mweb,
        "tx_count": len(block["tx"]),
        "subsidy": stats["subsidy"],
        "total_fee": stats["totalfee"],
        "coinbase_value": sum(sats(o["value"]) for o in block["tx"][0]["vout"]),
    }
    if mweb:
        mh = header["mweb_header"]
        ref.update(
            mweb_amount=header["mweb_amount"],
            prev_mweb_amount=header["prev_mweb_amount"],
            num_kernels=mh["num_kernels"],
            num_txos=mh["num_txos"],
            hogex_fee=hogex_fee(block),
        )
        if height > ACTIVATION:
            prev = rpc("getblock", header["previousblockhash"], 2)
            ref["prev_hogex_txid"] = prev["tx"][-1]["txid"]
    with open(os.path.join(out_dir, f"{height}.hex"), "w") as f:
        f.write(rpc("getblock", block_hash, 0))
    with open(os.path.join(out_dir, f"{height}.json"), "w") as f:
        json.dump(ref, f, indent=1)


def main():
    out_dir, *rest = sys.argv[1:]
    os.makedirs(out_dir, exist_ok=True)
    if rest == ["--sample"]:
        tip = rpc("getblockcount")
        heights = sorted({ACTIVATION - 1, ACTIVATION, *range(ACTIVATION + 10_000, tip, 10_000), *range(tip - 99, tip + 1)})
    else:
        heights = [int(h) for h in rest]
    for i, h in enumerate(heights, 1):
        dump(out_dir, h)
        print(f"[{i}/{len(heights)}] {h}", file=sys.stderr)


if __name__ == "__main__":
    main()
