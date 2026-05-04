# Validation Matrix

Use this matrix after both machines have builds installed.

## Scenarios
- Windows host -> macOS client
- macOS host -> Windows client
- Windows host -> Windows client
- macOS host -> macOS client

## Networks
- Wired LAN
- Wi-Fi LAN

## Cases
1. Pairing code success and rejection path (wrong/expired code).
2. Reconnect after client app restart.
3. Reconnect after host app restart.
4. Clipboard sync (short text, long text, rapid updates).
5. Edge handoff left/right/top/bottom.
6. Emergency release hotkey.
7. Sleep/wake of one endpoint.
8. Temporary network disconnect and recovery.

## Pass criteria
- No input lockups longer than 2 seconds.
- Clipboard converges to latest text without loop storms.
- Pairing is rejected when invalid.
- Reconnect succeeds without deleting trust store.
