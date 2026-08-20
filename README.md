# pulse

Strip health dashboard for a 1920×440 monitor. Tauri 2 — not Electron.

```powershell
cd C:\dev\pulse
npm install
npm run tauri dev
```

Starts at 960×440 on the left (Translate keeps the right). **Half** / **Full** snaps width. **Set** opens Settings.

User config lives in `%APPDATA%\pulse\probes.toml` (created on first run). Presets: TK421 (this machine) and generic (friends). First run copies TK421 if `C:\dev\cam` exists, otherwise generic.

## Release (Windows)

```powershell
$env:TAURI_SIGNING_PRIVATE_KEY = Get-Content -Raw src-tauri\pulse.key
npm run tauri build
.\scripts\make-portable.ps1 -Version 0.1.0
```

NSIS installer: `src-tauri\target\release\bundle\nsis\pulse_*_x64-setup.exe`  
Portable zip: `pulse-*-windows-x64.zip` (exe only; WebView2 is already on Windows 10/11)

Tag `v0.1.0` (or later) to have GitHub Actions publish a Release with the installer, `.sig`, `latest.json`, and portable zip.

Friends need a **public** download URL (GitHub keeps assets private when the repo is private). Either make this repo public, or copy the release assets onto a public repo. Auto-update reads `https://github.com/Randy-L-Thomas/pulse/releases/latest/download/latest.json`.

Updater signing: store `src-tauri\pulse.key` (gitignored) in GitHub Actions secret `TAURI_SIGNING_PRIVATE_KEY`. Optional: `TAURI_SIGNING_PRIVATE_KEY_PASSWORD`. If you lose the key, generate a new pair with `npx tauri signer generate` and update `plugins.updater.pubkey` in `src-tauri/tauri.conf.json`. First SmartScreen warning is expected (no Authenticode cert yet).
