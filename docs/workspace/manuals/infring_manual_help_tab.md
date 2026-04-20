# Infring Manual

_Operator-facing guide for the Help tab_

## Table of Contents
- [What Infring Is](#what-infring-is)
- [Install + Start](#install--start)
- [CLI Guide](#cli-guide)
- [UI Guide](#ui-guide)
- [Tools + Evidence](#tools--evidence)
- [Memory + Sessions](#memory--sessions)
- [Safety Model](#safety-model)
- [Troubleshooting](#troubleshooting)
- [Reporting Issues](#reporting-issues)

---

## What Infring Is

Infring is a local, deterministic, receipt-first automation and orchestration runtime.

In practical terms, that means:
- **Kernel truth lives in the Rust kernel.** Critical policy, receipts, execution, and safety decisions are authoritative in kernel lanes. (`Core` remains a compatibility alias for legacy wording.)
- **The orchestration layer coordinates work.** It shapes requests, plans work, handles clarification, and packages results.
- **The client/dashboard is a presentation surface.** It is there to help you operate the system, not to be the source of truth.
- **External systems are reached through the Gateway layer.** (`Adapters` is retained as a compatibility alias during transition.)
- **Operations are evidence-backed.** Important actions and outcomes are designed to be traceable.
- **Failure is designed to be fail-closed.** If Infring is unsure or a required lane is unavailable, the correct result is often to stop, degrade safely, or ask for clarification instead of guessing.

### Runtime Profiles

Infring supports multiple runtime profiles:
- **rich** — full operator experience, including the gateway/dashboard surface.
- **pure** — Rust-only profile with no rich gateway UI surface.
- **tiny-max** — smallest pure profile for constrained environments.

### Experimental Surfaces

Some lanes are explicitly experimental. In particular, the `assimilate` runtime surface is guarded and not part of the normal public production surface.

### When to use Infring

Use Infring when you want:
- a local operator runtime
- deterministic, policy-governed execution
- a dashboard for interactive operation
- a CLI for scripting, verification, and controlled workflows

---

## Install + Start

### Quick install

### macOS / Linux
```bash
curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --full infring gateway
```

### Windows (PowerShell)
Canonical install command (single-line):
`Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force; $tmp = Join-Path $env:TEMP "infring-install.ps1"; irm https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.ps1 -OutFile $tmp -ErrorAction Stop; & $tmp -Repair -Full; Remove-Item $tmp -Force -ErrorAction SilentlyContinue`

```powershell
Set-ExecutionPolicy -Scope Process -ExecutionPolicy Bypass -Force
$tmp = Join-Path $env:TEMP "infring-install.ps1"
irm https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.ps1 -OutFile $tmp -ErrorAction Stop
& $tmp -Repair -Full
Remove-Item $tmp -Force -ErrorAction SilentlyContinue
Get-Command infring -ErrorAction SilentlyContinue
infring gateway
```

If PATH has not refreshed in the same shell, run directly: `$HOME\.infring\bin\infring.cmd gateway`.

If script execution is still restricted in your environment, use a no-file fallback:

```powershell
$env:INFRING_INSTALL_REPAIR = "1"
$env:INFRING_INSTALL_FULL = "1"
irm https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.ps1 | iex
infring gateway
```

If a release has no Windows prebuilt binary for your architecture, installer fallback may require local source build prerequisites:

```powershell
winget install --id Git.Git -e
winget install --id Rustlang.Rustup -e
winget install --id Microsoft.VisualStudio.2022.BuildTools -e --override "--quiet --wait --norestart --add Microsoft.VisualStudio.Workload.VCTools"
```

When `winget` is unavailable or fails, the installer falls back to the direct Visual Studio bootstrapper:
`https://aka.ms/vs/17/release/vs_BuildTools.exe`

For locked-down environments, disable auto bootstrap lanes explicitly:

```powershell
$env:INFRING_INSTALL_AUTO_MSVC = "0"
$env:INFRING_INSTALL_ALLOW_DIRECT_MSVC_BOOTSTRAP = "0"
$env:INFRING_INSTALL_AUTO_RUSTUP = "0"
$env:INFRING_INSTALL_ALLOW_COMPATIBLE_RELEASE_FALLBACK = "0"
$env:INFRING_INSTALL_ALLOW_PINNED_VERSION_COMPATIBLE_FALLBACK = "0"
```

If a pinned release lacks required Windows prebuilts, opt in to compatible-release prebuilt fallback:
`$env:INFRING_INSTALL_ALLOW_PINNED_VERSION_COMPATIBLE_FALLBACK = "1"`.

### Verify the CLI
```bash
infring --help
infring list
infring gateway status
```

If your shell has not refreshed `PATH` yet:
```bash
. "$HOME/.infring/env.sh"
hash -r 2>/dev/null || true
infring --help
```

Direct-path fallback:
```bash
"$HOME/.infring/bin/infring" --help
```

PowerShell fallback:
```powershell
$env:Path = "$HOME/.infring/bin;$env:Path"
infring --help
```

### Start the operator surface
```bash
infring gateway
```

This starts the runtime and dashboard.

Primary dashboard URL:
```text
http://127.0.0.1:4173/dashboard#chat
```

Health endpoint:
```text
http://127.0.0.1:4173/healthz
```

### Common lifecycle commands
```bash
infring gateway status
infring gateway stop
infring gateway restart
```

### Install modes
- `--minimal` — CLI + daemon wrappers
- `--full` — full runtime bootstrap
- `--pure` — Rust-only runtime surface
- `--tiny-max` — smallest pure profile
- `--repair` — clean reinstall / stale-artifact cleanup

Examples:
```bash
# pure profile
curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --pure

# tiny-max profile
curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --tiny-max

# repair + full
curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --repair --full

# in-place update
infring update --repair --full
```

---

## CLI Guide

### Primary entrypoints
- `infring` — main operator entrypoint
- `infringctl` — wrapper/control surface
- `infringd` — daemon-oriented wrapper

### Everyday commands
```bash
infring help
infring list
infring version
infring gateway
infring gateway status
infring gateway stop
infring gateway restart
```

### Operational fallback surface
When Node.js is unavailable, Infring still exposes a reduced Rust-backed surface.

Available fallback families include:
- `gateway [start|stop|restart|status]`
- `update`
- `verify-gateway`
- `start`, `stop`, `restart`
- `dashboard`, `status`
- `session`
- `rag`
- `memory`
- `adaptive`
- `enterprise-hardening`
- `benchmark`
- `alpha-check`
- `research`
- `help`, `list`, `version`

Not available in Node-free fallback:
- `assimilate`

### Full / experimental surface
`assimilate` requires the full Node.js-assisted surface and should be treated as experimental.

Example:
```bash
infring assimilate target-name --plan-only=1 --json=1
```

Useful flags:
- `--plan-only=1` — emit the planning chain without executing mutations
- `--json=1` — structured output
- `--strict=1` — tighter enforcement
- `--allow-local-simulation=1` — test-only local simulation path

### Contributor / repository workflows
If you are working from the repository directly, these are the canonical workspace entrypoints:
```bash
npm run -s workspace:commands
npm run -s tooling:list
npm run -s workspace:dev
npm run -s workspace:verify
npm run -s lane:list -- --json=1
```

---

## UI Guide

### What the dashboard is for
The dashboard is the primary interactive operator surface in the **rich** profile. It is the right place to:
- work interactively
- inspect status and outputs
- use the chat/operator surface
- read built-in help
- validate that the runtime is up before you move into deeper CLI/ops work

### What the dashboard is not
The dashboard is **not** the system’s source of truth. If the UI and the runtime disagree, trust the runtime’s receipts, status commands, and support artifacts.

### Recommended operator workflow
1. Start the system with `infring gateway`.
2. Open the dashboard.
3. Use the chat/operator surface for interactive work.
4. Use CLI status commands for verification when needed.
5. Use support/export tooling when diagnosing incidents or filing issues.

### Rich vs pure profiles
- **rich**: dashboard available
- **pure / tiny-max**: intentionally no rich gateway UI surface

If you are on `--pure` or `--tiny-max`, use the CLI instead of expecting the dashboard.

### Accessibility expectations
The UI contract expects:
- keyboard navigation for primary actions
- visible focus indicators
- sufficient contrast for critical text
- documented discoverability for the command palette / primary actions

---

## Tools + Evidence

### What tools mean in Infring
A tool is an operator-usable lane that performs a governed action through the runtime. Infring is designed so important actions are policy-governed and evidence-backed instead of being opaque side effects.

### What evidence means
Evidence is the supporting record for a claim, result, or action. Infring’s documentation policy is explicit: measurable, comparative, security-sensitive, or customer-impacting claims must have linked evidence.

Examples of evidence include:
- receipts
- benchmark artifacts
- verification outputs
- drill / recovery artifacts
- support bundles
- logs and state artifacts when shareable and appropriate

### How to interpret outputs
When reading a result, ask:
- What happened?
- What evidence supports it?
- Was the action successful, degraded, blocked, or fail-closed?
- Is there a receipt, artifact, or status record I can inspect?

### Practical rule
If you want to make a public claim about performance, reliability, or security, do not rely on UI text alone. Link the supporting artifact.

### Useful evidence/ops commands
```bash
npm run -s ops:production-topology:status
npm run -s ops:transport:spawn-audit
npm run -s ops:support-bundle:export
npm run -s ops:release:verdict
```

---

## Memory + Sessions

### Sessions
Use sessions for active operator work and live runtime context.

### Memory
Use memory surfaces for persisted runtime state and retrieval-oriented workflows.

### RAG / retrieval
Use `rag` when you want retrieval-style behavior over indexed or memory-backed content.

### Session and memory command families
```bash
infring session
infring memory
infring rag
```

### Operator guidance
- Treat sessions as active working context.
- Treat memory as a governed system surface, not a scratchpad you can assume is unbounded.
- If a workflow matters, validate it through receipts/artifacts instead of assuming a UI-only state is durable.
- If you are troubleshooting a session problem, prefer runtime status and support-bundle export over guessing from stale UI state.

---

## Safety Model

Infring’s safety model is one of its defining traits.

### Kernel rules
- Safety authority stays deterministic and fail-closed.
- AI/probabilistic logic is not the root of correctness.
- Kernel truth lives in the authoritative kernel.
- External-system access must cross governed gateway contracts (adapter compatibility layer), never direct ad hoc bypasses.
- Boundary crossing is explicit and governed.
- Unsupported or unadmitted actions should stop or degrade safely.

### What that means for operators
- If a command is blocked, that is often the correct behavior.
- Experimental features may require explicit flags and extra validation.
- Production release channels are resident-IPC authoritative.
- Legacy process transport is not a supported production path.

### Security posture
The repository’s security posture emphasizes:
- fail-closed policy checks
- deterministic receipts on critical lanes
- least-authority command routing
- release-time evidence such as SBOMs, CodeQL, and verification artifacts

### Vulnerability reporting
Do **not** file public GitHub issues for security vulnerabilities. Use private reporting instead.

---

## Troubleshooting

### `infring` command not found
Reload your shell environment:
```bash
. "$HOME/.infring/env.sh"
hash -r 2>/dev/null || true
infring --help
```

Direct-path fallback:
```bash
"$HOME/.infring/bin/infring" --help
```

### Gateway/dashboard is not available
Check status:
```bash
infring gateway status
```

Check health endpoint:
```text
http://127.0.0.1:4173/healthz
```

Restart:
```bash
infring gateway restart
```

### You expected a dashboard, but none appears
You may be using `--pure` or `--tiny-max`, which intentionally do not expose the rich gateway UI surface.

### A command is missing
Run:
```bash
infring --help
infring list
```

If Node.js is unavailable, you are probably seeing the reduced fallback surface.

### Experimental lane fails closed
That is often expected behavior. For example, unknown assimilation targets are unadmitted by default, and local simulation requires explicit opt-in.

### You need a deeper incident path
Use the operator runbook and export a support bundle.

Useful commands:
```bash
npm run -s ops:support-bundle:export
npm run -s ops:status:production
npm run -s ops:production-topology:status
```

### Strict checks are failing in local repo work
Run the canonical verification path:
```bash
npm run -s workspace:verify
```

For surface/docs checks:
```bash
node client/runtime/systems/ops/docs_surface_contract.ts check --strict=1
node client/runtime/systems/ops/root_surface_contract.ts check --strict=1
```

---

## Reporting Issues

### Before filing
Please gather:
- summary of the problem
- reproduction steps
- expected behavior
- environment details (OS, Node, Rust, CLI version, relevant config)

### Public bug reports
Use the GitHub bug report template.

Include:
- what happened
- how to reproduce it
- what you expected instead
- environment details

### Feature requests
Use the feature request template.

Include:
- the problem you are trying to solve
- the proposed solution
- alternatives considered
- expected impact

### Security issues
Do **not** open a public issue for a vulnerability.

Use the private security disclosure path and include:
- impact summary
- reproduction steps
- affected files/modules
- suggested mitigation if known
- severity estimate and blast radius

### Good issue hygiene
A good issue report makes it easier to help you quickly:
- keep it specific
- attach the exact command or workflow
- include relevant receipts/artifacts if safe to share
- note whether you are on rich, pure, or tiny-max
- say whether the problem is reproducible or intermittent

---

## Quick Reference

### Start / stop
```bash
infring gateway
infring gateway status
infring gateway stop
infring gateway restart
```

### Verify installation
```bash
infring --help
infring list
```

### Update
```bash
infring update --repair --full
```

### Support / diagnostics
```bash
npm run -s ops:status:production
npm run -s ops:production-topology:status
npm run -s ops:support-bundle:export
```

### Important URLs
- Dashboard: `http://127.0.0.1:4173/dashboard#chat`
- Health: `http://127.0.0.1:4173/healthz`

---

## Final Notes

If you are unsure whether to trust the UI or the runtime, trust the runtime.

If a lane fails closed, treat that as a protective behavior first, not a product failure first.

If you are making a strong claim, link the evidence.
