# Sync Architecture

## Runtime components
- Tauri app shell with a React UI.
- Rust core runtime handles secure session state and trust persistence.
- Platform adapters expose hooks for OS-specific input capture/injection and clipboard operations.

## Connection and trust flow
1. Host generates short-lived pairing code.
2. Client sends pairing request with local identity/fingerprint.
3. Host validates code and writes trusted peer record.
4. Subsequent sessions are accepted based on trust-store match.

## Messaging
- `InputEvent` for key and mouse forwarding.
- `ClipboardTextUpdate` for text clipboard sync.
- `FocusChange` for edge handoff ownership.
- `Heartbeat` for liveness/reconnect logic.

## Current status
- Protocol and trust store are implemented.
- Input adapter functions are scaffolded and compile-safe.
- Clipboard and edge handoff commands are exposed through Tauri.
- Production OS-specific native hook/injection internals are marked as next hardening step.
