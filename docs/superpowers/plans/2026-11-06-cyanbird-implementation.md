# CyanBird Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Build the first runnable Windows-first Tauri + React CyanBird migration workbench with bilingual UI, fixture-backed session normalization, provider adapters, and transactional migration scaffolding.

**Architecture:** React/TypeScript owns the workbench UI, state, and i18n. Tauri/Rust owns filesystem discovery, provider orchestration, snapshot manifests, and verification boundaries. Provider-specific parsing/export begins with fixture-backed adapters and a stable normalized session contract so real Windows paths can be added without changing the UI.

**Tech Stack:** Tauri 2, Rust, React 18, TypeScript, Vite, CSS modules or scoped CSS, Vitest, serde/serde_json, uuid.

**Spec:** `docs/superpowers/specs/2026-11-06-cyanbird-design.md`

## Global Constraints

- Windows-first desktop app; no cloud sync or remote paths in v1.
- UI supports `zh-CN` and `en-US` without restart.
- Five provider IDs: `claude-code`, `antigravity`, `codex`, `zcode`, `dsh`.
- Migration always creates a backup manifest before writing target data.
- Unknown provider fields stay in `metadata`; unsupported fields are visible in preview.
- Source data is never silently overwritten; default conflict policy is `new-id`.
- First release remains offline-capable and keeps UI lightweight.

---

### Task 1: Scaffold the desktop app shell

**Files:**
- Create: `package.json`
- Create: `vite.config.ts`
- Create: `index.html`
- Create: `src/main.tsx`
- Create: `src/app/App.tsx`
- Create: `src/app/app.css`
- Create: `src/types/domain.ts`
- Create: `src/vite-env.d.ts`
- Create: `src-tauri/Cargo.toml`
- Create: `src-tauri/src/main.rs`
- Create: `src-tauri/tauri.conf.json`
- Test: `src/app/App.test.tsx`

**Interfaces:**
- `ProviderId = 'claude-code' | 'antigravity' | 'codex' | 'zcode' | 'dsh'`.
- `NormalizedSession`, `SessionSummary`, `SourceLocation`, and `MigrationPlan` live in `src/types/domain.ts` and are imported by later UI tasks.
- Tauri app launches a Vite-rendered React shell with no network dependency.

- [ ] **Step 1: Create package and Tauri manifests**

Use React, Vite, TypeScript, Vitest, Testing Library, and Tauri 2 dependencies. Add scripts `dev`, `build`, `test`, and `tauri`.

- [ ] **Step 2: Write the app shell test**

Create a test that renders `<App />` and asserts the CyanBird wordmark, the `Migration` navigation item, and a language toggle are visible.

- [ ] **Step 3: Implement the minimal shell**

Create a three-column desktop layout with sidebar, source column, and migration plan column. Use CSS variables for cyan accent, neutral surfaces, borders, spacing, and typography.

- [ ] **Step 4: Run the shell test and production build**

Run `npm test -- --run` and `npm run build`. Expected: the shell test passes and Vite emits `dist/`.

- [ ] **Step 5: Commit**

`git add package.json vite.config.ts index.html src src-tauri && git commit -m "feat: scaffold CyanBird desktop shell"`

### Task 2: Add bilingual copy and workbench state

**Files:**
- Create: `src/i18n/messages.ts`
- Create: `src/i18n/I18nProvider.tsx`
- Create: `src/app/useWorkbenchState.ts`
- Modify: `src/app/App.tsx`
- Modify: `src/app/app.css`
- Test: `src/i18n/messages.test.ts`
- Test: `src/app/useWorkbenchState.test.ts`

**Interfaces:**
- `I18nProvider` exposes `locale`, `setLocale`, and `t(key)` through React context.
- `useWorkbenchState()` returns `selectedProvider`, `selectedSessionIds`, `targetProvider`, `conflictPolicy`, `createBackup`, and event handlers.
- Messages include both Chinese and English labels for sources, sessions, migration, backup, scan, preview, execute, settings, and failure states.

- [ ] **Step 1: Write failing tests for locale switching and selection state**

Assert `t('nav.migration')` returns Chinese in `zh-CN`, English after `setLocale('en-US')`, and selecting a session updates `selectedSessionIds` without mutating the source list.

