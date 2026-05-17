# Codex Limit Monitor

Windows-first desktop monitor for Codex usage and ChatGPT Codex rate-limit status.

## What It Does

- Reads live Codex limit percentages from `codex app-server` using `account/rateLimits/read`.
- Falls back to read-only local usage summaries from `%USERPROFILE%\.codex\state_5.sqlite` and `logs_2.sqlite`.
- Shows status in a Tauri dashboard, system tray menu, native notifications, and an always-on-top badge.

## Development

The current PowerShell session has Rust/Cargo available, but plain `node`, `npm`, and `codex` are not on PATH. If needed, use the configured Node/NPM path from Herd/NVM or set the Codex executable path inside the app settings.

```powershell
npm install
npm run tauri dev
```

If `codex` is not on PATH, set the Codex executable in Settings. The app also tries common Windows candidates such as `%APPDATA%\npm\codex.cmd`.

## Verification

```powershell
cargo test --manifest-path src-tauri/Cargo.toml
npm run build
```
