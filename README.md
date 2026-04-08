# InfRing

[![License: Dual](https://img.shields.io/badge/license-dual%20(NC%20%2B%20Apache--2.0)-orange.svg)](LICENSE_SCOPE.md)
[![Architecture](https://img.shields.io/badge/architecture-three--plane%20metakernel-0A7A5E)](planes/README.md)
![Coverage](docs/client/badges/coverage.svg)

InfRing is a deterministic, receipt-first autonomous runtime built on a three-plane metakernel.  
It is designed for verifiable execution, fail-closed safety, and reproducible operator workflows.

Core authority is Rust-first (`core/**`). Client/runtime surfaces are thin wrappers around policy-governed core lanes.

## Why InfRing

- Deterministic execution with evidence-backed receipts.
- Fail-closed safety and policy enforcement by default.
- Rust-authoritative core with explicit thin-client boundaries.
- Multi-profile runtime strategy: rich, pure, and tiny-max.
- Operator-first CLI and gateway control surface.

## Architecture At A Glance

| Plane | Role |
|---|---|
| Safety Plane | Deterministic guardrails, invariants, fail-closed behavior |
| Cognition Plane | Agent orchestration, scheduling, adaptive workflows |
| Substrate Plane | Runtime integration, execution surfaces, system bridges |

See [planes/README.md](planes/README.md) for the canonical architecture contract.

## Current State (March 2026)

What is true in this repository today:

- Primary operator entrypoint is `infring` (with `infringctl` and `infringd` wrappers).
- Main dashboard is served by the gateway at `http://127.0.0.1:4173/dashboard#chat`.
- Gateway health endpoint is `http://127.0.0.1:4173/healthz`.
- Gateway persistence is enabled by default (auto-restart + reboot supervision unless disabled).
- Pure profiles (`--pure`, `--tiny-max`) are Rust-only and intentionally do not expose the rich `gateway` UI surface.
- Full command surface still requires Node.js 22+; Node-free fallback remains available for core operations.

## Quick Start

### macOS / Linux

```bash
curl -fsSL https://raw.githubusercontent.com/<github-owner>/InfRing/main/install.sh | sh -s -- --full
infring gateway
```

### Windows (PowerShell)

```powershell
# Use process-scoped bypass so locked-down execution policies do not block install.
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force
$tmp = Join-Path $env:TEMP "infring-install.ps1"
irm https://raw.githubusercontent.com/<github-owner>/InfRing/main/install.ps1 -OutFile $tmp
& $tmp -Repair -Full
# Remove-Item is silent on success in PowerShell.
Remove-Item $tmp -Force
# Confirm command resolution in this shell; if unresolved, use direct-path fallback below.
Get-Command infring -ErrorAction SilentlyContinue
infring gateway
```

If script execution is still restricted in your environment, use a no-file fallback:

```powershell
$env:INFRING_INSTALL_REPAIR = "1"
$env:INFRING_INSTALL_FULL = "1"
irm https://raw.githubusercontent.com/<github-owner>/InfRing/main/install.ps1 | iex
infring gateway
```

If command resolution has not propagated yet in the same session, run directly:

```powershell
$HOME\.infring\bin\infring.cmd gateway
```

If a release does not publish Windows prebuilt binaries for your architecture, the installer now attempts source fallback automatically. On fresh Windows machines, install prerequisites first if needed:

```powershell
winget install --id Git.Git -e
winget install --id Rustlang.Rustup -e
# Optional but often required for MSVC source builds:
winget install --id Microsoft.VisualStudio.2022.BuildTools -e
```

### Verify CLI Is Globally Available

```bash
infring --help
infring list
infring gateway status
```

If your shell has not reloaded `PATH` yet:

```bash
. "$HOME/.infring/env.sh"
hash -r 2>/dev/null || true
infring --help
```

Fallback (direct path):

```bash
"$HOME/.infring/bin/infring" --help
```

Shell-specific activation snippets:

```bash
# zsh / bash
. "$HOME/.infring/env.sh" && hash -r 2>/dev/null || true && infring --help

# fish
set -gx PATH "$HOME/.infring/bin" $PATH; and command -q rehash; and rehash; and infring --help
```

```powershell
# PowerShell
$env:Path = "$HOME/.infring/bin;$env:Path"; infring --help
```

Installer behavior:

- Persists `PATH` to shell startup file(s)
- Writes activation script at `~/.infring/env.sh`
- Applies command shims when install dir is not already on `PATH`
- Supports privileged shim fallback for stricter environments (`INFRING_INSTALL_SUDO_SHIMS=auto|off`)

## Install Modes

| Mode | Flag | Purpose |
|---|---|---|
| Minimal (default) | `--minimal` | CLI + daemon wrappers |
| Full | `--full` | Minimal plus workspace runtime bootstrap (release bundle or source fallback) |
| Pure | `--pure` | Rust-only runtime surface (no Node/TS runtime dependency) |
| Tiny-Max | `--tiny-max` | Lowest-footprint pure profile for constrained hardware |
| Repair | `--repair` | Removes stale wrappers/runtime artifacts before reinstall |

Examples:

```bash
# Pure
curl -fsSL https://raw.githubusercontent.com/<github-owner>/InfRing/main/install.sh | sh -s -- --pure

# Tiny-max
curl -fsSL https://raw.githubusercontent.com/<github-owner>/InfRing/main/install.sh | sh -s -- --tiny-max

# Repair + full
curl -fsSL https://raw.githubusercontent.com/<github-owner>/InfRing/main/install.sh | sh -s -- --repair --full

# In-place update from an existing install
infring update --repair --full

# Offline update from cached release artifacts (must pin version)
infring update --offline --version v0.3.1-alpha --full
```

## Gateway + Dashboard Operations

```bash
# Start runtime + dashboard
infring gateway

# Check runtime + dashboard status
infring gateway status

# Stop runtime + dashboard
infring gateway stop

# Restart
infring gateway restart
```

Default behavior:

- Auto-opens dashboard on launch
- Supervises runtime and dashboard
- Keeps gateway persistent unless explicitly disabled (`--gateway-persist=0`)

## Command Surfaces

### Rust Fallback Surface (No Node.js)

When Node.js is unavailable, `infring` exposes a reduced but operational command set:

- `gateway [start|stop|restart|status]`
- `update [--repair] [--full|--minimal|--pure|--tiny-max] [--version vX.Y.Z]`
- `verify-gateway [--dashboard-host=127.0.0.1] [--dashboard-port=4173]`
- `start`, `stop`, `restart`
- `dashboard`, `status`
- `session <status|register|resume|send|list>`
- `rag <status|search|chat|memory>`
- `memory <status|search>`
- `adaptive <status|propose|shadow-train|prioritize|graduate>`
- `enterprise-hardening <run|status|export-compliance|identity-surface|certify-scale|dashboard>`
- `benchmark <run|status>`
- `alpha-check [--strict=1|0] [--run-gates=1|0]`
- `research <status|diagnostics|fetch>`
- `help`, `list`, `version`

Not available in Node-free fallback:

- `assimilate` (experimental runtime lane; requires Node.js 22+ full surface)

Install Node.js 22+ to unlock full JS-assisted command surfaces.

### Experimental Runtime Surface (Node.js 22+ Required)

- `assimilate <target> [--payload-base64=...] [--strict=1] [--showcase=1] [--duration-ms=<n>] [--json=1]`

Behavior:

- Known targets route to governed core bridge lanes.
- Unknown targets run local simulation mode and are not treated as production integration.

### Local Source Workflow

```bash
npm ci
npm run local:init
npm run build
npm run test:ci
npm run gateway
```

## Performance Snapshot (Latest Artifact)

Latest benchmark source:

- [`docs/client/reports/benchmark_matrix_run_latest.json`](docs/client/reports/benchmark_matrix_run_latest.json)

Current measured rows in that artifact:

| Metric | Rich (`Infring`) | Pure (`InfRing (pure)`) | Tiny-Max (`InfRing (tiny-max)`) |
|---|---:|---:|---:|
| Cold start | 44.731 ms | 1.579 ms | 1.761 ms |
| Idle memory | 8.047 MB | 1.375 MB | 1.375 MB |
| Install artifact size | 25.84 MB | 2.480 MB | 0.617 MB |
| Throughput | 146,306.56 ops/sec | 146,306.56 ops/sec | 146,306.56 ops/sec |
| Security systems | 83 | 83 | 83 |
| Channel adapters | 6 | 0 | 0 |
| LLM providers | 3 | 0 | 0 |
| Data channels | 4 | 0 | 0 |
| Plugin marketplace checks | 4 | 0 | 0 |

Preflight metadata in the same artifact:

- `benchmark_preflight.enabled = false` (run override: `--benchmark-preflight=0`)
- `benchmark_validation.ok = true`
- `sample_cv_pct = 0.36` (tolerance `18.75`)
- Artifact timestamp: `2026-04-06T08:08:01.096Z`

Current nuance:

- Rich lane remains policy-valid with stable install/idle metrics and deterministic throughput sampling.
- Pure and tiny-max lanes continue to preserve low-latency footprint profiles.

### Competitor Comparison (Latest Matrix)

Source: [`docs/client/reports/benchmark_matrix_run_latest.json`](docs/client/reports/benchmark_matrix_run_latest.json)

| Project | Cold Start (ms) | Idle Memory (MB) | Install Size (MB) | Throughput (ops/sec) |
|---|---:|---:|---:|---:|
| Infring | 44.731 | 8.047 | 25.84 | 146,306.56 |
| AutoGen | 4000.0 | 250.0 | 200.0 | n/a |
| CrewAI | 3000.0 | 200.0 | 100.0 | n/a |
| Workflow Graph | 2500.0 | 180.0 | 150.0 | n/a |
| OpenHands | 1300.0 | 150.0 | 95.5 | n/a |

Refresh commands:

```bash
npm run -s ops:benchmark:refresh
npm run -s ops:benchmark:sanity
npm run -s ops:benchmark:public-audit
npm run -s ops:benchmark:repro
```

## What Ships Today

- Rust-authoritative control and policy lanes under `core/layer0`, `core/layer1`, and `core/layer2`
- Gateway runtime with dashboard serving and health probes
- Agent/session/memory/rag/research command surfaces
- Signed policy/config registry surfaces under `client/runtime/config/**`
- Regression and governance gates in `tests/**` and `verify.sh`

## Public SDK

- TypeScript SDK package: `packages/infring-sdk` (`@infring/sdk`)
- Stable contract methods:
  - `submitTask`
  - `inspectReceipts`
  - `queryMemory`
  - `reviewEvidence`
  - `runAssimilation`
  - `attachPolicies`
- Reference app build proof:
  - `examples/apps/reference-task-submit`
  - `examples/apps/reference-receipts-memory`
  - `examples/apps/reference-assimilation-policy`
  - `npm run -s ops:sdk:surface:build`

## Repository Map

| Path | Responsibility |
|---|---|
| `core/` | Rust authority layers and runtime core |
| `client/runtime/systems/` | Runtime wrappers and operator surfaces |
| `client/runtime/config/` | Policy manifests, registries, and guardrails |
| `adapters/` | Integration bridges |
| `apps/` | Runnable app surfaces and examples |
| `tests/` | Regression, governance, and toolchain validation |
| `docs/` | Runbooks, architecture, onboarding, and policies |
| `planes/` | Three-plane architecture contract definitions |
| `install.sh`, `install.ps1` | Cross-platform installers |

## Documentation

- [Getting Started](docs/client/GETTING_STARTED.md)
- [Onboarding Playbook](docs/client/ONBOARDING_PLAYBOOK.md)
- [Developer Lane Quickstart](docs/client/DEVELOPER_LANE_QUICKSTART.md)
- [Operator Runbook](docs/client/OPERATOR_RUNBOOK.md)
- [Security Posture](docs/client/SECURITY_POSTURE.md)
- [Backlog Governance](docs/client/BACKLOG_GOVERNANCE.md)
- [Architecture](ARCHITECTURE.md)
- [Roadmap](roadmap.md)
- [Glossary](glossary.md)

## Contributing

1. Read [CONTRIBUTING.md](docs/workspace/CONTRIBUTING.md).
2. Run tests and required gates for touched surfaces.
3. Keep claims evidence-backed (diff + test output + runtime proof).
4. Update [CHANGELOG.md](docs/workspace/CHANGELOG.md) for user-visible changes.

## Security

- Disclosure policy: [SECURITY.md](SECURITY.md)
- Runtime security docs: [docs/client/SECURITY.md](docs/client/SECURITY.md)

## License

InfRing uses dual licensing:

- Apache-2.0 for open-core scope: [LICENSE-APACHE-2.0](LICENSE-APACHE-2.0)
- LicenseRef-InfRing-NC-1.0 for default NC scope: [LICENSE-INFRING-NC-1.0](LICENSE-INFRING-NC-1.0)

Canonical SPDX matrix: [LICENSE_MATRIX.json](LICENSE_MATRIX.json)  
Human-readable path scope: [LICENSE_SCOPE.md](LICENSE_SCOPE.md)
