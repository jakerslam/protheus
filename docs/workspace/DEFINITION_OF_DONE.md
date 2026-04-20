# Definition Of Done (DoD)

## Purpose

Prevent false completion claims and keep execution ledgers truthful.

## Status Contract

- `queued`: scoped, not started.
- `in_progress`: actively being implemented.
- `blocked`: cannot proceed due to an explicit blocker.
- `done`: newly implemented in this repo revision with evidence.
- `existing-coverage-validated`: verified as already implemented before this revision; not a new implementation claim.

## Hard DoD Requirements For `done`

An item may be marked `done` only if all checks are true:

1. Authority change exists in the correct layer (core for canonical truth; `surface/orchestration/**` for non-canonical control-plane coordination such as decomposition/coordination/sequencing/recovery/packaging; shell path `client/**` only thin UX/wrapper).
2. Evidence points to non-backlog files (code/tests/scripts/artifacts), not only TODO/SRS text.
3. Evidence paths resolve:
   - concrete file path exists, or
   - glob evidence matches at least one file.
4. Validation exists and passes (`verify.sh`, lane test, or targeted regression command).
5. No conflict with unchecked TODO state for the same ID.
6. Repository churn is reconciled for the touched scope: no unresolved delete+untracked move pairs (`npm run -s ops:churn:guard`).
7. Touched source files comply with canonical caps in `docs/workspace/repo_file_size_policy.json` (as mirrored in `docs/workspace/codex_enforcer.md`), or include a valid time-bounded exception per policy.
8. Touched source files comply with the language allowlist in `docs/workspace/codex_enforcer.md` (no authored JavaScript).
9. Any new authority introduced by the change is implemented in `core/**` (shell path `client/**` stays thin wrapper/UX only).
10. Any net-new functionality in the revision has a matching SRS row/update in `docs/workspace/SRS.md` with acceptance criteria and regression evidence pointers.
11. For Codex ledger assimilation updates, rows are completed in a 4-8 file disjoint wave with strict preflight (clean status + churn guard + queued-row verification) and targeted tests logged before `done` mutation.

## Prohibited

- Marking regression-only confirmations as `done`.
- Treating `existing-coverage-validated` as code implementation.
- Claiming completion without non-backlog evidence.

## CI Enforcement

The following gates enforce this policy:

- `ops:srs:full:regression` (done/evidence/status consistency across SRS rows).
- `ops:dod:gate` (ROI execution ledger truthfulness and evidence existence).
- `ops:v8:runtime-proof:gate` (for any `done` `V8-*` row, requires Rust runtime proof execution via `core/layer0/ops/tests/v8_runtime_proof.rs`).
- `verify.sh` runs `ops:dod:gate` as a required step.

## Operational Rule

If evidence is missing, downgrade status immediately (`done -> in_progress` or `existing-coverage-validated`) and patch the ledger before continuing.
