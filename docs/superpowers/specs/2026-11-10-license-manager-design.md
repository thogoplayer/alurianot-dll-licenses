# AlurianOT License Manager (Tauri) — design

## Intent

The owner manages ddraw.dll licences today by running `gen_license.py`,
hand-editing `licenses.json`, committing and pushing, and copying config lines
into chat. He wants a desktop app that does all of it with clicks ("gerar a
key, tudo facilitado"). Single user, his own Windows PC, Portuguese UI.

## How the licence system works (context for the builder)

- `licenses.json` in this repo (public, on GitHub, branch `main`) is the list:
  `{ "leaseHours": 48, "licenses": [ { "id", "name", "serverPublicKey"
  (128 hex, P-256 X||Y), "revoked", "expires" (null or "YYYY-MM-DD"), "note" } ] }`.
- The Cloudflare Worker (`worker/src/index.js`) reads that file from GitHub raw
  (cache ~1 min) and signs `GET /tokens/<id>.tok` on demand with the master
  key. Revoked, unknown or expired licences get 404.
  Base URL: `https://alurianot-dll-licenses.thiago-freitasferraz.workers.dev`.
- Each game server holds its own private key (PKCS#8 PEM, EC P-256) and
  config.lua lines:
  ```lua
  dllLicenseKeyFile = "/home/<user>/.alurianot/dll_license.key"
  dllLicenseId = <id>
  dllLicenseTokenUrl = "<worker>/tokens/<id>.tok"
  ```
- Server keys live in `C:\tmp\licenses\` as `<name>-<id>.key` (+ `.json`
  with `publicKey`, written by `AlurianOt-ClientOld/tools/license/gen_license.py`).
  Existing: `tharcio-2.key`. Licence 1 (AlurianOT) has its key only in WSL
  (`~/.alurianot/dll_license.key`), not in `C:\tmp\licenses\`.

## Goals (what the owner asked for)

1. **List** licences: id, name, status (ativa / revogada / vencida), expiry
   date, note.
2. **New licence**: type a name (+ optional expiry date, note). The app picks
   the next free id, generates the P-256 key pair, saves
   `<name>-<id>.key` (+ `.json`) in the keys folder, adds the entry to
   `licenses.json`, commits and pushes.
3. **Revoke / reactivate**: toggle `revoked`, commit, push.
4. **Delete**: remove the entry (confirmation dialog), commit, push. The key
   file is kept (moved into a `deleted/` subfolder of the keys folder) —
   builder's call: safer than destroying it.
5. **Show config**: the three config.lua lines for a licence, with a copy
   button; button to open the keys folder in Explorer with the key selected.
6. **Status**: per licence, ask the Worker for its token and show whether it
   is served and until when (decode bytes 17..25 = expiresAt u64 LE of the
   base64 token; 404 = not served). On demand (button / on refresh), not
   polling.

## Non-goals

- Building the buyer's DLL or zip package (owner said keys/config are enough).
- Changing the master key, the Worker, or the auto updater.
- Multi-user, login, remote hosting.

## Anti-goals (failure even if it "works")

- Never show, copy, or transmit the master key; the app never needs it.
- Never upload a server private key anywhere (only the public key goes into
  licenses.json).
- Never leave `licenses.json` invalid or half-written (validate before
  writing; write atomically).
- Never push silently on top of a remote change: pull (rebase) before
  committing; on conflict/failure show the git error and leave the working
  copy as it was.

## Constraints / decisions

- **Tauri 2**, Rust backend, plain HTML/CSS/TypeScript frontend with Vite (my
  call: no framework needed for one screen + a dialog). Windows x64 only.
- Key generation in Rust (`p256` crate with `pkcs8` PEM export, `rand_core`
  OsRng). Output format identical to `gen_license.py` (PKCS#8 PEM private key;
  public key = 64-byte X||Y hex).
- Git: shell out to the installed `git` in the repo folder (the owner's
  existing credentials work; no token in the app). Sequence per change:
  `git pull --rebase` → edit file → `git add licenses.json` →
  `git commit -m "<action>: <name> (#id)"` → `git push`. Hide console windows
  (CREATE_NO_WINDOW).
- Settings (stored in the app's config dir, editable in a small Settings
  panel): repo folder (default
  `%USERPROFILE%\Documents\GitHub\alurianot-dll-licenses`), keys folder
  (default `C:\tmp\licenses`), Worker base URL (default above).
- HTTP for status: `reqwest` (blocking or async) from Rust, 5 s timeout.
- Preserve `licenses.json` formatting: 2-space indent, `leaseHours` kept,
  field order id, name, serverPublicKey, revoked, expires, note.
- Name rules: a-z0-9-_ (lowercased), 1..32 chars, unique among licences.
- Look: dark theme fitting a Tibia OT admin tool, compact table, Portuguese
  labels. Builder's call on details.

## Calls made after the builder check (mine, not the owner's)

- **Git robustness.** Pull with `git pull --rebase --autostash`; stage and
  commit only the list: `git commit -m ... -- licenses.json`, so other work in
  the repo (e.g. `manager/` itself) is never swept in. If the commit succeeds
  but the push fails, keep the local commit and show a banner "Alterações não
  enviadas ao GitHub" with a "Tentar enviar de novo" button (`git push`);
  never reset. If pull fails (conflict, offline), abort before touching
  `licenses.json` and show the git message. A newly generated key pair is
  kept even if the git step fails (retry reuses the saved entry; nothing is
  deleted automatically).
- **Ids are never reused.** A buyer's DLL is compiled with a fixed licence
  id, so reusing an id would let old players' DLLs accept the new server.
  `licenses.json` gains a top-level `"retiredIds": [..]` array (the Worker
  ignores unknown fields); Delete moves the id there. Next id =
  max(all ids in `licenses`, all `retiredIds`, ids parsed from
  `<name>-<id>.key` files in the keys folder and its `deleted/`) + 1.
- **Hand-off folder instead of hand-edited paths.** Besides the copy button,
  "Preparar envio" creates `<keys>\envio\<name>-<id>\` with the key copied as
  `dll_license.key` and a `LEIA-ME.txt` with the three config.lua lines
  (path `/home/SEU_USUARIO/.alurianot/dll_license.key`) and the same steps
  as the tharcio LEIA-ME, then opens it in Explorer. The copied config text
  uses the same placeholder path. For licence 1, whose key is not in the keys
  folder, these buttons say the key is not on this PC.

## Left to the builder

Visual details, component structure, error message wording (Portuguese),
app icon (placeholder fine), where exactly Settings lives in the UI.

## Location

`manager/` folder of this repo (Tauri project). Build output (`target/`,
`node_modules/`, `dist/`) git-ignored. Deliverable: a release `.exe` the owner
can run (`npm run tauri build`), plus `npm run tauri dev` for development.
