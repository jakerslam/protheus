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
- `pending` rows: 0
- Active files left outside `.assimilation_deleted`: 0

## Imported Capability
- `RTK-TOML-MATCH-001` (from `src/core/toml_filter.rs`):
  - Imported RTK-style `match_output` short-circuit filtering pattern into Rust-core placeholder guards.
  - New module: `core/layer0/ops/src/tool_output_match_filter.rs`
  - Wired into response finalization and ack detection:
    - `core/layer0/ops/src/dashboard_compat_api_parts/030-set-config-payload.rs`
  - Adds deterministic rewrite for generic tool-failure placeholder:
    - `I couldn't complete <tool> right now.` → actionable retry/doctor guidance.
  - Adds DuckDuckGo findings-placeholder suppression for ack-only no-findings copy.
- `RTK-DISCOVER-001` (from `src/discover/{lexer.rs,registry.rs,rules.rs,report.rs}`):
  - Imported RTK-style command discovery/classification lane for deterministic shell telemetry triage.
  - New kernel module:
    - `core/layer0/ops/src/session_command_discovery_kernel.rs`
  - Wired command surface:
    - `core/layer0/ops/src/main.rs.inc`
    - `core/layer0/ops/src/lib.rs.inc`
    - `core/layer0/ops/src/ops_main_usage.rs`
  - Capability:
    - Quote-aware command-chain splitting.
    - Env/global-option/path normalization before classification.
    - Supported/unsupported report with estimated token-savings summary and deterministic receipts.
- `RTK-SESSION-ANALYTICS-001` (from `src/discover/provider.rs`, `src/discover/mod.rs`, `src/analytics/session_cmd.rs`):
  - Imported provider-style transcript extraction (`tool_use`/`tool_result`) and session adoption analytics.
  - New kernel module:
    - `core/layer0/ops/src/session_command_session_analytics_kernel.rs`
  - Command surface:
    - `protheus-ops session-command-session-analytics-kernel <extract-jsonl|classify-jsonl|adoption-report>`
- `RTK-TRACKING-001` (from `src/core/tracking.rs`):
  - Imported SQLite-backed command telemetry persistence and aggregate summary surfaces.
  - New kernel module:
    - `core/layer0/ops/src/session_command_tracking_kernel.rs`
  - Command surface:
    - `protheus-ops session-command-tracking-kernel <record|summary|status>`
- `RTK-PERMISSIONS-001` (from `src/hooks/permissions.rs`):
  - Imported deny/ask wildcard permission profile evaluator with compound-command support.
  - New kernel module:
    - `core/layer0/ops/src/command_permission_kernel.rs`
  - Command surface:
    - `protheus-ops command-permission-kernel <evaluate|match-pattern|extract-pattern>`
- `RTK-FILTER-001` (from `src/core/filter.rs`):
  - Imported language-aware filter-level compaction primitive for source text.
  - New kernel module:
    - `core/layer0/ops/src/source_comment_filter_kernel.rs`
  - Command surface:
    - `protheus-ops source-comment-filter-kernel <filter|detect-language>`

## Captured Candidates (Not Yet Imported)
- None (all `reviewed_candidate` RTK rows were assimilated in this pass).
