# Provider compatibility

| Provider | Native discovery | Remaining limitations |
|---|---|---|
| Claude Code | ~/.claude/projects; primary-session ID validation; custom title before first prompt | Claude-3p metadata root not yet adapted |
| Antigravity | transcript.jsonl in Windows .gemini/antigravity family roots | annotations titles and workspace reconstruction pending |
| Codex | sessions and archived_sessions; plain and .zst logs; session_meta ID/cwd; session_index titles | injected prompt filtering and page grouping need more fixtures |
| Zcode | read-only .zcode/cli/db/db.sqlite; parentless session rows, current WAL visible to SQLite | schema variation and message/tool statistics pending |
| DSH | DSH_HOME or ~/.dsh; .jsonl and .zstd logs | desktop additional roots and multi-generation deduplication pending |

ChatGPT website conversations.json export is a separate format from Codex and is not yet wired to a file-import UI.

Discovery returns source summaries only. These are not full normalized transcripts: actual target-provider export and resumability verification remain incomplete. Lists are never populated from example fixtures after a native error or an empty result.

Large logs currently require full decoding during discovery; bounded reads and scan caching remain to be implemented. A few malformed records are skipped by the current parser; detailed per-file warnings are still pending.
