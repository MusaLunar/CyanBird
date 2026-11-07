# CyanBird

Windows-first Tauri + React workbench for discovering, reviewing, migrating and backing up AI agent sessions.

## Development

```powershell
npm ci
npm run dev
npm test -- --run
npm run build
npm run tauri dev
npm run tauri build
```

Desktop builds require Rust with the Windows MSVC toolchain, Visual C++ build tools and WebView2. Native discovery runs on a blocking worker rather than the UI thread. Browser-only mode cannot access native session directories and displays the native-command error rather than inventing sessions.

## Current scope

Native read-only discovery is connected for Claude Code, Antigravity, Codex, Zcode and DSH. Migration to Claude Code, Codex, DSH V4 and Zcode supports historical text with mandatory backups and per-session results. Antigravity is currently source-only. See [migration details](docs/migration.md) for fidelity and compatibility limits. See [compatibility](docs/compatibility.md) and [reference analysis](docs/dsh-chat-import-analysis.md).

Update checks query the latest public GitHub Release from MusaLunar/CyanBird and link to release notes; automatic update installation is not implemented.
