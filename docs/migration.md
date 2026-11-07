# Migration support (0.2.0)

Native window decorations are disabled in Tauri. The built-in titlebar supports dragging, minimizing, maximizing/restoring and closing.

| Source | Claude Code target | Codex target | DSH V4 target | Zcode target | Antigravity target |
|---|---|---|---|---|---|
| Claude Code / Codex / DSH / Zcode / Antigravity | supported | supported | supported | existing database required | not supported |

Migration preserves conversational roles and historical text. Tool calls/results and reasoning are archived as text rather than executable tool state. Binary attachments are not copied. Original source files are retained in a mandatory backup. This is a history migration, not an exact recreation of the old agent runtime, permissions, model or context compaction state.

## Flow

1. Select source sessions and a destination Agent.
2. Choose an existing absolute project directory.
3. Generate a migration preview and review the destination and conversion differences.
4. Acknowledge those differences, then execute.
5. Review each session's written/failed result. Open or resume the new session in the destination Agent.

A changed target, project directory or session selection invalidates the preview and acknowledgment. New session IDs are always created. JSONL outputs are published without overwriting existing files. Zcode inserts are transactional and the whole target database is backed up first. The source snapshot, not the live source log, supplies the conversion input. `migration-report.json` is saved next to the backup manifest.

## Verification

- Rust unit tests cover native envelopes, message roles, JSONL non-overwrite behavior, Zcode transaction commit/rollback and backup integrity.
- Claude/Codex fixture outputs round-tripped through dsh-chat-import parsers.
- DSH V4 fixture events replayed successfully using the locally installed official dsh-session Session.create API.
- Frontend test verifies preview/acknowledgment gating and invalidation after directory changes.

These checks validate structure, persistence and DSH replay. They do not prove provider model execution or full UI compatibility across every vendor version. Zcode targets require an initialized database matching the supported session/message/part schema; schema errors fail without committing the new session. Antigravity remains source-only until its native conversation/protobuf writer is validated.
