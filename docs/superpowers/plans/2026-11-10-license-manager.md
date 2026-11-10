# AlurianOT License Manager Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** A Tauri 2 desktop app (Portuguese UI) that lists, creates, revokes, reactivates and deletes ddraw.dll licences in `licenses.json`, generates server keys, pushes via git, shows config lines, prepares a hand-off folder and checks the Worker token.

**Architecture:** Rust backend split into pure modules (licences model, keys, token decoding, hand-off text) with unit tests, plus thin side-effect modules (git, HTTP, settings, filesystem) orchestrated by Tauri commands. Vanilla TypeScript + Vite frontend: one table screen, a "Nova licença" dialog, a Settings panel and a details drawer.

**Tech Stack:** Tauri 2, Rust (serde/serde_json with `preserve_order`, p256 0.13 `pkcs8`+`pem`, rand_core `getrandom`, reqwest 0.12 `blocking`+`rustls-tls`, base64 0.22, chrono 0.4, tempfile dev), Node + Vite 5 + TypeScript, `@tauri-apps/api` 2, `@tauri-apps/cli` 2.

**Spec:** `docs/superpowers/specs/2026-11-10-license-manager-design.md`

## Global Constraints

- Project lives in `manager/` of this repo; `manager/node_modules/`, `manager/dist/`, `manager/src-tauri/target/` git-ignored.
- Windows x64 only; all child processes (git, explorer) spawned with `CREATE_NO_WINDOW` (0x08000000) except Explorer itself.
- Defaults: repo folder `%USERPROFILE%\Documents\GitHub\alurianot-dll-licenses`, keys folder `C:\tmp\licenses`, Worker base `https://alurianot-dll-licenses.thiago-freitasferraz.workers.dev`.
- `licenses.json`: 2-space indent, trailing newline, top-level key order `leaseHours`, `licenses`, `retiredIds`; licence field order `id, name, serverPublicKey, revoked, expires, note`.
- Name: lowercase a-z0-9-_ , 1..32 chars, unique. Expires: `null` or `YYYY-MM-DD` (inclusive, UTC).
- Key files: `<name>-<id>.key` (PKCS#8 PEM, EC P-256) + `<name>-<id>.json` `{"name","licenseId","publicKey"}` in the keys folder; public key = 128 lowercase hex (X||Y).
- Next id = max(ids in `licenses`, `retiredIds`, ids parsed from `*-<id>.key` in keys folder and `keys/deleted/`) + 1. Ids never reused.
- Git per change: `git pull --rebase --autostash` → write file → `git commit -m "<msg>" -- licenses.json` → `git push`. Push failure keeps the commit; never `reset`.
- Commit messages: `licença nova: <name> (#<id>)`, `revogar: <name> (#<id>)`, `reativar: <name> (#<id>)`, `apagar: <name> (#<id>)`.
- Config lines (exact):
  ```
  dllLicenseKeyFile = "/home/SEU_USUARIO/.alurianot/dll_license.key"
  dllLicenseId = <id>
  dllLicenseTokenUrl = "<workerBase>/tokens/<id>.tok"
  ```
- Never read, show or transmit `master.key`.

## Review Focus

- `licenses.json` invalid or hand-edited badly → the app shows "licenses.json inválido: <erro>" and refuses every write (no overwrite).
- Name typed with spaces/uppercase/accents ("João Silva") → rejected with a Portuguese message; uppercase alone is lowercased.
- Offline / push rejected → banner "Alterações não enviadas ao GitHub" + "Tentar enviar de novo"; local commit kept.
- Other files modified or staged in the repo → only `licenses.json` ends up in the commit.
- Worker unreachable / timeout / 404 / garbage body → status cell shows "sem resposta", "não servido" or "resposta inválida", no crash.

---

### Task 1: Scaffold Tauri app with licences model

**Files:**
- Create: `manager/package.json`, `manager/vite.config.ts`, `manager/tsconfig.json`, `manager/index.html`, `manager/src/main.ts`, `manager/src/style.css`
- Create: `manager/src-tauri/Cargo.toml`, `manager/src-tauri/build.rs`, `manager/src-tauri/tauri.conf.json`, `manager/src-tauri/capabilities/default.json`, `manager/src-tauri/icons/icon.ico` (placeholder generated with `npx tauri icon` from any PNG)
- Create: `manager/src-tauri/src/main.rs`, `manager/src-tauri/src/licenses.rs`
- Modify: `.gitignore`

**Interfaces:**
- Produces (`licenses.rs`):
  - `struct License { id: u32, name: String, server_public_key: String, revoked: bool, expires: Option<String>, note: String }` (serde rename camelCase → `serverPublicKey`)
  - `struct LicenseFile { lease_hours: u32, licenses: Vec<License>, retired_ids: Vec<u32> }` (`retiredIds` defaults to empty when absent)
  - `fn parse(text: &str) -> Result<LicenseFile, String>` (validates every entry: name rule, 128-hex key, date format, unique ids/names)
  - `fn to_json(file: &LicenseFile) -> String` (exact format from Global Constraints)
  - `fn normalize_name(input: &str) -> Result<String, String>`
  - `fn next_id(file: &LicenseFile, key_file_ids: &[u32]) -> u32`
  - `enum Status { Ativa, Revogada, Vencida }` + `fn status(l: &License, today: chrono::NaiveDate) -> Status`

- [ ] **Step 1: Write failing tests in `licenses.rs`**
  - `parse_roundtrip_keeps_format`: parsing the current repo `licenses.json` text (embed via `include_str!("../../../licenses.json")`) then `to_json` → equals original text with `"retiredIds": []` appended after `licenses` (assert the exact string for a 1-licence sample instead, to stay stable).
  - `parse_rejects_bad_key`: entry with 127-hex key → `Err` containing `"serverPublicKey"`.
  - `normalize_name_rules`: `"Tharcio"`→`Ok("tharcio")`, `"joão"`→Err, `"a b"`→Err, `""`→Err, 33 chars→Err.
  - `next_id_never_reuses`: licenses ids [1,2], retired [5], key files [3] → 6; empty everything → 1.
  - `status_rules`: revoked → Revogada; expires "2026-11-09" with today 2026-11-10 → Vencida; expires "2026-11-10" today 2026-11-10 → Ativa.
- [ ] **Step 2: Run** `cargo test --manifest-path manager/src-tauri/Cargo.toml` → FAIL (not implemented).
- [ ] **Step 3: Implement** `licenses.rs`; `serde_json` with `preserve_order`; `to_json` uses `serde_json::to_string_pretty` (2-space) + `"\n"`.
- [ ] **Step 4: Run tests** → PASS. Run `npm install` and `npm run tauri dev` in `manager/` → window titled "AlurianOT License Manager" opens (empty page OK).
- [ ] **Step 5: Commit** `feat(manager): scaffold Tauri app and licences model`.

### Task 2: Key generation and key-file discovery

**Files:**
- Create: `manager/src-tauri/src/keys.rs`

**Interfaces:**
- Produces:
  - `struct KeyPair { pem: String, public_hex: String }`
  - `fn generate() -> KeyPair`
  - `fn public_hex_from_pem(pem: &str) -> Result<String, String>`
  - `fn save(keys_dir: &Path, name: &str, id: u32, pair: &KeyPair) -> Result<PathBuf, String>` (writes `.key` + `.json`, refuses to overwrite existing `.key`)
  - `fn key_path(keys_dir: &Path, name: &str, id: u32) -> PathBuf`
  - `fn ids_in_dir(keys_dir: &Path) -> Vec<u32>` (scans `keys_dir` and `keys_dir/deleted`, parses `*-<digits>.key`)
  - `fn move_to_deleted(keys_dir: &Path, name: &str, id: u32) -> Result<(), String>` (missing file = Ok)

- [ ] **Step 1: Failing tests** (tempfile dirs):
  - `generate_roundtrip`: `public_hex_from_pem(&p.pem) == p.public_hex`, `public_hex.len()==128`, pem starts with `"-----BEGIN PRIVATE KEY-----"`.
  - `reads_python_key`: `public_hex_from_pem` on `C:\tmp\licenses\tharcio-2.key` equals `"f832528781da14339f557b67c9e2ffe94c4b72df3c39da4266fb8592d73a26ce122869342eada05856ad2e523727afa846643ba0f3ef0f9a2e43a46290f0b40b"` (`#[ignore]` when the file is absent).
  - `save_refuses_overwrite`, `ids_in_dir_reads_deleted` (files `a-3.key`, `deleted/b-7.key`, `x.key` → [3,7]).
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement** with `p256::SecretKey::random(&mut OsRng)`, `to_pkcs8_pem(LineEnding::LF)`, `public_key().to_encoded_point(false)` bytes 1..65 → hex.
- [ ] **Step 4: Run** → PASS. **Step 5: Commit** `feat(manager): server key generation`.

### Task 3: Git operations

**Files:**
- Create: `manager/src-tauri/src/gitops.rs`

**Interfaces:**
- Produces:
  - `fn pull(repo: &Path) -> Result<(), String>`
  - `fn commit_licenses(repo: &Path, message: &str) -> Result<(), String>`
  - `fn push(repo: &Path) -> Result<(), String>`
  - `fn ahead_count(repo: &Path) -> Result<u32, String>` (`git rev-list --count @{u}..HEAD`)
  - Errors carry git's stderr trimmed.

- [ ] **Step 1: Failing integration tests** (tempfile: bare remote + clone with a `licenses.json` and `other.txt`):
  - `commit_only_licenses`: modify and `git add` `other.txt`, modify `licenses.json`, `commit_licenses` → `git show --name-only HEAD` lists only `licenses.json`; `other.txt` still staged.
  - `push_failure_keeps_commit`: point `origin` at a nonexistent path, commit, `push` → Err; `ahead_count` errors or HEAD still contains the commit (assert `git log -1 --format=%s` == message).
  - `pull_with_dirty_tree`: dirty `other.txt`, `pull` → Ok (autostash), change still present.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement** via `std::process::Command::new("git").current_dir(repo)` + `creation_flags(0x08000000)`.
- [ ] **Step 4: Run** → PASS. **Step 5: Commit** `feat(manager): git operations`.

### Task 4: Token status and hand-off

**Files:**
- Create: `manager/src-tauri/src/worker.rs`, `manager/src-tauri/src/handoff.rs`

**Interfaces:**
- Produces:
  - `enum TokenStatus { Served { expires_at: i64 }, NotServed, Invalid, Unreachable(String) }` (serde tagged `kind`)
  - `fn decode_expiry(body: &str) -> Option<i64>` (base64 → 153 bytes, magic `ALLT`, bytes 17..25 u64 LE)
  - `fn fetch_status(base: &str, id: u32) -> TokenStatus` (5 s timeout; 404 → NotServed)
  - `fn config_text(base: &str, id: u32) -> String` (exact three lines from Global Constraints, `\n`-joined, trailing newline)
  - `fn prepare(keys_dir: &Path, name: &str, id: u32, base: &str) -> Result<PathBuf, String>` → `<keys>\envio\<name>-<id>\dll_license.key` + `LEIA-ME.txt` (config text + steps: copy key to `~/.alurianot/`, chmod 600, edit config.lua, restart, needs curl/internet, `[DllLicense] Lease token valid until ...` in log); Err "A chave desta licença não está neste PC." when the key file is missing.

- [ ] **Step 1: Failing tests:** `decode_expiry_reads_field` (build 153 bytes: `ALLT`,1,id,issued,expires=1762960591, 128 zero bytes → base64 → `Some(1762960591)`), `decode_rejects_garbage` (`"abc"`, wrong magic → None), `config_text_exact` (id 3 → exact string), `prepare_writes_files` and `prepare_missing_key`.
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement** (`reqwest::blocking::Client::builder().timeout(5s)`).
- [ ] **Step 4: Run** → PASS. **Step 5: Commit** `feat(manager): token status and hand-off folder`.

### Task 5: Settings and Tauri commands

**Files:**
- Create: `manager/src-tauri/src/settings.rs`, `manager/src-tauri/src/commands.rs`
- Modify: `manager/src-tauri/src/main.rs`

**Interfaces:**
- Consumes: Tasks 1–4.
- Produces (Tauri commands, camelCase args, all return `Result<_, String>`):
  - `get_settings() -> Settings { repoDir, keysDir, workerBase }`, `save_settings(settings)` (JSON in `app_config_dir()/settings.json`)
  - `list_licenses() -> Vec<LicenseView { id, name, status: "ativa"|"revogada"|"vencida", expires, note, hasKey }>`
  - `create_license(name, expires: Option<String>, note) -> OpResult { pushed: bool, message: String }`
  - `set_revoked(id, revoked) -> OpResult`, `delete_license(id) -> OpResult`, `retry_push() -> OpResult`
  - `pending_push() -> bool`
  - `config_text(id) -> String`, `prepare_handoff(id) -> String` (also opens Explorer), `open_key_folder(id)` (`explorer /select,<key>`)
  - `token_status(id) -> TokenStatus`
- Write flow (create/revoke/delete): `pull` → read+`parse` (Err aborts, file untouched) → mutate → `to_json` → write `licenses.json.tmp` + rename → `commit_licenses` → `push` (Err → `pushed:false`, message with git error). Create: `normalize_name`, uniqueness, `next_id(file, ids_in_dir)`, `generate`, `save` before editing JSON. Delete: remove entry, push id to `retiredIds`, `move_to_deleted`.

- [ ] **Step 1: Failing test** in `commands.rs` for the pure core `fn apply_create(file: &mut LicenseFile, name: &str, id: u32, public_hex: &str, expires: Option<String>, note: &str) -> Result<(), String>` and `fn apply_delete(file: &mut LicenseFile, id: u32) -> Result<License, String>` (delete puts id in `retired_ids`; create rejects duplicate name).
- [ ] **Step 2: Run** → FAIL. **Step 3: Implement** commands + register in `main.rs`.
- [ ] **Step 4: Run** `cargo test` → PASS; `npm run tauri dev` → from devtools console `await window.__TAURI__.core.invoke('list_licenses')` returns licences 1 and 2.
- [ ] **Step 5: Commit** `feat(manager): settings and commands`.

### Task 6: Frontend

**Files:**
- Modify: `manager/src/main.ts`, `manager/src/style.css`, `manager/index.html`
- Create: `manager/src/api.ts` (typed `invoke` wrappers for every Task 5 command)

**Interfaces:**
- Consumes: Task 5 command names/types.

- [ ] **Step 1: Build UI:** header "AlurianOT — Licenças" + buttons "Nova licença", "Atualizar", "Configurações"; table columns Id, Nome, Status (colored badge), Validade ("sem validade" for null), Token (button "Verificar" → "até dd/mm hh:mm" / "não servido" / "sem resposta" / "resposta inválida"), Ações (Revogar/Reativar, Config, Enviar, Apagar). Dialog "Nova licença" (nome, validade date input optional, nota). Config drawer: `<pre>` with config text + "Copiar" (navigator.clipboard) + "Abrir pasta da chave". Delete uses `confirm()`-style custom modal "Apagar a licença <nome> (#id)? O id nunca será reutilizado." Banner when `pending_push()` true or an `OpResult.pushed` is false. Errors in a toast. Dark theme.
- [ ] **Step 2: Verify manually** with `npm run tauri dev` against a scratch copy of the repo (Settings → repo folder pointing to a temp clone): create "teste" → appears as #3 ativa, commit+push in the clone; revoke → "revogada"; delete → gone, `retiredIds` contains 3; Verificar on #1 → "até ...".
- [ ] **Step 3: Commit** `feat(manager): license manager UI`.

### Task 7: Release build and docs

**Files:**
- Modify: `README.md` (section "App de licenças"), `manager/src-tauri/tauri.conf.json` (productName "AlurianOT License Manager", identifier `com.alurianot.licenses`, bundle targets `nsis`)

- [ ] **Step 1:** `npm run tauri build` in `manager/` → `manager/src-tauri/target/release/alurianot-license-manager.exe` exists and opens.
- [ ] **Step 2:** README: how to run the exe, defaults, that it never touches the master key.
- [ ] **Step 3: Commit** `docs(manager): build and usage`.
