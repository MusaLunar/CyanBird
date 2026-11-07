# Current capabilities

- Native main-session discovery for Claude Code, Antigravity, Codex, Zcode and DSH.
- Antigravity: unique conversation IDs, summary database title/workspace, annotation title fallback, decoded file URIs and dominant Cwd fallback. Child conversations with an explicit parent are hidden from the main-session list.
- Codex: subagent markers filter spawned threads while retaining user forks. Injected AGENTS/environment messages do not become fallback titles.
- Backup: selected source transcripts plus manifest/SHA-256 records; Antigravity annotation sidecars included when available. Zcode takes a consistent SQLite snapshot of the **entire database**, including WAL data. Backup centers list completed manifests; incomplete attempts remain in .partial directories and are not listed as successful.
- Markdown export: text, reasoning and available tool-call/result data. Each selected session gets a separate .md file. Zcode reads message/part tables. Malformed transcripts fail with an error rather than silently dropping lines. Referenced binary attachments are described, not embedded or downloaded.
- Backup center and settings routes are implemented. Settings persist output directories; theme and locale persist locally. Changing directories does not move previous artifacts.
- History migration to Claude Code, Codex, DSH V4 and Zcode is implemented with mandatory backups and preview confirmation. Antigravity is source-only. Tools/reasoning are historical text and binary attachments are not copied. Automatic backup restoration remains unimplemented. See [migration.md](migration.md).
- Large transcript scans still decode full logs; incremental caching and broader schema compatibility remain future work.

Icon provenance and licenses: [agent-icons.md](agent-icons.md).
