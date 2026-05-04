# Setup and Permissions

## Windows
1. Build the desktop bundle from `apps/desktop` with `npm run tauri:build`.
2. Install the generated MSI/EXE.
3. Run app as normal user. If global hooks are added later, elevation may be needed for some secure desktop contexts.

## macOS
1. Build the desktop bundle from `apps/desktop` with `npm run tauri:build`.
2. Install the generated `.app`/`.dmg`.
3. Grant permissions in System Settings:
   - Accessibility
   - Input Monitoring
4. Restart the app after permissions are granted.

## Pairing flow
1. On Host: generate pairing code.
2. On Client: enter host address and pairing code, then connect.
3. Verify trusted peer appears in list.
4. Configure edge layout and test handoff.
