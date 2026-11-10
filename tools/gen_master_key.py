#!/usr/bin/env python3
"""Generate the AlurianOT DLL licence MASTER key (do this once).

  python tools/gen_master_key.py --out C:\\tmp\\licenses

Writes master.key (PKCS#8 PEM, SECRET: goes into the GitHub secret
DLL_LICENSE_MASTER_KEY and an offline backup, never into git) and prints the
TIBIA_LICENSE_MASTER_PUBLIC_KEY block for the DLL's config.h. Refuses to
overwrite an existing master.key: a new master key invalidates every DLL.
"""
import argparse
import os
import sys

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lease_token  # noqa: E402
from cryptography.hazmat.primitives import serialization  # noqa: E402
from cryptography.hazmat.primitives.asymmetric import ec  # noqa: E402


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--out", required=True)
    args = ap.parse_args()
    os.makedirs(args.out, exist_ok=True)
    path = os.path.join(args.out, "master.key")
    if os.path.exists(path):
        raise SystemExit("%s already exists; refusing to replace the master key" % path)

    key = ec.generate_private_key(ec.SECP256R1())
    pem = key.private_bytes(serialization.Encoding.PEM, serialization.PrivateFormat.PKCS8,
                            serialization.NoEncryption())
    fd = os.open(path, os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o600)
    with os.fdopen(fd, "wb") as f:
        f.write(pem)

    raw = lease_token.public_key_raw(key.public_key())
    tokens = ["0x%02X" % b for b in raw]
    lines = [", ".join(tokens[i:i + 12]) for i in range(0, 64, 12)]
    print("master key : %s  (SECRET - GitHub secret + offline backup)" % path)
    print("public hex : %s" % raw.hex())
    print("#define TIBIA_LICENSE_MASTER_PUBLIC_KEY \\")
    print(", \\\n".join("    " + l for l in lines))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
