# Current capabilities

- Native main-session discovery for Claude Code, Antigravity, Codex, Zcode and DSH.
- Antigravity: unique conversation IDs, summary database title/workspace, annotation title fallback, decoded file URIs and dominant Cwd fallback. Child conversations with an explicit parent are hidden from the main-session list.
- Codex: subagent markers filter spawned threads while retaining user forks. Injected AGENTS/environment messages do not become fallback titles.
- Backup: selected source transcripts plus manifest/SHA-256 records; Antigravity annotation sidecars included when available. Zcode takes a consistent SQLite snapshot of the **entire database**, including WAL data. Backup centers list completed manifests; incomplete attempts remain in .partial directories and are not listed as successful.
- Markdown export: text, reasoning and available tool-call/result data. Each selected session gets a separate .md file. Zcode reads message/part tables. Malformed transcripts fail with an error rather than silently dropping lines. Referenced binary attachments are described, not embedded or downloaded.
- Backup center and settings routes are implemented. Settings persist output directories; theme and locale persist locally. Changing directories does not move previous artifacts.
- History migration to Claude Code, Codex, DSH V4, Zcode and Antigravity Desktop 2.29 is implemented with mandatory backups and preview confirmation. Antigravity must be fully closed before import; its native SQLite/protobuf indexes and project registration are written together. Tools/reasoning are historical text and binary attachments are not copied. Backup history can be imported into any supported target as new sessions, after checksum verification. Exact in-place database rollback is a separate operation. See [migration.md](migration.md).
- Large transcript scans still decode full logs; incremental caching and broader schema compatibility remain future work.

Release history & latest notes: [CHANGELOG.md](../CHANGELOG.md) · [v0.9.0-beta release notes](release-notes-v0.9.0-beta.md).

Icon provenance and licenses: [agent-icons.md](agent-icons.md).
