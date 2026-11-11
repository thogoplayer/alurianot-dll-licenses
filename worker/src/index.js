// AlurianOT ddraw.dll lease-token signer (Cloudflare Worker).
//
//   GET /tokens/<id>.tok  ->  one line of base64: a fresh 153-byte lease token
//                             for licence <id>, signed with the master key.
//   404 when the licence is unknown, revoked or past its "expires" date.
//
// The licence list is licenses.json in the public GitHub repo; the master key
// is the Worker secret MASTER_KEY (PKCS#8 PEM). Token layout and tags must
// match tools/lease_token.py, the DLL (license_guard.cpp) and the server
// (dll_license.cpp).

const LICENSES_URL =
	"https://raw.githubusercontent.com/thogoplayer/alurianot-dll-licenses/main/licenses.json";
// Last good copy of licenses.json, used when GitHub cannot be reached.
const LICENSES_FALLBACK_KEY = "https://alurianot-dll-licenses.internal/licenses.json";
const LEASE_TAG = new TextEncoder().encode("AlurianOT-DLL-Lease-v1");
const BODY_SIZE = 89;
const TOKEN_SIZE = 153;
// v2 (GET /v2/tokens/<id>.tok) appends a u64 feature bitmask to the body:
// 97-byte body + 64-byte signature = 161 bytes. v1 stays byte-identical.
const BODY_SIZE_V2 = 97;
const TOKEN_SIZE_V2 = 161;
// Append-only; index = bit. Must match manager/src-tauri/src/licenses.rs
// FEATURES and the DLL's Feature enum (dll/src/features.h).
const FEATURES = [
	"store", "market", "forge", "supply_stash", "task_hunt", "daily_reward", "hunting",
	"creature_icons", "guild_emblems", "skills", "side_columns", "store_inbox_purse",
	"login_screen", "discord_rpc", "mount", "auto_updater",
];

// null when the list has an unknown id.
function featureMask(list) {
	let mask = 0n;
	for (const id of list || []) {
		const bit = FEATURES.indexOf(id);
		if (bit < 0) return null;
		mask |= 1n << BigInt(bit);
	}
	return mask;
}

let cachedKey = null;

async function masterKey(env) {
	if (cachedKey) return cachedKey;
	const pem = env.MASTER_KEY || "";
	const b64 = pem.replace(/-----[^-]+-----/g, "").replace(/\s+/g, "");
	if (!b64) throw new Error("MASTER_KEY secret is not set");
	const der = Uint8Array.from(atob(b64), (c) => c.charCodeAt(0));
	cachedKey = await crypto.subtle.importKey(
		"pkcs8", der, { name: "ECDSA", namedCurve: "P-256" }, false, ["sign"]);
	return cachedKey;
}

async function loadLicenses(ctx) {
	const cache = caches.default;
	try {
		const res = await fetch(LICENSES_URL, { cf: { cacheTtl: 60, cacheEverything: true } });
		if (res.ok) {
			const text = await res.text();
			const data = JSON.parse(text);
			ctx.waitUntil(cache.put(LICENSES_FALLBACK_KEY, new Response(text, {
				headers: { "Cache-Control": "max-age=1209600", "Content-Type": "application/json" },
			})));
			return data;
		}
	} catch (_) {
		// fall through to the last good copy
	}
	const stale = await cache.match(LICENSES_FALLBACK_KEY);
	if (stale) return JSON.parse(await stale.text());
	throw new Error("licenses.json unavailable");
}

function hexToBytes(hex) {
	if (!/^[0-9a-fA-F]{128}$/.test(hex)) return null;
	const out = new Uint8Array(64);
	for (let i = 0; i < 64; i++) out[i] = parseInt(hex.substr(i * 2, 2), 16);
	return out;
}

function writeU64(view, offset, value) {
	view.setUint32(offset, value % 0x100000000, true);
	view.setUint32(offset + 4, Math.floor(value / 0x100000000), true);
}

// features === null -> v1 token; otherwise v2 carrying that BigInt mask.
async function makeToken(env, licenseId, issuedAt, expiresAt, serverKey, features = null) {
	const v2 = features !== null;
	const bodySize = v2 ? BODY_SIZE_V2 : BODY_SIZE;
	const token = new Uint8Array(v2 ? TOKEN_SIZE_V2 : TOKEN_SIZE);
	const view = new DataView(token.buffer);
	token.set([0x41, 0x4c, 0x4c, 0x54], 0); // "ALLT"
	token[4] = v2 ? 2 : 1;
	view.setUint32(5, licenseId, true);
	writeU64(view, 9, issuedAt);
	writeU64(view, 17, expiresAt);
	token.set(serverKey, 25);
	if (v2) view.setBigUint64(89, features, true);

	const message = new Uint8Array(LEASE_TAG.length + bodySize);
	message.set(LEASE_TAG, 0);
	message.set(token.subarray(0, bodySize), LEASE_TAG.length);
	// WebCrypto ECDSA returns raw r || s (IEEE P1363), the format the DLL wants.
	const sig = new Uint8Array(await crypto.subtle.sign(
		{ name: "ECDSA", hash: "SHA-256" }, await masterKey(env), message));
	if (sig.length !== 64) throw new Error("unexpected signature size");
	token.set(sig, bodySize);
	return token;
}

function text(body, status, extra = {}) {
	return new Response(body, {
		status,
		headers: { "Content-Type": "text/plain; charset=utf-8", "Cache-Control": "no-store", ...extra },
	});
}

export default {
	async fetch(request, env, ctx) {
		const url = new URL(request.url);
		if (request.method !== "GET" && request.method !== "HEAD") return text("method not allowed\n", 405);
		if (url.pathname === "/" || url.pathname === "/health") return text("ok\n", 200);

		const m = url.pathname.match(/^\/(v2\/)?tokens\/(\d{1,10})\.tok$/);
		if (!m) return text("not found\n", 404);
		const v2 = Boolean(m[1]);
		const id = Number(m[2]);

		let data;
		try {
			data = await loadLicenses(ctx);
		} catch (e) {
			return text("licence list unavailable\n", 503);
		}
		const lic = (data.licenses || []).find((l) => l.id === id);
		if (!lic || lic.revoked) return text("no such licence\n", 404);
		const serverKey = hexToBytes(String(lic.serverPublicKey || ""));
		if (!serverKey) return text("bad licence entry\n", 500);

		const now = Math.floor(Date.now() / 1000);
		const leaseHours = Math.min(Math.max(Number(data.leaseHours) || 48, 1), 720);
		let expiresAt = now + leaseHours * 3600;
		if (lic.expires) {
			const end = Date.parse(lic.expires + "T00:00:00Z");
			if (Number.isNaN(end)) return text("bad licence entry\n", 500);
			const hardEnd = end / 1000 + 86400; // "expires" day is inclusive
			if (hardEnd <= now) return text("licence expired\n", 404);
			expiresAt = Math.min(expiresAt, hardEnd);
		}

		try {
			let features = null;
			if (v2) {
				features = featureMask(lic.features);
				if (features === null) return text("bad licence entry\n", 500);
			}
			const token = await makeToken(env, id, now, expiresAt, serverKey, features);
			let bin = "";
			for (const b of token) bin += String.fromCharCode(b);
			return text(btoa(bin) + "\n", 200);
		} catch (e) {
			return text("signing failed\n", 500);
		}
	},
};
