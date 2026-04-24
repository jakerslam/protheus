# Codex Assimilation Wave Runbook

## Purpose

Scale Codex assimilation throughput without losing ledger trust.

## Related Docs

- Manual assimilation system template: `docs/workspace/ASSIMILATION_MANUAL_TEMPLATE.md`
  - Use this when planning a new target assimilation (map, ledger, active queue).
  - This runbook is the wave-execution protocol once the active queue is prepared.

## Wave Contract

- Wave size: **4-8** rows.
- Rows must be **disjoint file paths**.
- One integration checkpoint per wave.
- Source file burn-down is mandatory per wave:
  - every assimilated source file must be recorded in `[Target-Name]-Assimilation/source-burn-down.tsv`
  - status must be advanced to `burned_down` in the same wave that assimilates it
  - if a physical archive move is used, record `deleted_to=target-repo/.assimilation_deleted/<path>`

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
5. Update source burn-down tracking for each assimilated source file in:
   - `[Target-Name]-Assimilation/source-burn-down.tsv`

## Completion

1. Update ledger rows to `done` in:
   - `local/workspace/reports/CODEX_FILE_LEDGER_2026-04-08.full.json`
   - `local/workspace/reports/CODEX_FILE_LEDGER_2026-04-08.full.tsv`
   - summary progress in `local/workspace/reports/CODEX_FILE_LEDGER_2026-04-08.md`
2. Confirm source-file burn-down closure for the wave:
   - each assimilated source file appears in `source-burn-down.tsv` with `status=burned_down`
   - archive move path is recorded when using physical burn-down
3. Commit wave code + ledger + burn-down updates together.
4. Push.
5. Confirm clean post-wave hygiene:
   - `git status --short`
   - `npm run -s ops:churn:guard`
