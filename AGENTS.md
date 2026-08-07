# Atualizador MapaRadar – Agent Operations Guide

This repository (`maparadar-atualizador`) is the **cross-platform desktop app** that replaces
the legacy Windows-only WPF updater. It is a **Tauri 2** application: a thin Vue 3 frontend
plus a Rust backend that holds all business logic. It logs in with forum credentials,
auto-detects a connected iGO8/NDrive GPS device (USB drive), downloads the latest radar
export from `maparadar-api`, writes it onto the device, and self-updates via GitHub Releases.

It is a sibling repo to `maparadar-api`, `maparadar-mapa`, `maparadar-site`, and
`maparadar-webapp`.

The legacy app lives in `legacy-atualizador-maparadar/` (gitignored) for reference only —
do not modify it.

---

## 1. Quick Reference

```bash
# Install frontend deps
npm install

# Run the full desktop app (Vite on :1420 + native Tauri window)
npm run tauri dev

# Frontend only in a browser (Tauri commands are NOT available here)
npm run dev

# Build frontend (vue-tsc type-check + vite build -> dist/)
npm run build

# Rust unit tests (30 tests)
cd src-tauri && cargo test

# Rust debug binary
cd src-tauri && cargo build

# Regenerate platform icons from a 512x512 source PNG
npx tauri icon public/app-icon.png

# Release: tag, push, CI builds signed installers + latest.json
git tag vX.Y.Z && git push origin vX.Y.Z

# After the release workflow finishes, the GitHub release MUST be published
# (not a draft): the self-updater downloads `latest.json` via
# /releases/latest/download/latest.json, and GitHub's "latest" endpoint
# 404s for draft releases. The workflow does NOT create drafts anymore
# (releaseDraft was removed from release.yml). If a release ends up as a
# draft for any reason, publish it manually:
gh release edit vX.Y.Z --draft=false
```

**Linux build prerequisites** (required for any `cargo check`/`build` of the Tauri app):

```bash
# Fedora
sudo dnf install -y webkit2gtk4.1-devel libsoup3-devel librsvg2-devel
# Debian/Ubuntu (also used in CI)
sudo apt-get update && sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf xdg-utils
```

Verify: `pkg-config --exists webkit2gtk-4.1 && pkg-config --exists libsoup-3.0 && echo OK`

---

## 2. Repository Layout

| Path | Purpose |
|---|---|
| `src/` | Vue 3 + TypeScript frontend |
| `src/main.ts` | Vue entry point |
| `src/App.vue` | Session-based routing (Login ↔ Main) |
| `src/api.ts` | Typed Tauri `invoke()` wrappers + `toAppError()` |
| `src/types.ts` | Shared DTO interfaces |
| `src/views/LoginView.vue` | Login form |
| `src/views/MainView.vue` | Device status, alert-type pills, update button, self-update banner |
| `src/styles.css` | Global CSS design tokens + shared classes |
| `public/` | Static assets: `logo.svg`, alert-type SVG icons, `app-icon.png` |
| `src-tauri/` | Tauri 2 + Rust backend |
| `src-tauri/src/*.rs` | Rust modules (see §4) |
| `src-tauri/tauri.conf.json` | Tauri config (window, bundle, CSP, updater) |
| `src-tauri/capabilities/default.json` | Capability permissions (core, opener, process, updater) |
| `.github/workflows/release.yml` | Release pipeline on `v*` tags |
| `legacy-atualizador-maparadar/` | Old WPF app — **gitignored, reference only** |
| `maparadar-atualizador.key` | Tauri updater signing key — **gitignored, NEVER commit** |

---

## 3. Architecture

**All business logic lives in Rust; the Vue UI is thin and presentational.**

```
Vue UI (webview)
   ⇄  Tauri commands (src-tauri/src/commands.rs)
   ⇄  Rust modules
        ├── api.rs     (reqwest client: login / refresh / export / preview)
        ├── device.rs  (USB drive enumeration + iGO8/NDrive detection)
        ├── writer.rs  (atomic file writes onto devices)
        ├── session.rs (JWT + refresh-token persistence)
        └── alert_types.rs (canonical alert-type catalog)
```

