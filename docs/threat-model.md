# Threat Model (v1)

## Assets
- Keyboard and mouse input stream.
- Clipboard text payloads.
- Trusted peer identities.

## Trust boundaries
- Untrusted local network.
- Trusted app peers after explicit pairing.

## Security controls
- Pairing code has TTL and single active value.
- Trusted peers are fingerprint-pinned in local storage.
- Plaintext sessions are disallowed by design; TLS transport is required.
- Clipboard content should not be logged in production.

## Known limitations
- Placeholder TLS certificate provisioning requires replacement with generated cert/key material before production rollout.
- Input adapter internals need OS-native hardened implementations.

## Future hardening
- Mutual TLS with per-device cert rotation.
- Signed update channel.
- Audit logging with tamper protection.
