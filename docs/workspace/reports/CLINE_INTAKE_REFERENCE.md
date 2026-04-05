# Cline Intake Reference (Isolated Import Lane)

## Source
- Repo: `https://github.com/cline/cline`
- Isolated clone: `local/workspace/vendor/cline`
- Checked revision: `e52a052c8` (short SHA)

## Master Ledger
- File-by-file status ledger: `docs/workspace/reports/CLINE_FILE_STATUS.tsv`
- Format: `status<TAB>path<TAB>notes`
- Scope excludes `.git/**` and `node_modules/**`.

## Status Vocabulary
- `pending`: not yet reviewed.
- `reviewed_no_import`: reviewed, no net-new capability worth porting.
- `reviewed_candidate`: reviewed, candidate capability identified.
- `imported`: capability ported into runtime.
- `skipped_non_runtime`: assets/docs/legal/non-runtime file skipped for capability ingestion.

## Checkoff Deletion Rule
- After a row is checked off (`reviewed_*` or `imported`), the source file is removed from the active intake tree and moved to:
  - `local/workspace/vendor/cline/.assimilation_deleted/<path>`
- Ledger notes include `deleted_to=.assimilation_deleted`.

## Current Imported Capability
- `CLINE-FILE-SEARCH-001` (from `src/services/search/file-search.ts`):
  - Added Rust-core `workspace-file-search` domain with:
    - ripgrep-backed file/folder discovery
    - fuzzy ranking (tight-match preference)
    - multi-root input support
    - workspace boundary gate (`workspace_outside_root` unless explicitly allowed)
- `CLINE-TERMINAL-TRUNCATION-001` (from `cli/src/acp/AcpTerminalManager.ts`):
  - Imported head+tail terminal output truncation semantics into core terminal broker:
    - explicit truncation marker (`... (output truncated) ...`)
    - preserves both early and recent output context (instead of tail-only clipping)
    - UTF-8 boundary-safe slicing and byte-budget enforcement

## Captured Candidates (Not Yet Imported)
- `CLINE-CANDIDATE-SESSION-GUARD-001` (from `cli/src/agent/ClineAgent.ts`):
  - Per-session "already processing" prompt gate to prevent overlapping request races.
- `CLINE-CANDIDATE-STREAM-DEDUPE-001` (from `cli/src/agent/ClineAgent.ts` + `messageTranslator.ts`):
  - Stable mapping between streaming message timestamps and tool-call IDs to avoid duplicate tool events.
- `CLINE-CANDIDATE-PERMISSION-AUTOALLOW-001` (from `cli/src/agent/permissionHandler.ts`):
  - Scoped auto-approval tracker for repeated command/tool/server approvals.