Key design principles:

- **Testable core**: logic lives in Rust and is covered by `cargo test` unit tests
  (`#[cfg(test)]` blocks colocated with each module). The UI contains no business logic.
- **Tauri commands are thin**: they orchestrate modules, never re-implement them.
- **No surprises on user devices**: the app never writes without an explicit click, and the
  writer only ever receives paths produced by device detection (never user-supplied folders).
- **pt-BR end-user strings**: every user-facing message is Brazilian Portuguese.

---

## 4. Rust Modules (`src-tauri/src/`)

| File | Responsibility |
|---|---|
| `main.rs` | Entry point; calls `maparadar_atualizador_lib::run()` |
| `lib.rs` | `AppState` (`ApiClient` + `config_dir`), plugin registration, command handler |
| `error.rs` | `AppError` enum; serializes as `{ kind, message }` for the frontend |
| `alert_types.rs` | Canonical 7-type catalog + `default_selected()` / `radar_types_string()` |
| `session.rs` | Load/save/clear `Session` (JSON in config dir); atomic save; corrupt-file recovery |
| `api.rs` | `ApiClient` (reqwest): `login`, `refresh`, `export_updater`, `preview_count`, `now_unix` |
| `device.rs` | `DeviceKind`, `DetectedDevice`, `removable_mount_points`, `detect`, `detect_in_drive`, `scan` |
| `writer.rs` | `WriteSummary`, `write_igo8` (atomic `speedcam.txt` + `.spdb` cleanup), `write_ndrive` |
| `commands.rs` | 7 Tauri commands + DTOs (`SessionInfo`, `DeviceInfo`, `UpdateSummary`) |

### Error contract

`AppError` serializes as `{ "kind": "<camelCase>", "message": "<pt-BR string>" }`. Kinds:

| Kind | Meaning |
|---|---|
| `network` | Transport/connection failure |
| `invalidLogin` | Wrong credentials |
| `unauthorized` | Token expired/invalid → frontend forces re-login |
| `emptyExport` | No points matched the selected types |
| `deviceNotFound` | No compatible device detected |
| `api` | Server returned an error |
| `io` | File system error (including partial-write failures) |
| `session` | Local session storage error |

The frontend uses `toAppError()` and switches on `kind` where behavior differs
(e.g. `unauthorized` → `logout()` + return to Login).

---

## 5. Tauri Command ↔ Frontend Contract

Tauri v2 auto-converts JS camelCase argument keys to Rust snake_case parameters
(e.g. frontend `{ radarTypes }` → Rust `radar_types`). All DTOs use
`#[serde(rename_all = "camelCase")]`.

| Command | Frontend fn | Args | Returns |
|---|---|---|---|
| `get_alert_types` | `getAlertTypes()` | — | `AlertType[]` |
| `login` | `login(username, password)` | `username`, `password` | `SessionInfo` |
| `logout` | `logout()` | — | `void` |
| `session_status` | `sessionStatus()` | — | `SessionInfo \| null` |
| `detect_device` | `detectDevice()` | — | `DeviceInfo[]` |
| `preview_count` | `previewCount(radarTypes)` | `radarTypes` | `number` |
| `update_device` | `updateDevice(kind, radarTypes)` | `kind: 'igo8' \| 'ndrive'`, `radarTypes` | `UpdateSummary` |

DTOs (as the frontend sees them, camelCase):

```ts
interface SessionInfo { username: string; expiresAtUnix: number }
interface DeviceInfo  { kind: 'igo8' | 'ndrive'; display: string; drive: string }
interface AlertType   { code: number; label: string; icon: string; default: boolean }
interface UpdateSummary { filesWritten: string[]; filesDeleted: string[] }
```