- [ ] **Step 2: Implement the message catalog and provider**

Use a typed nested key map and a flat lookup helper. Persist locale to `localStorage` and default to `zh-CN`.

- [ ] **Step 3: Implement workbench state**

Use `useReducer` so all selection and migration-plan transitions are explicit and serializable.

- [ ] **Step 4: Wire state and translations into the shell**

Replace hard-coded UI labels, add the language toggle, source selection, session checkboxes, and target provider control.

- [ ] **Step 5: Run tests**

Run `npm test -- --run src/i18n/messages.test.ts src/app/useWorkbenchState.test.ts src/app/App.test.tsx`. Expected: PASS.

- [ ] **Step 6: Commit**

`git add src && git commit -m "feat: add bilingual workbench state"`

### Task 3: Implement normalized session fixtures and provider registry

**Files:**
- Create: `src/providers/providerRegistry.ts`
- Create: `src/providers/fixtures.ts`
- Create: `src/providers/normalize.ts`
- Create: `tests/fixtures/sessions.json`
- Modify: `src/types/domain.ts`
- Test: `src/providers/normalize.test.ts`
- Test: `src/providers/providerRegistry.test.ts`

**Interfaces:**
- `ProviderAdapter` exposes `id`, `displayName`, `discover()`, `listSessions()`, `parse()`, and `previewExport()`.
- `normalizeRawSession(raw, source)` returns `NormalizedSession` while copying unknown fields into `metadata`.
- `providerRegistry` contains exactly the five required provider IDs and returns stable summaries for fixture data.

- [ ] **Step 1: Create fixture data for one representative session per provider**

Each fixture includes messages, at least one tool call, reasoning, attachment metadata, workspace, and an unknown field.

- [ ] **Step 2: Write failing normalize and registry tests**

Assert every provider is registered, unknown fields survive under `metadata`, and a fixture round trip preserves message count, tool call count, and workspace.

- [ ] **Step 3: Implement the normalized domain types and normalizer**

Use discriminated unions for message/tool-call kinds and `Record<string, unknown>` for provider-specific metadata.

- [ ] **Step 4: Implement fixture-backed adapters**

The adapters expose deterministic `discover`, `listSessions`, `parse`, and `previewExport` behavior with `sourceKind: 'fixture'`, leaving real Windows paths behind the same interface.

- [ ] **Step 5: Run provider tests**

Run `npm test -- --run src/providers`. Expected: PASS for registry completeness and metadata preservation.

- [ ] **Step 6: Commit**

`git add src/providers src/types tests && git commit -m "feat: add normalized session provider registry"`

### Task 4: Build the migration preview and execution contract

**Files:**
- Create: `src/features/migration/migrationEngine.ts`
- Create: `src/features/migration/MigrationPanel.tsx`
- Create: `src/features/migration/migrationEngine.test.ts`
- Modify: `src/app/App.tsx`
- Modify: `src/app/app.css`

**Interfaces:**
- `buildMigrationPreview(plan, sessions, targetAdapter)` returns counts, unsupported fields, target path preview, and warnings.
- `executeMigration(plan, sessions, adapter, hooks)` emits `backup-created`, `exported`, `verified`, or `failed` events and returns a per-session report.
- `MigrationPanel` accepts `plan`, `sessions`, `preview`, and callbacks for target selection, conflict policy, backup toggle, and execute.

- [ ] **Step 1: Write failing engine tests**

Cover empty selection, `new-id` conflict policy, unsupported metadata warning, and one successful plus one failed session report.

- [ ] **Step 2: Implement preview generation**

Compute message/tool/reasoning counts, unsupported field labels, target path display, and warning severity without performing writes.

- [ ] **Step 3: Implement transactional execution as an injectable engine**

The engine calls `createBackup`, `writeTemp`, `verify`, and `commit` hooks in order. On failure it preserves the failure report and never calls commit for that session.

- [ ] **Step 4: Implement the right-side migration panel**

Add target cards, conflict select, backup toggle, summary stats, unsupported-field warnings, and execute button with progress states.

- [ ] **Step 5: Run tests and build**

