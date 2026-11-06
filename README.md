CyanBird
========

Windows-first local desktop workbench for moving and backing up sessions across Claude Code, Antigravity, Codex, Zcode and DeepSeek Harness.

## Development

```powershell
npm install
npm run dev
npm test -- --run
npm run build
```

The current environment has Node/npm but does not include Rust or the Tauri CLI, so the React workbench runs in browser/Vite mode. The `src-tauri/` boundary is scaffolded for the desktop build once Rust is installed.
