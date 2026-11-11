"""AlurianOT ddraw.dll lease tokens: format, signing and verification.

A lease token says "game server <serverPublicKey> may run DLL licence <id>
until <expiresAt>". It is signed with the AlurianOT master key; the DLL only
holds the master public key. Tokens are short-lived (licenses.json leaseHours)
and re-signed by the GitHub Action, so a revoked licence simply stops getting
new tokens and dies when its last one expires.

Binary layout (little-endian), 153 bytes:
    0   4  magic "ALLT"
    4   1  version (1)
    5   4  licenseId (u32)
    9   8  issuedAt  (u64, unix seconds UTC)
   17   8  expiresAt (u64, unix seconds UTC)
   25  64  game server public key, P-256 X || Y
   89  64  master signature, raw r || s, ECDSA P-256 / SHA-256 over
           LEASE_DOMAIN_TAG || bytes[0:89]

Published as one line of standard base64 (gists are text).
"""
import base64
import struct

from cryptography.hazmat.primitives import hashes, serialization
from cryptography.hazmat.primitives.asymmetric import ec
from cryptography.hazmat.primitives.asymmetric.utils import (
    decode_dss_signature,
    encode_dss_signature,
)

# Must match TIBIA_LICENSE_LEASE_TAG in the DLL (license_guard.cpp).
LEASE_DOMAIN_TAG = b"AlurianOT-DLL-Lease-v1"
MAGIC = b"ALLT"
VERSION = 1
BODY_SIZE = 89
TOKEN_SIZE = 153
# v2 (Worker path /v2/tokens/<id>.tok): + u64 feature bitmask at 89..97,
# signature over LEASE_DOMAIN_TAG || bytes[0:97]; 161 bytes.
VERSION_V2 = 2
BODY_SIZE_V2 = 97
TOKEN_SIZE_V2 = 161


def public_key_raw(public_key) -> bytes:
    n = public_key.public_numbers()
    return n.x.to_bytes(32, "big") + n.y.to_bytes(32, "big")


def public_key_from_raw(raw: bytes):
    if len(raw) != 64:
        raise ValueError("public key must be 64 bytes (X||Y)")
    return ec.EllipticCurvePublicKey.from_encoded_point(ec.SECP256R1(), b"\x04" + raw)


def load_private_key(pem: bytes):
    key = serialization.load_pem_private_key(pem, password=None)
    if not isinstance(key, ec.EllipticCurvePrivateKey) or key.curve.name != "secp256r1":
        raise ValueError("master key must be an EC P-256 private key")
    return key


def make_token(master_key, license_id: int, issued_at: int, expires_at: int,
               server_public_key: bytes) -> bytes:
    if len(server_public_key) != 64:
        raise ValueError("server public key must be 64 bytes")
    body = MAGIC + struct.pack("<BIQQ", VERSION, license_id, issued_at, expires_at) + server_public_key
    assert len(body) == BODY_SIZE
    der = master_key.sign(LEASE_DOMAIN_TAG + body, ec.ECDSA(hashes.SHA256()))
    r, s = decode_dss_signature(der)
    token = body + r.to_bytes(32, "big") + s.to_bytes(32, "big")
    assert len(token) == TOKEN_SIZE
    return token


def make_token_v2(master_key, license_id: int, issued_at: int, expires_at: int,
                  server_public_key: bytes, features: int) -> bytes:
    if len(server_public_key) != 64:
        raise ValueError("server public key must be 64 bytes")
    body = (MAGIC + struct.pack("<BIQQ", VERSION_V2, license_id, issued_at, expires_at)
            + server_public_key + struct.pack("<Q", features))
    assert len(body) == BODY_SIZE_V2
    der = master_key.sign(LEASE_DOMAIN_TAG + body, ec.ECDSA(hashes.SHA256()))
    r, s = decode_dss_signature(der)
    return body + r.to_bytes(32, "big") + s.to_bytes(32, "big")


def parse_token(token: bytes) -> dict:
    if len(token) < 5 or token[:4] != MAGIC:
        raise ValueError("not a lease token")
    version = token[4]
    if version == VERSION and len(token) == TOKEN_SIZE:
        body_size, features = BODY_SIZE, 0
    elif version == VERSION_V2 and len(token) == TOKEN_SIZE_V2:
        body_size = BODY_SIZE_V2
        (features,) = struct.unpack_from("<Q", token, 89)
    else:
        raise ValueError("unsupported token version %d / size %d" % (version, len(token)))
    _, license_id, issued_at, expires_at = struct.unpack_from("<BIQQ", token, 4)
    return {
        "licenseId": license_id,
        "issuedAt": issued_at,
        "expiresAt": expires_at,
        "serverPublicKey": token[25:89].hex(),
        "features": features,
        "signature": token[body_size:],
        "body": token[:body_size],
    }


def verify_token(master_public_key, token: bytes) -> dict:
    info = parse_token(token)
    sig = info["signature"]
    der = encode_dss_signature(int.from_bytes(sig[:32], "big"), int.from_bytes(sig[32:], "big"))
    master_public_key.verify(der, LEASE_DOMAIN_TAG + info["body"], ec.ECDSA(hashes.SHA256()))
    return info


def encode_text(token: bytes) -> str:
    return base64.b64encode(token).decode("ascii") + "\n"


def decode_text(text: str) -> bytes:
    return base64.b64decode(text.strip(), validate=True)
