# MapaRadar Atualizador

A lightweight cross-platform desktop app (Windows, macOS, Linux) that downloads the latest
MapaRadar radar database and writes it directly onto your iGO8 or NDrive GPS device.

It replaces the legacy Windows-only WPF updater with a modern Tauri 2 app: log in, plug in
your GPS, pick the alert types you want, and click **Atualizar dispositivo** — done. The app
also self-updates via GitHub Releases.

## Features

- **Cross-platform** — native installers for Windows, macOS, and Linux (~5-10 MB, no runtime)
- **Auto-detects your device** — recognizes iGO8 (`content/speedcam`) and NDrive (`speedcams`)
  folders on any removable USB drive
- **Alert-type selection** — choose which radar types to include (7 types, same as the site)
- **One-click update** — downloads the export and writes it atomically to the device
- **Self-updating** — prompts when a new app version is available, installs on consent
- **Session persistence** — stays logged in (JWT + refresh token), no daily re-login

## Install & Run

Requirements: Node.js ≥ 20 and a Rust stable toolchain. On Linux, the Tauri build needs
webkit/GTK system packages:

```bash
# Fedora
sudo dnf install -y webkit2gtk4.1-devel libsoup3-devel librsvg2-devel
# Debian/Ubuntu
sudo apt-get install -y libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf xdg-utils
```

```bash
npm install
npm run tauri dev     # Run the desktop app (Vite :1420 + native window)
npm run build         # Frontend type-check + production build → dist/
cd src-tauri && cargo test   # Rust unit tests (30)
```

Prebuilt installers are published on the
[GitHub Releases](https://github.com/yzraeu/maparadar-atualizador/releases) page.

## Usage

1. **Login** with your forum credentials (same account used on `forum.maparadar.com`).
2. **Plug in** your GPS device (iGO8 or NDrive).
3. The app **detects** the device automatically.
4. Select the **alert types** you want (defaults match the site export).
5. Click **Atualizar dispositivo**.
6. The radar file is written to the device and you're ready to go.

### Supported devices

| Device | Detection | File written |
|---|---|---|
| iGO8 | folder ending in `content/speedcam` | `speedcam.txt` (stale `.spdb` cleaned) |
| NDrive | folder ending in `speedcams` | `maparadar.kml` |

## Requirements

- A MapaRadar forum account
- A USB-connected iGO8 or NDrive GPS device
- macOS auto-update requires a Developer ID–signed build (until configured, install the DMG
  manually)

## Architecture

| Layer | Files | Purpose |
|---|---|---|
| Vue UI | `src/` | Login + main screens (thin, presentational) |
| Tauri commands | `src-tauri/src/commands.rs` | IPC orchestration (7 commands) |
| Rust core | `src-tauri/src/{api,device,writer,session}.rs` | API client, device detection, file writing, session |
| Config | `src-tauri/tauri.conf.json` | Window, bundle, CSP, updater |

All business logic lives in Rust and is unit-tested with `cargo test`; the UI stays thin.
For contributors, see `AGENTS.md` for the full operations guide.

## Development

- `npm run tauri dev` — full app with hot reload
- `npm run dev` — frontend only in a browser (Tauri commands unavailable)
- `npm run build` + `cd src-tauri && cargo build` — local release build
- `npx tauri icon public/app-icon.png` — regenerate platform icons

The app talks to `https://api.maparadar.com` (hardcoded in `src-tauri/src/lib.rs`).

## Release

Tagging `vX.Y.Z` triggers CI (`tauri-action`) to build signed installers + the updater
`latest.json` for all three platforms and publish a draft GitHub Release.

```bash
git tag v0.1.0 && git push origin v0.1.0
```

## License

The MapaRadar radar data is licensed under CC BY-NC-ND 4.0. This app is the community tool
that delivers that data to your GPS.
