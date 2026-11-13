# Antigravity Desktop history import

As of 0.2.3, Antigravity imports only user messages and assistant text, including
normal code examples and intermediate replies. Tool calls/results and reasoning
are retained in mandatory source backups and are not inserted as chat messages.
The preview count and acknowledgment use this same policy. Other migration
targets retain the existing historical-text archive policy.

Validated target: Windows Antigravity Desktop **2.29.0**, bundled language server
**1.11.0**, SHA-256 `c98e7c331da5bcc37002a550e7d625b4c636f775fa2804f73777c6a500e1c3d5`.
This does not establish compatibility with every Antigravity IDE or CLI release.

Fully quit Antigravity before preparing/executing an import. The application must
have been started once to initialize `~/.gemini/antigravity/conversation_summaries.db`.
Select Antigravity as the destination, choose a project folder, prepare the preview,
acknowledge the conversion differences and execute. Reopen Antigravity afterwards.

The writer creates a new conversation UUID, native SQLite trajectory, transcript
mirror and title annotation. It registers the project folder if no existing project
matches. Existing project settings and permissions are not copied or replaced.
The SQLite picker and `agyhub_summaries_proto.pb` trajectory lookup are both updated.
Both existing indexes are backed up before import, in addition to the source snapshot.
An index transaction and exclusive protobuf-file handle protect installation;
collisions fail without overwriting existing conversations. Failed installation
removes its own files/project registration and truncates its protobuf addition.

Historical user/assistant roles and text are preserved. Tools/results/reasoning stay
in the source backup; binary attachments and old runtime state are not recreated. Native read validation does not establish model/provider execution.

Important desktop format details: `PRAGMA user_version=1` prevents the client's
version-zero migration from reinitializing trajectory IDs and metadata. A summary
must reference the exact generated trajectory ID. Timestamps, project metadata and
title annotations survive client restarts. Project registration lives in
`~/.gemini/config/projects/<id>.json`.

The minimal wire mapping was informed by the MIT-licensed
[session-migrate adapter](https://github.com/xhluca/session-migrate/blob/45351ff95ce79684c23377920b92026ecbe9759b/src/session_migrate/formats/antigravity.py).
Its original validation covers a different Linux CLI version; desktop behavior was
checked independently against the local binary. The retained license is
[session-migrate-LICENSE.txt](session-migrate-LICENSE.txt).

Run the credential-free native check (Python standard library and Rust toolchain):

```powershell
python scripts/verify-antigravity-native.py --binary "$env:LOCALAPPDATA/Programs/antigravity/resources/bin/language_server.exe"
```

It initializes a temporary native store, runs CyanBird's writer, then restarts the
actual language server twice. Assertions cover both message markers, title,
trajectory identity, native project registration and a Chinese workspace path.
No real account data or conversations are copied.

## Codex pagination and multi-message turns (0.2.1)

Codex Desktop rollout windows share a thread ID. CyanBird snapshots every window
and reconstructs history at each `history_base.end_ordinal_exclusive` boundary,
excluding stale tails from the old window. Missing prefix history stops migration
instead of reporting partial success. Completed-message events supplement response
items without duplicating their mirrored messages. Tool records are historical
assistant text, not artificial user turns; encrypted reasoning payloads are not text.

Adjacent assistant/progress/tool records are combined into one native planner response
per user turn so intermediate messages remain visible alongside the final answer.
The native restart test checks first, middle and final markers in that response.
Already imported sessions need to be imported again with 0.2.1; this update does not
silently overwrite existing native conversations.

## Literal tool records (0.2.2)

Historical Codex tool calls and outputs are stored inside fenced JSON code blocks.
The fence is longer than every backtick run in the payload, preserving PowerShell
variables, dollar signs, nested Markdown and raw tool metadata as literal text.
This prevents the destination Markdown renderer from interpreting commands as math.

## User message items and Codex prompt filtering (0.9.0-beta)

Antigravity Desktop renders user turns from `UserInputStep.items` (protobuf field `3`,
containing `TextOrScopeItem.text` field `1`) inside `step_payload` field `19`, while RPC
endpoints also expose `userResponse` (protobuf field `2`). CyanBird now populates **both**
field `2` and field `3` for every imported user turn, verified across restarts via
`scripts/verify-antigravity-native.py` (`steps[0]["userInput"]["items"]`).

Codex parsing also filters injected metadata blocks (`<environment_context>`,
`<INSTRUCTIONS>`, etc.) per content block inside `response_item` user messages rather than
discarding mixed blocks, and falls back to `event_msg` `user_message` events with
deduplication when `response_item` user text is absent. Conversations imported into
Antigravity prior to `0.9.0-beta` should be re-imported (from live source or the Backup
Center) while Antigravity is fully quit so user turns appear cleanly.
