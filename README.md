# pulse

Native Windows health strip for a 1920×440 monitor. **Tauri 2 — not Electron.**

Pulse docks on the short panel as a 960×440 left half (Google Translate can keep the right) or full 1920. It watches internet path, CPU / RAM / GPU, local HTTP services, and process working-set (Cursor, Chrome, etc.). Click a cell for Open / Start / Stop. Pin keeps it on top.

User config is `%APPDATA%\pulse\probes.toml`. First run copies the TK421 preset if `C:\dev\cam` exists, otherwise generic (Net + host meters). **Set** in the titlebar is Settings: cells, presets, Start with Windows, Check now.

```powershell
cd C:\dev\pulse
npm install
npm run tauri dev
```

Installed copy lives in `%LOCALAPPDATA%\pulse\`.

## Release (Windows)

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content -Raw src-tauri\pulse.key
npm run tauri build
.\scripts\make-portable.ps1 -Version 0.1.0
```

NSIS installer: `src-tauri\target\release\bundle\nsis\pulse_*_x64-setup.exe`  
Portable zip: `pulse-*-windows-x64.zip` (exe only; WebView2 is already on Windows 10/11)

Tag `v0.1.4` (or later) to have GitHub Actions publish a Release with the installer, `.sig`, `latest.json`, and portable zip.

The repo must be **public** for Settings → Update → Check now to work. GitHub keeps release assets private on a private repo, so the installed app cannot fetch `latest.json`. Auto-update reads `https://github.com/Randy-L-Thomas/pulse/releases/latest/download/latest.json`.

Updater signing: store `src-tauri\pulse.key` (gitignored) in GitHub Actions secret `TAURI_SIGNING_PRIVATE_KEY`. Optional: `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. If you lose the key, generate a new pair with `npx tauri signer generate` and update `plugins.updater.pubkey` in `src-tauri/tauri.conf.json`. First SmartScreen warning is expected (no Authenticode cert yet).
