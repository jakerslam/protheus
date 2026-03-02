# OpenClaw

OpenClaw is a personal automation and orchestration toolkit for macOS/Linux workflows, with a typed control plane, policy-guarded runtime modules, and reproducible state/receipt contracts for reliable local operations.

## Quick Start

```bash
npm ci
npm run build
npm run start
```

## Features

- Extensionless CLI entrypoints (`protheus`, `protheusd`, `protheusctl`, `protheus-top`)
- TypeScript-first systems with generated JS wrappers for runtime compatibility
- Policy-driven control plane with explicit state receipts and audit artifacts
- Modular lanes across ops, security, memory, routing, and observability
- CI gates for typechecks, contract validation, and deterministic test suites

## Architecture Overview

The repository is organized into runtime lanes and shared primitives. `systems/` holds executable modules grouped by domain (ops, security, memory, routing, sensory, etc.). `lib/` contains shared runtime helpers used by lane implementations. `config/` defines policy surfaces and lane behavior settings. `state/` is the runtime artifact area (ignored from git by default), while `memory/tools/tests/` provides deterministic test coverage for each lane contract.

The control plane is operated through CLI commands that map to lane actions (`run`, `status`, `verify`, etc.) and produce structured artifacts (`latest`, `receipts`, `history`). This keeps day-to-day operations scriptable while preserving replayable evidence for changes.

## Commands

| Command | Purpose |
|---|---|
| `npm run dev` | Start the local daemon control surface |
| `npm run start` | Start the daemon control surface |
| `npm run build` | Build TypeScript outputs and run build smoke checks |
| `npm run test` | Run the stable CI test suite |
| `npm run lint` | Run TypeScript/system lint gate (`typecheck:systems`) |
| `npm run security:audit` | Run npm dependency audit |
| `protheus status` | Show control-plane status |
| `protheusd start` | Start daemon facade |
| `protheusd stop` | Stop daemon facade |
| `protheusctl job-submit --kind=reconcile` | Submit a control-plane job |

## Documentation

- [Public Operator Profile](docs/PUBLIC_OPERATOR_PROFILE.md)
- [Documentation Hub](docs/README.md)
- [Operator Runbook](docs/OPERATOR_RUNBOOK.md)
- [Onboarding Playbook](docs/ONBOARDING_PLAYBOOK.md)
- [UI Surface Maturity Matrix](docs/UI_SURFACE_MATURITY_MATRIX.md)
- [History Cleanliness Program](docs/HISTORY_CLEANLINESS.md)
- [Claim-Evidence Policy](docs/CLAIM_EVIDENCE_POLICY.md)
- [Public Collaboration Triage Contract](docs/PUBLIC_COLLABORATION_TRIAGE.md)
- [Changelog](CHANGELOG.md)
- [Branch Protection Policy](docs/BRANCH_PROTECTION_POLICY.md)
- [Security Lanes](docs/SECURITY.md)
- [Compliance Posture](docs/COMPLIANCE_POSTURE.md)

<details>
<summary>Legal</summary>

- License: [LICENSE](LICENSE)
- Contribution terms: [CONTRIBUTING_TERMS.md](CONTRIBUTING_TERMS.md)
- Terms of service: [TERMS_OF_SERVICE.md](TERMS_OF_SERVICE.md)
- End-user license: [EULA.md](EULA.md)

</details>
