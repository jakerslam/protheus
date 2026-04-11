# InfRing

[![License: Dual](https://img.shields.io/badge/license-dual%20(NC%20%2B%20Apache--2.0)-orange.svg)](LICENSE_SCOPE.md)
[![Architecture](https://img.shields.io/badge/architecture-three--plane%20metakernel-0A7A5E)](planes/README.md)
![Coverage](docs/client/badges/coverage.svg)

InfRing is a deterministic, receipt-first autonomous runtime built on a three-plane metakernel.  
It is designed for verifiable execution, fail-closed safety, and reproducible operator workflows.

Core authority is Rust-first (`core/**`).  
Orchestration coordination lives in `surface/orchestration/**` (non-canonical, contract-driven).  
Client/runtime surfaces remain thin presentation wrappers around policy-governed core lanes.

## Why InfRing

- Deterministic execution with evidence-backed receipts.
- Fail-closed safety and policy enforcement by default.
- Rust-authoritative core with explicit thin-client boundaries.
- Multi-profile runtime strategy: rich, pure, and tiny-max.
- Operator-first CLI and gateway operator surface (presentation + orchestration ingress).

## Architecture At A Glance

| Plane | Role |
|---|---|
| Safety Plane | Deterministic guardrails, invariants, fail-closed behavior |
| Cognition Plane | Orchestration coordination + presentation cognition surfaces (non-authoritative) |
| Substrate Plane | Runtime integration, execution surfaces, system bridges |

Runtime split inside cognition:

- Authoritative Core: `core/**`
- Orchestration Surface: `surface/orchestration/**`
- Presentation Client: `client/**`

See [planes/README.md](planes/README.md) for the canonical architecture contract.

## Current State (April 2026)

What is true in this repository today:

- Primary operator entrypoint is `infring` (with `infringctl` and `infringd` wrappers).
- Main dashboard is served by the gateway at `http://127.0.0.1:4173/dashboard#chat`.
- Gateway health endpoint is `http://127.0.0.1:4173/healthz`.
- Gateway persistence is enabled by default (auto-restart + reboot supervision unless disabled).
- Pure profiles (`--pure`, `--tiny-max`) are Rust-only and intentionally do not expose the rich `gateway` UI surface.
- Full command surface still requires Node.js 22+; Node-free fallback remains available for core operations.
- Production release channels are resident-IPC authoritative: process transport fallbacks are blocked (`process_transport_forbidden_in_production` / `process_fallback_forbidden_in_production`).
- Release-closure evidence now includes topology diagnostics, live stateful upgrade/rollback rehearsal, recovery rehearsal, numeric release scorecards, and support-bundle export.

## Production Support Contract

- Canonical production profile: rich
- Constrained profiles: `--pure`, `--tiny-max`
- Experimental lanes (explicit opt-in): `assimilate`
- Resident IPC is the only supported production topology; the legacy process runner is dev-only.
- Release entrypoints quarantine the legacy runner under `adapters/runtime/dev_only/**`.
- Legacy runner deletion target: remove `adapters/runtime/dev_only/legacy_process_runner.ts` by `v0.3.11-stable` / `2026-05-15` unless an explicit open release blocker depends on it.
- Operator topology diagnostic: `npm run -s ops:production-topology:status`
- Transport spawn audit: `npm run -s ops:transport:spawn-audit`
- Assimilation v1 support guard: `npm run -s ops:assimilation:v1:support:guard`
- Frozen assimilation v1 slice: one ingress -> orchestration -> assimilation-kernel -> receipt-output path is hardened; broader assimilation surfaces remain experimental.
- Assimilation v1 can graduate only through candidate-build evidence; no new assimilation surface is added during hardening.
- Numeric release thresholds are enforced by `npm run -s ops:release:scorecard:gate` and re-checked directly by `npm run -s ops:production-closure:gate`.
- Release-candidate dress rehearsal: `npm run -s ops:release:rc-rehearsal`.
- Release-candidate recovery rehearsal is required every cycle through `npm run -s ops:release:rc-rehearsal`.
- Client authority regression guard: `npm run -s ops:client-layer:boundary`.
- Support bundle is the single incident truth package for release closure.
- Internal/maintenance lanes are not part of the public production SLA.
- Operator diagnostics and incident export: `npm run -s ops:support-bundle:export`

## Quick Start

### macOS / Linux

```bash
curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --full
infring gateway
```

### Windows (PowerShell)

```powershell
# Use process-scoped bypass so locked-down execution policies do not block install.
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force
$tmp = Join-Path $env:TEMP "infring-install.ps1"
irm https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.ps1 -OutFile $tmp
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
irm https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.ps1 | iex
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

## Repository Workflows

For local repository work, use the canonical workspace entrypoints:

- Inspect the script surface: `npm run -s workspace:commands`
- Inspect the governed tooling registry: `npm run -s tooling:list`
- Start the canonical local dev loop: `npm run -s workspace:dev`
- Run the canonical full verification path: `npm run -s workspace:verify`
- Run the fast tooling profile: `npm run -s tooling:profile -- --id=fast`
- Run any registered tooling gate: `npm run -s tooling:run -- --id=ops:arch:conformance`
- Inspect indexed lane inventory: `npm run -s lane:list -- --json=1`
- Run any registered lane: `npm run -s lane:run -- --id=<ID>`
- Run lane-specific regression coverage: `npm run -s test:lane:run -- --id=<ID>`

`workspace:verify` delegates to the root [`verify.sh`](/Users/jay/.openclaw/workspace/verify.sh) pipeline, which now reads the manifest-driven tooling profile in [`tests/tooling/config/verify_profiles.json`](/Users/jay/.openclaw/workspace/tests/tooling/config/verify_profiles.json) through the shared runner at [`tests/tooling/scripts/ci/tooling_registry_runner.ts`](/Users/jay/.openclaw/workspace/tests/tooling/scripts/ci/tooling_registry_runner.ts). `workspace:ci` is the canonical CI-equivalent alias for the same path.

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

- `assimilate <target> [--payload-base64=...] [--strict=1] [--showcase=1] [--duration-ms=<n>] [--json=1] [--allow-local-simulation=1] [--plan-only=1] [--hard-selector=<selector>] [--selector-bypass=1]`

Behavior:

- Known targets route to governed core bridge lanes.
- Unknown targets fail as `unadmitted` by default.
- Local simulation mode is test-only and must be explicitly enabled via `--allow-local-simulation=1`.
- Use `--plan-only=1` to emit the canonical assimilation planning chain without executing bridge mutations.

### Release-Closure Diagnostics

- `npm run -s ops:production-topology:status`
- `npm run -s ops:legacy-runner:release-guard`
- `npm run -s ops:stateful-upgrade-rollback:gate`
- `npm run -s dr:gameday`
- `npm run -s dr:gameday:gate`
- `npm run -s ops:support-bundle:export`

### Local Source Workflow

```bash
npm ci
npm run local:init
npm run build
npm run test:ci
npm run gateway
```

<!-- BEGIN: benchmark-snapshot -->
## Performance Snapshot (Latest Artifact)

Latest benchmark source:

- [`docs/client/reports/benchmark_matrix_run_latest.json`](docs/client/reports/benchmark_matrix_run_latest.json)

Canonical throughput metric (kernel/shared workload): `kernel_shared_workload_ops_per_sec`
Rich zero-boot cold-start metric: `rich_zero_boot_command_path_cold_start_ms`
Rich end-to-end command-path throughput metric: `rich_end_to_end_command_path_ops_per_sec`
Realistic workload throughput metric: `realistic_user_workload_ops_per_sec`

### Benchmark Categories

- **Micro-kernel benchmarks (Pure / Tiny-Max):** shared hot-path kernel workload metrics (intentionally optimistic).
- **Zero-boot lane (Rich):** stopped-from-zero command-process launch from disk with daemon reuse disabled.
- **Full governed path benchmark (Rich):** command path through runtime guards and receipts.
- **Realistic user workload (Rich):** ten governed tool calls with memory cache and receipt side effects per sample.
- **Cold start definitions:** status-path readiness and a distinct rich zero-boot lane are both published to avoid metric blending.

Current measured rows in that artifact:

| Metric | Rich | Pure (`InfRing (pure)`) | Tiny-Max (`InfRing (tiny-max)`) |
|---|---:|---:|---:|
| Readiness latency (status-path; not zero-boot) | 0.004 ms | 1.908 ms | 2.738 ms |
| Cold start (zero-boot rich command lane; stopped-from-zero) | 218.796 ms | n/a | n/a |
| Cold start (engine init micro) | 0.004 ms | n/a | n/a |
| Cold start (orchestration component) | 0.000 ms | n/a | n/a |
| Kernel ready | 0.004 ms | n/a | n/a |
| Gateway ready | 0.004 ms | n/a | n/a |
| Dashboard interactive | 0.004 ms | n/a | n/a |
| Idle memory | 8.359 MB | 1.516 MB | 1.516 MB |
| Install artifact size | 28.116 MB | 3.837 MB | 0.631 MB |
| Throughput (kernel/shared workload) | 315,070.20 ops/sec | 315,070.20 ops/sec | 315,070.20 ops/sec |
| Throughput (rich end-to-end command path) | 4.22 ops/sec | n/a | n/a |
| Throughput (realistic user workload: 10 tool calls + memory + receipts) | 160.67 ops/sec | n/a | n/a |
| Zero-boot lane sample rigor | n=30; p50=218.796 ms; p95=245.237 ms; p99=281.783 ms; min=207.408 ms; max=281.783 ms | n/a | n/a |
| Rich command-path sample rigor | n=30; p50=233.032 ms; p95=262.306 ms; p99=272.664 ms; min=224.296 ms; max=272.664 ms | n/a | n/a |
| Realistic workload sample rigor | n=30; p50=61.733 ms; p95=68.001 ms; p99=70.395 ms; min=58.189 ms; max=70.395 ms | n/a | n/a |
| Security systems | 83 | 83 | 83 |
| Channel adapters | 6 | 0 | 0 |
| LLM providers | 3 | 0 | 0 |
| Data channels | 4 | 0 | 0 |
| Plugin marketplace checks | 4 | 0 | 0 |

Preflight metadata in the same artifact:

- `benchmark_preflight.enabled = true`
- `benchmark_validation.ok = true`
- `sample_cv_pct = 2.16` (tolerance `18.75`)
- Artifact timestamp: `2026-04-09T17:39:25.221Z`

Current nuance:

- Public benchmark summaries are generated from the canonical artifact during refresh and verified by `ops:benchmark:public-audit`.
- Reproducibility commands are listed below; claims should match the linked JSON artifact exactly.
- `kernel_shared_workload_ops_per_sec` is a shared kernel workload metric; treat it separately from end-to-end runtime throughput.
- `rich_zero_boot_command_path_cold_start_ms` is the stopped-from-zero rich command-process cold-start lane.
- `rich_end_to_end_command_path_ops_per_sec` is the rich command-path throughput metric measured through the governed command bridge.
- `realistic_user_workload_ops_per_sec` runs ten governed tool calls per sample with memory/receipt side effects.
- `cold_start_ms` in this matrix is a status-path readiness metric, not a full stopped-from-zero dashboard boot benchmark.
- JSON report includes raw run arrays and percentile summaries for command-path metrics (median/p95/p99/min/max).
- Competitor rows are matrix snapshots from the canonical artifact; when methodology is unavailable, entries are treated as comparative references only.

### Competitor Comparison (Latest Matrix)

Source: [`docs/client/reports/benchmark_matrix_run_latest.json`](docs/client/reports/benchmark_matrix_run_latest.json)

| Project | Cold Start (ms) | Idle Memory (MB) | Install Size (MB) | Throughput (ops/sec) |
|---|---:|---:|---:|---:|
| Infring | 0.004 | 8.359 | 28.116 | 315,070.20 |
| AutoGen | 4000.000 | 250.000 | 200.000 | 0.00 |
| CrewAI | 3000.000 | 200.000 | 100.000 | 0.00 |
| OpenHands | 1300.000 | 150.000 | 95.500 | 0.00 |
| Workflow Graph | 2500.000 | 180.000 | 150.000 | 0.00 |

Refresh commands:

```bash
npm run -s ops:benchmark:refresh
npm run -s ops:benchmark:sanity
npm run -s ops:benchmark:public-audit
npm run -s ops:benchmark:repro
```
<!-- END: benchmark-snapshot -->














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
