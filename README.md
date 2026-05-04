# LAN Input Sync

<p align="center">
  <img src="./assets/hero.jpg" alt="LAN Input Sync hero image" width="100%" />
</p>

<p align="center">
  Seamless keyboard, mouse, and clipboard sharing between Windows and macOS over LAN/Wi-Fi.
</p>

<p align="center">
  <a href="https://github.com/robintrepte/sync/actions/workflows/ci.yml"><img alt="CI" src="https://img.shields.io/github/actions/workflow/status/robintrepte/sync/ci.yml?branch=main&label=CI"></a>
  <a href="./LICENSE"><img alt="License: MIT" src="https://img.shields.io/badge/license-MIT-blue.svg"></a>
  <img alt="Platform" src="https://img.shields.io/badge/platform-Windows%20%7C%20macOS-black">
  <img alt="Built with Tauri" src="https://img.shields.io/badge/built%20with-Tauri-24C8DB">
</p>

## Why This Project

`LAN Input Sync` is a desktop app for sharing one keyboard and mouse across multiple nearby computers, as if they were one setup:

- one keyboard and mouse across machines
- text clipboard synchronization
- edge-based screen handoff
- secure pairing and trusted peer persistence

Designed for dual-computer workflows (for example: Windows desktop + MacBook side-by-side).

## Current Capabilities

- Tauri-based cross-platform desktop application
- Host/client roles with pairing code flow
- Local identity and trusted peer store
- TLS-backed network session commands
- Clipboard text sync with loop prevention
- Configurable edge handoff logic and emergency release
- Runtime heartbeat and network status reporting

## Project Status

This repository is a strong reference implementation and active foundation.  
For production-grade behavior, continue hardening low-level global input capture/injection and run full multi-device validation matrix.

## Quick Start

### Prerequisites

- Node.js 20+
- Rust stable toolchain (`rustup`, `cargo`, `rustc`)
- Tauri OS prerequisites

### Install & Build

```bash
npm install
npm run build
```

### Backend Check

```bash
cd apps/desktop/src-tauri
cargo check
```

## Run Locally

```bash
npm run dev -w apps/desktop
```

For full desktop runtime:

```bash
cd apps/desktop
npm run tauri:dev
```

## Repository Structure

- `apps/desktop` - React frontend + Tauri app
- `apps/desktop/src-tauri` - Rust core runtime, networking, platform adapters
- `shared/proto` - shared message schema
- `docs` - architecture, threat model, setup, validation, troubleshooting

## Documentation

- `docs/architecture.md`
- `docs/threat-model.md`
- `docs/setup.md`
- `docs/validation-matrix.md`
- `docs/troubleshooting.md`

## GitHub

Repository: [github.com/robintrepte/sync](https://github.com/robintrepte/sync)

CI runs on `main` via [`.github/workflows/ci.yml`](.github/workflows/ci.yml). Enable **Actions** in the repo settings if the workflow is disabled.

## License

MIT - see `LICENSE`.
