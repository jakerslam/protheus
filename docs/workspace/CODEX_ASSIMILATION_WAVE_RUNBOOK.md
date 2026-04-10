# Codex Assimilation Wave Runbook

## Purpose

Scale Codex assimilation throughput without losing ledger trust.

## Wave Contract

- Wave size: **4-8** rows.
- Rows must be **disjoint file paths**.
- One integration checkpoint per wave.

## Strict Preflight (Required)

1. Workspace is clean:
   - `git status --short`
2. Churn is clean:
   - `npm run -s ops:churn:guard`
3. Wave rows are valid:
   - exist in `local/workspace/reports/CODEX_FILE_LEDGER_2026-04-08.full.json`
   - each row is `queued`
   - row count is between 4 and 8
   - file paths are unique
4. Run the codex wave preflight guard:
   - `npm run -s ops:codex:wave:preflight -- --orders=<comma-separated-or-ranges>`

## Execution

1. Implement the selected rows only.
2. Keep edits disjoint to avoid merge churn.
3. Run targeted tests for touched files/surfaces.
4. Run one integration checkpoint across the wave surface.

## Completion

1. Update ledger rows to `done` in:
   - `local/workspace/reports/CODEX_FILE_LEDGER_2026-04-08.full.json`
   - `local/workspace/reports/CODEX_FILE_LEDGER_2026-04-08.full.tsv`
   - summary progress in `local/workspace/reports/CODEX_FILE_LEDGER_2026-04-08.md`
2. Commit wave code and ledger updates together.
3. Push.
4. Confirm clean post-wave hygiene:
   - `git status --short`
   - `npm run -s ops:churn:guard`