When a command fails, the `invoke()` promise rejects with the serialized `{ kind, message }`.
Always surface errors to the user via `toAppError(e).message`.

---

## 6. API Contract (`maparadar-api`)

The app talks to **`https://api.maparadar.com`** (hardcoded in `src-tauri/src/lib.rs` — no
env override in v1; edit that line if you need to test against a local API).

| Endpoint | Auth | Purpose |
|---|---|---|
| `POST /auth/login` | Public | `{ username, password }` → `{ access_token, expires_in, user: { username, groupId } }` + `maparadar_refresh` cookie |
| `POST /auth/refresh` | Cookie | Sends `maparadar_refresh`; returns a fresh token and rotates the cookie |
| `POST /export/updater` | `[Authorize]` Bearer | `{ exportType, radarTypes }` → raw file bytes (204 = no points) |
| `GET /export/preview?radarTypes=...` | Public | `{ count }` — point count for selected types |

Notes:

- **JWT lifetime is 24h**; the app transparently refreshes via the `maparadar_refresh`
  cookie (30d) so users don't re-login daily. On refresh failure → login screen.
- **Referer-gate exemption**: `POST /export/updater` and `GET /export/preview` are exempt
  from the `[ValidateRequest]` referer gate in `maparadar-api` (a desktop client has no
  browser referer). `POST /export` and `POST /export/mobile` still require it. If you change
  the gate in `maparadar-api`, keep the regression test
  (`ValidateRequestPlacementTests.cs`) in sync.
- `ExportType` is `igo8` or `ndrive` (must match `DeviceKind::export_type()`).

---

## 7. Alert Types (single source of truth)

The catalog in `alert_types.rs` **must stay in sync** with
`maparadar-site/src/index.live.html`. Codes and defaults are contract-critical.

| Code | Label (pt-BR) | Icon (public/icons/) | Default |
|---|---|---|---|
| 1 | Radar Fixo | `fixed_110km.svg` | on |
| 2 | Radar Móvel | `mobile_110km.svg` | on |
| 4 | Semáforo c/ Câmera | `traffic_camera.svg` | on |
| 5 | Semáforo c/ Radar | `traffic_light_80km.svg` | on |
| 6 | Polícia Rodoviária | `highway_patrol.svg` | on |
| 7 | Pedágio | `toll.svg` | off |
| 9 | Lombada | `speed_bump.svg` | off |

Selected codes are joined comma-separated and sent as `radarTypes`.

---

## 8. Self-Updater

- **Plugins**: `tauri-plugin-updater` (Rust + JS) and `tauri-plugin-process` (JS `relaunch()`).
- **Update source**: GitHub Releases `latest.json` at
  `https://github.com/yzraeu/maparadar-atualizador/releases/latest/download/latest.json`.
- **Flow**: on MainView mount, `checkForUpdate()` checks; if a newer version exists, show a
  banner; on user consent, `downloadAndInstall()` then `relaunch()`. Never silent.
- **Signing**: the public key is baked into `tauri.conf.json` → `plugins.updater.pubkey`.
  The **private key** is `maparadar-atualizador.key`, **gitignored** — never commit it.
- **CI secrets** (repo → Settings → Secrets): `TAURI_SIGNING_PRIVATE_KEY` (file content) and
  `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`.
- **Updater artifacts**: `tauri.conf.json` → `bundle.createUpdaterArtifacts` **must be `true`**.
  Without it the builds produce no `.sig` signature files, so `tauri-action` skips uploading
  `latest.json` entirely and updates silently never appear. This bit us once (v0.2.0/v0.2.1 had
  no `latest.json`). `tauri-action` merges per-platform entries into a single `latest.json`, so
  the 3-OS matrix is fine as-is.
- **macOS caveat**: auto-update requires a Developer ID–signed build. Until `APPLE_*`
  secrets are configured, macOS users install the DMG manually; the updater banner may show
  but install will fail there. The release body documents this.
