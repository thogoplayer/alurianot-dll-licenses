"""python tools/test/test_lease_token.py — v1/v2 lease token round trips."""
import os
import sys

sys.path.insert(0, os.path.join(os.path.dirname(__file__), ".."))
import lease_token as t  # noqa: E402
from cryptography.hazmat.primitives.asymmetric import ec  # noqa: E402

key = ec.generate_private_key(ec.SECP256R1())
pub = key.public_key()
server = b"\x11" * 64

v1 = t.make_token(key, 3, 100, 200, server)
assert len(v1) == 153 and t.verify_token(pub, v1)["features"] == 0

v2 = t.make_token_v2(key, 3, 100, 200, server, 0x4002)
assert len(v2) == 161 and v2[4] == 2
info = t.verify_token(pub, v2)
assert info["features"] == 0x4002 and info["licenseId"] == 3

flipped = bytearray(v2)
flipped[89] ^= 1  # grant another feature
try:
    t.verify_token(pub, bytes(flipped))
    raise SystemExit("FAIL: tampered feature mask accepted")
except Exception as e:  # InvalidSignature
    if isinstance(e, SystemExit):
        raise
print("ok")
