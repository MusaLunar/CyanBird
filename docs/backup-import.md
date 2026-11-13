# Importing backed-up conversations (0.9.0-beta)

The two-pane Backup Center separates **All**, **Session backups**, and **Index
snapshots** tabs with instant filtering by title, backup ID, or provider name. Selecting a
backup card shows its dossier on the right (directory, included sessions, message counts,
and collapsible file `SHA-256` checksums). Choose **Import into agent**, select
conversations, a target Agent and an existing project directory, then preview and
confirm. The same Agent may be selected as both the original source and destination.

Import reads stored snapshots rather than discovering live originals, so deleting
the original conversation does not prevent recovery. All stored file sizes and
SHA-256 hashes are checked, including older Codex rollout windows. Missing files,
invalid paths or failed checksums stop the import. The source backup is preserved.

The importer uses the same native writers as session migration and generates fresh
session IDs. Existing conversations are not overwritten. Antigravity retains its
dialogue-only policy and must be fully closed before import; Zcode and Antigravity
require initialized native stores. Target database/index snapshots are labelled
separately and are not offered as conversation backups.

Import receipts are saved in `import-reports/` under the configured backup directory.
Old manifests without a `kind` field remain readable. This is conversation-history
recovery, not replacement of an Agent's complete database/runtime state.

External CyanBird icons use an alpha-transparent black portrait with cyan accents.
The white portrait asset used at the upper left of the application is unchanged.
