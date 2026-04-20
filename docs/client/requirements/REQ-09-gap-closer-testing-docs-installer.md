# REQ-09: Testing + Documentation + Curl Installer Gap Closer

Version: 1.0  
Date: 2026-03-06

## Objective

Close the highest-visibility completeness gaps with a focused implementation wave:

- Production-ready one-line installer (`curl ... | sh` + PowerShell path)
- Test and coverage uplift with visible badge + CI gate
- Architecture/onboarding documentation refresh for fast operator adoption

## Requirements

1. `REQ-09-001` Installer parity
- Acceptance:
  - Root [install.sh](../../install.sh) exists and provisions canonical wrappers `infring`, `infringctl`, and `infringd`.
  - Legacy aliases (`protheus`, `protheusctl`, `protheusd`) remain compatibility-only.
  - Root [install.ps1](../../install.ps1) exists with equivalent Windows behavior.
  - [docs/client/GETTING_STARTED.md](../GETTING_STARTED.md) includes one-line install commands.

2. `REQ-09-002` Coverage pipeline and badge
- Acceptance:
  - Vitest coverage and Rust `cargo llvm-cov` are wired in scripts/CI.
  - Combined coverage artifact is generated with a coverage badge.
  - README displays a coverage badge.
  - CI coverage gate enforces `combined_lines_pct >= 75`.

3. `REQ-09-003` Architecture and onboarding polish
- Acceptance:
  - Root [ARCHITECTURE.md](../../ARCHITECTURE.md) includes a Mermaid system map with conduit and 7 primitives.
  - [README.md](../../README.md) provides an install-first quickstart.
  - [docs/client/GETTING_STARTED.md](../GETTING_STARTED.md) provides a <2 minute path.

4. `REQ-09-004` Optional Python packaging path must remain thin and Rust-authoritative
- Acceptance:
  - A dedicated Python package exists under `packages/infring-py` (or compatibility-equivalent path).
  - `pip install` exposes an `infring` CLI entrypoint that delegates to `infring-ops`.
  - No kernel logic is re-implemented in Python; wrapper only forwards command execution.

## Execution Notes (Current Batch)

Implemented in this batch:
- Added root installers (`install.sh`, `install.ps1`) with release-binary provisioning and CLI wrappers.
- Added vitest + llvm-cov coverage scripts, CI workflow, and combined coverage badge generation.
- Added/updated docs: `ARCHITECTURE.md`, `README.md`, and `docs/client/GETTING_STARTED.md`.

## First-Run Failure Decision Tree (Operator Contract)

When first-run fails, diagnose in this order:

1. Command resolution (`infring` not found):
   - Reload path: `. "$HOME/.infring/env.sh" && hash -r 2>/dev/null || true`
   - Verify: `infring --help`
   - Direct-path fallback: `"$HOME/.infring/bin/infring" --help`

2. Runtime/setup not completed:
   - Run: `infring setup --yes --defaults`
   - Verify status: `infring setup status --json`

3. Gateway/dashboard down:
   - `infring gateway status`
   - `infring gateway restart`
   - Health endpoint: `http://127.0.0.1:4173/healthz`

4. Stale root/path drift:
   - `infringctl doctor --json`
   - Verify `INFRING_WORKSPACE_ROOT` / `PROTHEUS_WORKSPACE_ROOT` consistency with active workspace

5. Full-surface Node dependency missing:
   - Reinstall with Node bootstrap: `curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --full --install-node`
   - Or use constrained mode (`--pure` / `--tiny-max`) if full surface is not required.

### Deterministic Failure-Code Matrix (First-Run)

| failure_code | primary_command | expected_output | deterministic_next_action |
| --- | --- | --- | --- |
| `command_not_found` | `infring --help` (after env reload) | Help output or explicit missing-wrapper failure | Run direct wrapper (`$HOME/.infring/bin/infring --help`) and rerun install if still missing |
| `setup_incomplete` | `infring setup status --json` | `onboarding_receipt.status` is `incomplete` with mode/workspace metadata | `infring setup --yes --defaults`, then re-check status |
| `gateway_unhealthy` | `infring gateway status` + `/healthz` | Deterministic gateway state and health endpoint response | `infring gateway restart`, then verify `/healthz` and doctor output |
| `stale_workspace_root` | `infringctl doctor --json` | Stale path/root findings with explicit fields | Align `INFRING_WORKSPACE_ROOT`/`PROTHEUS_WORKSPACE_ROOT`, re-run doctor |
| `full_surface_dependency_missing` | Full install command with `--install-node` | Runtime wrappers + full-surface dependencies installed | Retry first-run flow or switch to `--pure` / `--tiny-max` |
