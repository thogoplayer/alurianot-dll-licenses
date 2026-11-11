// node --test worker/test — runs the Worker's fetch() with a throwaway master
// key and a stubbed licenses.json.
import { test } from "node:test";
import assert from "node:assert/strict";

const store = new Map();
globalThis.caches = {
  default: {
    async put(k, r) { store.set(k, await r.text()); },
    async match(k) { return store.has(k) ? new Response(store.get(k)) : undefined; },
  },
};

const SERVER_KEY = "bd5af042deeb3f5028e149b3462d4d9b2393d5e5ff7c809cffe9c6fa3f6d19a87c6fce22f899c65192467c67ed99cc9e4dc3a93350611211153aa6d7bf799406";
const LICENSES = {
  leaseHours: 48,
  licenses: [
    { id: 1, name: "a", serverPublicKey: SERVER_KEY, revoked: false, expires: null, note: "", features: ["store", "mount", "auto_updater"] },
    { id: 2, name: "b", serverPublicKey: SERVER_KEY, revoked: false, expires: null, note: "" },
    { id: 3, name: "c", serverPublicKey: SERVER_KEY, revoked: false, expires: null, note: "", features: ["guild"] },
  ],
  retiredIds: [],
};
globalThis.fetch = async () => new Response(JSON.stringify(LICENSES));

const pair = await crypto.subtle.generateKey({ name: "ECDSA", namedCurve: "P-256" }, true, ["sign", "verify"]);
const pkcs8 = Buffer.from(await crypto.subtle.exportKey("pkcs8", pair.privateKey)).toString("base64");
const env = { MASTER_KEY: `-----BEGIN PRIVATE KEY-----\n${pkcs8}\n-----END PRIVATE KEY-----\n` };
const ctx = { waitUntil: (p) => p };
const worker = (await import("../src/index.js")).default;
const TAG = new TextEncoder().encode("AlurianOT-DLL-Lease-v1");

async function get(path) {
  const res = await worker.fetch(new Request("https://w.test" + path), env, ctx);
  return { status: res.status, bytes: Buffer.from((await res.text()).trim(), "base64") };
}

async function verifies(token, bodySize) {
  const msg = Buffer.concat([TAG, token.subarray(0, bodySize)]);
  return crypto.subtle.verify({ name: "ECDSA", hash: "SHA-256" }, pair.publicKey, token.subarray(bodySize), msg);
}

test("v1 path unchanged: 153 bytes, version 1", async () => {
  const { status, bytes } = await get("/tokens/1.tok");
  assert.equal(status, 200);
  assert.equal(bytes.length, 153);
  assert.equal(bytes[4], 1);
  assert.ok(await verifies(bytes, 89));
});

test("v2 path: 161 bytes, version 2, feature mask, signature", async () => {
  const { status, bytes } = await get("/v2/tokens/1.tok");
  assert.equal(status, 200);
  assert.equal(bytes.length, 161);
  assert.equal(bytes[4], 2);
  assert.equal(bytes.readBigUInt64LE(89), BigInt(1 | (1 << 14) | (1 << 15)));
  assert.ok(await verifies(bytes, 97));
});

test("v2 with no features field: mask 0", async () => {
  const { bytes } = await get("/v2/tokens/2.tok");
  assert.equal(bytes.readBigUInt64LE(89), 0n);
});

test("unknown id is 404 on both paths", async () => {
  assert.equal((await get("/tokens/9.tok")).status, 404);
  assert.equal((await get("/v2/tokens/9.tok")).status, 404);
});

test("v2 guild feature is bit 16", async () => {
  const { status, bytes } = await get("/v2/tokens/3.tok");
  assert.equal(status, 200);
  assert.equal(bytes.readBigUInt64LE(89), 1n << 16n);
});
