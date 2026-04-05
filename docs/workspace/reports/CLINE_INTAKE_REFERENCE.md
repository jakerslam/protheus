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