- **Windows SmartScreen caveat**: installers are currently **not Authenticode-signed**.
  Users may see "O Windows protegeu o seu PC" / unknown publisher on first install.
  Workaround: **Mais informacoes -> Executar assim mesmo**. This is expected while we
  have updater signing only (`TAURI_SIGNING_PRIVATE_KEY`) and no Windows code-signing cert.
  To remove the warning in future releases, we need OV/EV code signing (preferably cloud
  signing so GitHub Actions can sign during CI).
- **Draft releases break updates**: the updater hits `/releases/latest/download/latest.json`,
  which GitHub serves only for **published** (non-draft) releases. A draft release returns
  404 and the app silently shows no update. The workflow publishes releases automatically;
  if a release is ever left as a draft, publish it with
  `gh release edit vX.Y.Z --draft=false`.

---

## 9. Code Style

### Rust
- `rustfmt` defaults (4-space indent, Allman braces — matches sibling repos).
- `thiserror` for error types; propagate with `?` via `From` impls.
- Tests colocated in `#[cfg(test)] mod tests`; use `tempfile` and `httpmock` where relevant.
- User-facing strings are pt-BR.
- No `unwrap()` in production paths; no panic-prone code in library modules.

### Vue / TypeScript
- `<script setup lang="ts">` composition API.
- Views are thin: no business logic; all IPC through `src/api.ts`.
- Scoped styles referencing CSS variables from `src/styles.css` (brand `#e6241a`,
  tokens `--brand`, `--bg`, `--card`, `--text`, `--muted`, `--ok`, `--err`, `--border`).
- No semicolons (repo convention), strict TS (`noUnusedLocals`/`noUnusedParameters` on).
- Always use `toAppError(e).message` for surfaced errors.
- **UI language: Brazilian Portuguese (pt-BR) only.**

---

## 10. Security & Secrets

- **NEVER commit** `*.key`, `*.key.pub`, `.env`, or `.env.*` (all gitignored).
- The updater pubkey is safe to commit (it is already in `tauri.conf.json`).
- CSP is set in `tauri.conf.json`; `connect-src` must allow `api.maparadar.com` and the
  GitHub release hosts (updater). If you add new network hosts, update the CSP.
- The session file (`~/.config/maparadar-atualizador/session.json`) stores the JWT + refresh
  token in plaintext — acceptable for this threat model (radar-update access only). OS
  keyring is a possible future improvement; do not silently add heavy dependencies for it.
- Signing secrets exist only as GitHub Actions secrets — never in source.

---

## 11. Gitignored

`node_modules/`, `dist/`, `src-tauri/target/`, `src-tauri/gen/schemas/`,
`legacy-atualizador-maparadar/`, `*.key`, `*.key.pub`, `.env`, `.env.*`, `!.env.example`.

---

## 12. Agent Execution Rules

1. **Read before writing.** Inspect the relevant Rust module, its tests, `commands.rs`,
   `src/api.ts`, and `src/types.ts` before making changes.
2. **Keep logic in Rust.** New behavior belongs in a Rust module with `cargo test` coverage;
   keep the Vue views presentational.
3. **Verify before claiming done.** Run `cd src-tauri && cargo test` and
   `npm run build` after any change; fix errors before reporting.
4. **TDD for Rust modules.** Write the failing test first, then implement, then confirm
   green.
5. **Never commit the signing key.** Check `git status` before committing; if
   `maparadar-atualizador.key` appears, do not stage it.
6. **Respect the alert-type contract.** Any change to `alert_types.rs` must be reflected in
   `maparadar-site/src/index.live.html` (and vice versa).
7. **Cross-repo changes.** If you change an API endpoint contract, update `maparadar-api`
   and its consumers. The referer-gate exemption in `maparadar-api` is a hard dependency of
   this app.
8. **pt-BR strings.** All end-user-facing text is Brazilian Portuguese.
9. **Minimal footprint.** Prefer editing existing files; no README or doc files unless
   instructed.
10. **No placeholders.** Write complete, production-ready code.
