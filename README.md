# Project Modzboid

Desktop mod manager for Project Zomboid B42. Built with Tauri (Rust + React).

## Features

- **Mod Manager** — drag-and-drop load order, search, bulk enable/disable
- **Server Profiles** — link to server.ini files, sync mod lists, dirty state tracking
- **Compatibility Scanner** — detect deprecated properties, removed APIs, case mismatches
- **Auto-Fixer** — generate modpack fix mods that patch script errors without touching originals
- **Diagnostics** — crash log analysis, preflight checks, mod bisect tool
- **Backup & Restore** — snapshot and restore load orders
- **Mod Sharing** — export/import load orders as shareable files
- **API Documentation** — searchable PZ Lua/Java API reference (via [extension](https://github.com/AyAmbo/modzboid-extensions))
- **RCON Console** — send commands to a running dedicated server
- **Extension System** — install community data packs (API docs, migration rules)

## Install

Download the latest release from [Releases](../../releases).

## Build from Source

Requirements: [Node.js](https://nodejs.org/) 18+, [Rust](https://rustup.rs/) 1.75+, platform dependencies for [Tauri v2](https://v2.tauri.app/start/prerequisites/).

```bash
npm install
npm run tauri build
```

The built installer will be in `src-tauri/target/release/bundle/`.

## Extensions

Modzboid supports installable extension packs for game-derived data:

- **[modzboid-extensions](https://github.com/AyAmbo/modzboid-extensions)** — PZ API docs and migration rules

## Known Issues

- **Windows scaling** — UI elements may render incorrectly at non-100% display scaling
- **API docs search** — searching broad terms can be slow due to large result sets (4,100+ pages)
- **Light theme** — incomplete; dark theme is recommended

## Built With

This project was built with the help of [Claude Code](https://claude.ai/code) by Anthropic.

## License

[AGPL-3.0](LICENSE) — see [CLA.md](CLA.md) for contributor terms.

## Notice

Project Zomboid is a trademark of The Indie Stone Ltd. This project is not affiliated with or endorsed by The Indie Stone.
