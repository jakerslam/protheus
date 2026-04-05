# RTK Intake Reference (Isolated Import Lane)

## Source
- Repo: `https://github.com/rtk-ai/rtk`
- Isolated clone: `local/workspace/vendor/rtk`
- Checked revision: `45d8dca` (short SHA)

## Master Ledger
- File-by-file status ledger: `docs/workspace/reports/RTK_FILE_STATUS.tsv`
- Format: `status<TAB>path<TAB>notes`
- Scope excludes `.git/**`, `node_modules/**`, Python virtualenvs, and Rust `target/**` build artifacts.

## Status Vocabulary
- `pending`: not yet reviewed.
- `reviewed_no_import`: reviewed, no net-new capability worth porting.
- `reviewed_candidate`: reviewed, candidate capability identified.
- `imported`: capability ported into runtime.
- `skipped_non_runtime`: assets/docs/legal/non-runtime file skipped for capability ingestion.

## Checkoff Deletion Rule
- After a row is checked off (`reviewed_*`, `imported`, or `skipped_non_runtime`), the source file is moved from active intake tree to:
  - `local/workspace/vendor/rtk/.assimilation_deleted/<path>`
- Ledger notes include `deleted_to=.assimilation_deleted`.

## Current Intake State
- `pending` rows: 166
- Active files left outside `.assimilation_deleted`: 166

## Imported Capability
- `RTK-TOML-MATCH-001` (from `src/core/toml_filter.rs`):
  - Imported RTK-style `match_output` short-circuit filtering pattern into Rust-core placeholder guards.
  - New module: `core/layer0/ops/src/tool_output_match_filter.rs`
  - Wired into response finalization and ack detection:
    - `core/layer0/ops/src/dashboard_compat_api_parts/030-set-config-payload.rs`
  - Adds deterministic rewrite for generic tool-failure placeholder:
    - `I couldn't complete <tool> right now.` → actionable retry/doctor guidance.
  - Adds DuckDuckGo findings-placeholder suppression for ack-only no-findings copy.

## Captured Candidates (Not Yet Imported)
- `RTK-CANDIDATE-DISCOVER-001` (from `src/discover/mod.rs`):
  - Session command discovery/aggregation report lane for future diagnostics.
- `RTK-CANDIDATE-CODE-FILTER-001` (from `src/core/filter.rs`):
  - Language-aware comment/boilerplate filter strategy as future read/summarize import.
