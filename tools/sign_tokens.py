#!/usr/bin/env python3
"""Sign fresh lease tokens for every active licence in licenses.json.

The master private key comes from the DLL_LICENSE_MASTER_KEY environment
variable (PEM text, the GitHub secret) or from --master-key FILE.

  python tools/sign_tokens.py --out public            # write <id>.tok files
  python tools/sign_tokens.py --gist GIST_ID          # publish to the gist
      (needs GIST_TOKEN: a fine-grained token with only Gists read/write)

A licence is active when revoked is false and its optional "expires"
(YYYY-MM-DD, UTC, inclusive) has not passed. Revoked/expired licences get no
token, and their file is deleted from the gist, so their last token dies on
its own after leaseHours.
"""
import argparse
import datetime as dt
import json
import os
import re
import sys
import time
import urllib.error
import urllib.request

sys.path.insert(0, os.path.dirname(os.path.abspath(__file__)))
import lease_token  # noqa: E402

TOKEN_NAME = re.compile(r"^\d+\.tok$")


def load_licenses(path):
    with open(path, encoding="utf-8") as f:
        data = json.load(f)
    lease_hours = int(data.get("leaseHours", 48))
    if not 1 <= lease_hours <= 24 * 30:
        raise SystemExit("leaseHours must be between 1 and 720")
    seen = set()
    out = []
    for entry in data.get("licenses", []):
        lid = entry.get("id")
        if not isinstance(lid, int) or not 1 <= lid <= 0xFFFFFFFF:
            raise SystemExit("invalid licence id: %r" % (lid,))
        if lid in seen:
            raise SystemExit("duplicate licence id %d" % lid)
        seen.add(lid)
        pub_hex = str(entry.get("serverPublicKey", "")).strip().lower()
        if not re.fullmatch(r"[0-9a-f]{128}", pub_hex):
            raise SystemExit("licence %d: serverPublicKey must be 128 hex digits" % lid)
        lease_token.public_key_from_raw(bytes.fromhex(pub_hex))  # on the curve?
        expires = entry.get("expires")
        hard_end = None
        if expires:
            day = dt.datetime.strptime(expires, "%Y-%m-%d").replace(tzinfo=dt.timezone.utc)
            hard_end = int((day + dt.timedelta(days=1)).timestamp())
        out.append({
            "id": lid,
            "name": entry.get("name", ""),
            "pub": bytes.fromhex(pub_hex),
            "revoked": bool(entry.get("revoked", False)),
            "hardEnd": hard_end,
        })
    return lease_hours, out


def sign_all(master_key, lease_hours, licenses, now):
    tokens = {}
    for lic in licenses:
        if lic["revoked"]:
            print("licence %d (%s): revoked, no token" % (lic["id"], lic["name"]))
            continue
        expires_at = now + lease_hours * 3600
        if lic["hardEnd"] is not None:
            if lic["hardEnd"] <= now:
                print("licence %d (%s): expired, no token" % (lic["id"], lic["name"]))
                continue
            expires_at = min(expires_at, lic["hardEnd"])
        token = lease_token.make_token(master_key, lic["id"], now, expires_at, lic["pub"])
        tokens["%d.tok" % lic["id"]] = lease_token.encode_text(token)
        print("licence %d (%s): token valid until %s UTC" % (
            lic["id"], lic["name"],
            dt.datetime.fromtimestamp(expires_at, dt.timezone.utc).strftime("%Y-%m-%d %H:%M")))
    return tokens


def gist_request(method, gist_id, token, body=None):
    req = urllib.request.Request(
        "https://api.github.com/gists/" + gist_id,
        method=method,
        data=None if body is None else json.dumps(body).encode("utf-8"),
        headers={
            "Accept": "application/vnd.github+json",
            "Authorization": "Bearer " + token,
            "X-GitHub-Api-Version": "2022-11-28",
            "User-Agent": "alurianot-dll-licenses",
        },
    )
    with urllib.request.urlopen(req, timeout=30) as resp:
        return json.loads(resp.read().decode("utf-8"))


def publish_gist(gist_id, token, tokens):
    current = gist_request("GET", gist_id, token)
    files = {name: {"content": text} for name, text in tokens.items()}
    for name in current.get("files", {}):
        if TOKEN_NAME.match(name) and name not in tokens:
            files[name] = None  # revoked/expired: delete the file
            print("removing %s from the gist" % name)
    # A gist must keep at least one file.
    files["README.md"] = {"content": "AlurianOT ddraw.dll lease tokens. Generated; do not edit.\n"}
    gist_request("PATCH", gist_id, token, {"files": files})
    print("gist %s updated (%d tokens)" % (gist_id, len(tokens)))


def main():
    ap = argparse.ArgumentParser()
    ap.add_argument("--licenses", default=os.path.join(os.path.dirname(__file__), "..", "licenses.json"))
    ap.add_argument("--master-key", help="PEM file (default: $DLL_LICENSE_MASTER_KEY)")
    ap.add_argument("--out", help="directory to write <id>.tok files into")
    ap.add_argument("--gist", help="gist id to publish into (needs $GIST_TOKEN)")
    ap.add_argument("--now", type=int, help="override the clock (tests)")
    args = ap.parse_args()

    if args.master_key:
        with open(args.master_key, "rb") as f:
            pem = f.read()
    else:
        pem = os.environ.get("DLL_LICENSE_MASTER_KEY", "").encode("utf-8")
    if not pem.strip():
        raise SystemExit("no master key (set DLL_LICENSE_MASTER_KEY or --master-key)")
    master_key = lease_token.load_private_key(pem)

    lease_hours, licenses = load_licenses(args.licenses)
    now = args.now if args.now is not None else int(time.time())
    tokens = sign_all(master_key, lease_hours, licenses, now)

    # Self-check before anything leaves this machine.
    master_pub = master_key.public_key()
    for text in tokens.values():
        lease_token.verify_token(master_pub, lease_token.decode_text(text))

    if args.out:
        os.makedirs(args.out, exist_ok=True)
        for name in os.listdir(args.out):
            if TOKEN_NAME.match(name) and name not in tokens:
                os.remove(os.path.join(args.out, name))
        for name, text in tokens.items():
            with open(os.path.join(args.out, name), "w", encoding="ascii", newline="\n") as f:
                f.write(text)
        print("wrote %d tokens to %s" % (len(tokens), args.out))
    if args.gist:
        gist_token = os.environ.get("GIST_TOKEN", "")
        if not gist_token:
            raise SystemExit("GIST_TOKEN is not set")
        try:
            publish_gist(args.gist, gist_token, tokens)
        except urllib.error.HTTPError as e:
            raise SystemExit("gist update failed: HTTP %d %s" % (e.code, e.read()[:300]))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
