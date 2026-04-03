# InfRing

[![CI](https://github.com/protheuslabs/InfRing/actions/workflows/ci.yml/badge.svg)](https://github.com/protheuslabs/InfRing/actions/workflows/ci.yml)
[![CodeQL](https://github.com/protheuslabs/InfRing/actions/workflows/codeql.yml/badge.svg)](https://github.com/protheuslabs/InfRing/actions/workflows/codeql.yml)
[![License: Dual](https://img.shields.io/badge/license-dual%20(NC%20%2B%20Apache--2.0)-orange.svg)](LICENSE_SCOPE.md)
[![Release](https://img.shields.io/github/v/release/protheuslabs/InfRing?display_name=tag)](https://github.com/protheuslabs/InfRing/releases)
[![Docker](https://img.shields.io/badge/docker-ghcr.io%2Fprotheuslabs%2Finfring-blue)](https://github.com/protheuslabs/InfRing/pkgs/container/infring)
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
curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --full
infring gateway
```

### Windows (PowerShell)

```powershell
irm https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.ps1 | iex -Full
infring gateway
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
curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --pure

# Tiny-max
curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --tiny-max

# Repair + full
curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --repair --full
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

Install Node.js 22+ to unlock full JS-assisted command surfaces.

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

- [`docs/client/reports/benchmark_matrix_run_2026-03-06.json`](docs/client/reports/benchmark_matrix_run_2026-03-06.json)

Current measured rows in that artifact:

| Metric | Rich (`Infring`) | Pure (`InfRing (pure)`) | Tiny-Max (`InfRing (tiny-max)`) |
|---|---:|---:|---:|
| Cold start | 207.391 ms | 11.250 ms | 12.919 ms |
| Idle memory | 6.859 MB | 1.344 MB | 1.344 MB |
| Install artifact size | 25.84 MB | 2.443 MB | 0.617 MB |
| Throughput | 4,223.41 ops/sec | 4,223.41 ops/sec | 4,223.41 ops/sec |
| Security systems | 83 | 83 | 83 |
| Channel adapters | 6 | 0 | 0 |
| LLM providers | 3 | 0 | 0 |
| Data channels | 4 | 0 | 0 |
| Plugin marketplace checks | 4 | 0 | 0 |

Preflight metadata in the same artifact:

- `benchmark_preflight.ok = true`
- `noise_cv_pct = 0.0` (limit `100.0`)
- `load_per_core_peak = 3.252` (limit `8.0`)
- Artifact timestamp: `2026-04-02T04:50:35.450Z`

Current nuance:

- Runtime efficiency receipt shows rich mode not passing strict cold-start target (`runtime_receipt.ok = false`) due elevated startup latency.
- Pure and tiny-max lanes remain low-latency footprint profiles.

### Competitor Comparison (Latest Matrix)

Source: [`docs/client/reports/benchmark_matrix_run_2026-03-06.json`](docs/client/reports/benchmark_matrix_run_2026-03-06.json)

| Project | Cold Start (ms) | Idle Memory (MB) | Install Size (MB) | Throughput (ops/sec) |
|---|---:|---:|---:|---:|
| Infring | 207.391 | 6.859 | 25.84 | 4,223.41 |
| AutoGen | 4000.0 | 250.0 | 200.0 | n/a |
| CrewAI | 3000.0 | 200.0 | 100.0 | n/a |
| Workflow Graph | 2500.0 | 180.0 | 150.0 | n/a |
| OpenHands | 1300.0 | 150.0 | 95.5 | n/a |

Refresh commands:

```bash
npm run -s ops:benchmark:refresh
npm run -s ops:benchmark:sanity
```

## What Ships Today

- Rust-authoritative control and policy lanes under `core/layer0`, `core/layer1`, and `core/layer2`
- Gateway runtime with dashboard serving and health probes
- Agent/session/memory/rag/research command surfaces
- Signed policy/config registry surfaces under `client/runtime/config/**`
- Regression and governance gates in `tests/**` and `verify.sh`

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
- InfRing-NC-1.0 for default NC scope: [LICENSE-INFRING-NC-1.0](LICENSE-INFRING-NC-1.0)

See [LICENSE_SCOPE.md](LICENSE_SCOPE.md) for path-level scope resolution.