Run `npm test -- --run src/features/migration/migrationEngine.test.ts src/app/App.test.tsx` and `npm run build`. Expected: PASS and production build success.

- [ ] **Step 6: Commit**

`git add src/features/migration src/app && git commit -m "feat: add migration preview and execution flow"`

### Task 5: Add Rust/Tauri command boundaries for Windows discovery and snapshots

**Files:**
- Create: `src-tauri/src/commands/mod.rs`
- Create: `src-tauri/src/commands/discovery.rs`
- Create: `src-tauri/src/commands/snapshots.rs`
- Create: `src-tauri/src/core/models.rs`
- Modify: `src-tauri/src/main.rs`
- Modify: `src-tauri/Cargo.toml`
- Test: `src-tauri/src/core/models.rs` unit tests

**Interfaces:**
- Tauri command `discover_sources()` returns JSON-safe `Vec<SourceLocation>`.
- Tauri command `create_snapshot(source_path, snapshot_root, manifest)` writes a timestamped manifest and returns `SnapshotReceipt`.
- Rust structs use `serde::{Serialize, Deserialize}` and match the TypeScript `SourceLocation` and manifest fields.

- [ ] **Step 1: Write Rust unit tests for path normalization and manifest serialization**

Cover Windows paths with spaces, Unicode, and stable JSON round-trip.

- [ ] **Step 2: Implement commands with read-only discovery first**

Check known environment variables and candidate directories, but return `unavailable` entries instead of failing the entire scan.

- [ ] **Step 3: Implement copy-first snapshot creation**

Create a sibling snapshot directory, copy manifest metadata, and return paths and byte counts. Do not delete or overwrite source files.

- [ ] **Step 4: Register commands in Tauri and add typed frontend wrappers**

Add `src/lib/tauri.ts` with `discoverSources()` and `createSnapshot()` wrappers that use fixture fallback when running in a browser-only dev session.

- [ ] **Step 5: Run Rust and frontend checks**

Run `cargo test --manifest-path src-tauri/Cargo.toml` and `npm test -- --run`. Expected: PASS.

- [ ] **Step 6: Commit**

`git add src-tauri src/lib && git commit -m "feat: add Windows discovery and snapshot commands"`

### Task 6: Finish visual polish, empty/error states, and release checks

**Files:**
- Modify: `src/app/app.css`
- Modify: `src/app/App.tsx`
- Modify: `src/i18n/messages.ts`
- Create: `README.md`
- Create: `docs/compatibility.md`
- Test: `src/app/App.test.tsx`

**Interfaces:**
- Empty, unavailable, partial-failure, and completed-migration states are visible and bilingual.
- README documents `npm install`, `npm run dev`, `npm test`, `npm run build`, and Tauri prerequisites.
- Compatibility doc records fixture-backed support versus real-path support per provider.

- [ ] **Step 1: Add failing UI assertions for empty and partial-failure states**

Assert the workbench shows a clear empty state before scan and a per-session failure badge after a partial migration.

- [ ] **Step 2: Implement states and responsive desktop polish**

Keep the primary 3-column desktop layout, add a compact stacked layout below 1100px, visible focus states, reduced-motion support, and high-contrast status colors.

- [ ] **Step 3: Add documentation and compatibility matrix**

Document the current fixture-backed adapters and the Windows discovery boundary so unsupported real formats are explicit.

- [ ] **Step 4: Run full verification**

Run `npm test -- --run`, `npm run build`, and `cargo test --manifest-path src-tauri/Cargo.toml`. Expected: all PASS.

- [ ] **Step 5: Commit**

`git add . && git commit -m "feat: complete CyanBird migration workbench v1"`

## Self-review

- Spec coverage: goals and success criteria are covered by Tasks 1–6; provider isolation is Task 3; transaction/error rules are Task 4 and Task 5; bilingual UI is Task 2 and Task 6; testing and release documentation are Task 6.
- Placeholder scan: no TODO/TBD steps; real file paths, commands, interfaces, and test expectations are listed.
- Type consistency: `ProviderId`, `NormalizedSession`, `MigrationPlan`, `SourceLocation`, and `ProviderAdapter` are defined before consumers; Rust command payloads map to the same field names.
