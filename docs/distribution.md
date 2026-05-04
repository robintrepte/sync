# Distribution: installers and double‑click apps

This project is a **Tauri** desktop app. The standard, user‑friendly way to ship it is **installers built by Tauri**, not asking users to run `npm` or `cargo`.

## What users get

| Platform | Typical artifact | User experience |
|----------|-------------------|-----------------|
| **Windows** | `.msi` and/or **NSIS** `.exe` setup | Double‑click installer → Start Menu / desktop shortcut → GUI opens |
| **macOS** | `.dmg` (or `.app` in a zip) | Open DMG → drag app to **Applications** → launch from Launchpad or Spotlight |

The GUI is already the Tauri window; packaging only wraps it so people do not use the terminal.

## Build installers locally

From the **repository root**:

```bash
npm install
npm run tauri:build
```

Or from `apps/desktop`:

```bash
cd apps/desktop
npm install
npm run tauri:build
```

Outputs appear under:

`apps/desktop/src-tauri/target/release/bundle/`

- **Windows**: look under `msi/` and `nsis/` (exact names depend on Tauri defaults).
- **macOS**: look under `dmg/` (and/or `macos/`) when you build **on a Mac**.

## Why installers are not committed to Git

Built `.msi` / `.exe` files are **large binaries**. Git tracks source; **GitHub Releases** are meant for downloadable artifacts. That keeps clones fast and avoids bloating history.

## Publish a GitHub Release (automated)

This repo includes **`.github/workflows/release.yml`**.

1. Bump `version` in `apps/desktop/src-tauri/tauri.conf.json` and `apps/desktop/package.json` / root `package.json` if you version them together (optional but clearer).
2. Create and push a **SemVer tag**:

   ```bash
   git tag v0.1.1
   git push origin v0.1.1
   ```

3. GitHub Actions builds **`npm run tauri:build`** on Windows and **uploads the MSI and NSIS installer** to a Release for that tag.

You can also run the workflow manually (**Actions → Release → Run workflow**) and download **workflow artifacts** from the run summary (same installers, not attached to a Release unless you used a tag).

## Publish manually (upload ZIP yourself)

1. Run `npm run tauri:build` locally.
2. On GitHub: **Releases → Draft a new release → choose tag → attach** the files from `bundle/msi/` and `bundle/nsis/`.

## Best practice (short)

1. **Prefer an installer** (MSI/NSIS on Windows, DMG on macOS) over “here is a loose `.exe`” so shortcuts and uninstall entries exist.
2. **Build Windows installers on Windows**, **macOS bundles on macOS**. Cross‑compiling a signed `.app`/DMG from Windows is not the straightforward path.
3. **Icons**: use one high‑resolution source image and let Tauri generate all sizes:

   ```bash
   cd apps/desktop
   npx tauri icon path/to/icon-1024.png
   ```

   Then rebuild with `npm run tauri:build`.

4. **Code signing** (recommended for public downloads):
   - **Windows**: Authenticode signing (certificate from a CA or internal PKI).
   - **macOS**: Apple Developer ID + notarization so Gatekeeper does not block the app.

   Unsigned builds still run, but users may see SmartScreen (Windows) or Gatekeeper warnings (macOS).

## CI vs local builds

GitHub Actions can run `tauri build` on `windows-latest` to produce Windows artifacts as workflow artifacts. macOS DMGs are usually built on `macos-latest` in a separate job or on your machine.

## Development vs release

- **Development** (hot reload): `npm run tauri:dev` from `apps/desktop` — for you, not for end users.
- **Release**: `npm run tauri:build` — produces installable bundles for others.
