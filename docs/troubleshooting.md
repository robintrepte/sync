# Troubleshooting

## Rust compile fails with rustc toolchain error
Run:
```bash
rustup self update
rustup update stable
rustup default stable-x86_64-pc-windows-msvc
rustup component add rustc cargo rust-std
```
Then retry:
```bash
cd apps/desktop/src-tauri
cargo check
```

## Pairing fails
- Ensure host generated a fresh code.
- Confirm host listener bind address/port is reachable on LAN.
- Verify both machines are on same subnet.

## Clipboard does not update
- Confirm app has OS permission for clipboard access.
- Verify heartbeat/network status in the app is not reporting an error.

## macOS input not captured/injected
- Grant Accessibility and Input Monitoring permissions.
- Restart app after changing permissions.
