# Onboarding Playbook

## Objective

Provide role-based onboarding that can scale to a larger engineering organization without losing safety, quality, or velocity.

## Canonical First-Run Sequence (Install -> Setup -> Gateway)

All tracks use the same first-run sequence before role-specific steps:

1. Install full runtime:
   - `curl -fsSL https://raw.githubusercontent.com/protheuslabs/InfRing/main/install.sh | sh -s -- --full`
2. Complete setup:
   - `infring setup --yes --defaults`
   - `infring setup status --json`
3. Start gateway:
   - `infring gateway`
4. Verify:
   - `infring gateway status`

Role bootstrap one-command contract:

| Role | Bootstrap command | Expected outcomes |
| --- | --- | --- |
| `operator` | `./tests/tooling/scripts/onboarding/infring_onboarding_bootstrap.sh --role=operator --install=1 --setup=1 --install-mode=full --gateway=1` | `binary_outcome=ready`, `setup_outcome=completed`, `setup_status_check=completed`, `gateway_outcome=started`, `status=success` |
| `backend` | `./tests/tooling/scripts/onboarding/infring_onboarding_bootstrap.sh --role=backend --install=1 --setup=1 --install-mode=full` | `binary_outcome=ready`, `setup_outcome=completed`, `setup_status_check=completed`, `gateway_outcome=not_requested`, `status=success` |
| `infra` | `./tests/tooling/scripts/onboarding/infring_onboarding_bootstrap.sh --role=infra --install=1 --setup=1 --install-mode=full` | `binary_outcome=ready`, `setup_outcome=completed`, `setup_status_check=completed`, `gateway_outcome=not_requested`, `status=success` |
| `security` | `./tests/tooling/scripts/onboarding/infring_onboarding_bootstrap.sh --role=security --install=1 --setup=1 --install-mode=full` | `binary_outcome=ready`, `setup_outcome=completed`, `setup_status_check=completed`, `gateway_outcome=not_requested`, `status=success` |

## Release-Mode Runtime Surface Contract

Authoritative runtime manifest:
- `client/runtime/config/install_runtime_manifest_v1.txt`

Required vs optional command surfaces:

| Surface | Required in all modes | `full` | `minimal` | `pure` / `tiny-max` |
| --- | --- | --- | --- | --- |
| Wrappers (`infring`, `infringctl`, `infringd`) | Yes | Yes | Yes | Yes |
| Setup lane (`infring setup`, `infring setup status --json`) | Yes | Yes | Yes | Yes |
| Gateway status (`infring gateway status`) | Yes | Yes | Yes | Yes |
| Rich gateway launch (`infring gateway`) | Optional | Available | Available (explicit setup may be required) | Limited/optional by design |

Wrapper contract:

- Canonical wrappers: `infring`, `infringctl`, `infringd`
- Legacy aliases (`infring`, `infringctl`, `infringd`) are deprecated compatibility shims.

## Shared Prerequisites

- Node and npm installed (version from repo lockfile/tooling)
- Local clone of repository
- Ability to run:
  - `npm ci`
  - `npm run build`
  - `npm run test`
- Read before first code change:
  - `README.md`
  - `docs/workspace/CONTRIBUTING.md`
  - `docs/client/OPERATOR_RUNBOOK.md`
  - `docs/client/HISTORY_CLEANLINESS.md`
  - `docs/client/CLAIM_EVIDENCE_POLICY.md`

## Track A: Operator (Day 0 / Day 7 / Day 30)

### Day 0

- Run one-command bootstrap from repo root (installs if missing + starts dashboard):
  - `./tests/tooling/scripts/onboarding/infring_onboarding_bootstrap.sh --role=operator --install=1 --setup=1 --install-mode=full --gateway=1`
- Verify control plane:
  - `infring gateway status`
- Verify setup state:
  - `infring setup status --json`
- Verify onboarding artifacts:
  - `local/state/ops/onboarding_portal/bootstrap_operator.json`
  - `local/state/ops/onboarding_portal/bootstrap_operator.txt`
  - `local/state/ops/onboarding_portal/bootstrap_operator_setup_status.json`
- Expected outcomes:
  - `binary_outcome=ready`
  - `setup_outcome=completed`
  - `setup_status_check=completed`
  - `gateway_outcome=started`
  - `status=success`
- Read escalation surfaces in `docs/client/OPERATOR_RUNBOOK.md`.
- Confirm ability to run backlog sync:
  - `npm run ops:backlog:registry:sync`

### Day 7

- Execute one full dry-run of:
  - backlog update
  - docs update
  - validation check
  - changelog entry
- Submit one pull request with complete template fields.

### Day 30

- Lead one release-note pass.
- Perform one governance audit of generated backlog views and document outcome.

## Track B: Platform Engineer (Day 0 / Day 7 / Day 30)

### Day 0

- Bootstrap first so CLI/runtime are ready:
  - `./tests/tooling/scripts/onboarding/infring_onboarding_bootstrap.sh --role=backend --install=1 --setup=1 --install-mode=full`
- Run full local baseline:
  - `npm ci`
  - `npm run lint`
  - `npm run test`
- Verify onboarding artifacts:
  - `local/state/ops/onboarding_portal/bootstrap_backend.json`
  - `local/state/ops/onboarding_portal/bootstrap_backend.txt`
  - `local/state/ops/onboarding_portal/bootstrap_backend_setup_status.json`
- Expected outcomes:
  - `binary_outcome=ready`
  - `setup_outcome=completed`
  - `setup_status_check=completed`
  - `gateway_outcome=not_requested`
  - `status=success`
- Identify one lane in `client/runtime/systems/` and map:
  - entrypoint
  - tests
  - dependent client/runtime/config/state files

### Day 7

- Ship one scoped change with:
  - tests
  - docs update
  - changelog entry
  - evidence links in PR

### Day 30

- Own one runbook/doc page with explicit review cadence.
- Participate in one incident rehearsal or recovery drill.

## Track C: External Contributor (Day 0 / Day 7 / Day 30)

### Day 0

- Read contribution and security policies.
- Use issue templates for bug/feature intake.
- Verify local build + tests pass before opening PR.

### Day 7

- Land one reviewed PR following commit hygiene and validation checklist.

### Day 30

- Participate in triage rotation (labels, prioritization, closure hygiene).

## Safety Gates

- Never bypass security disclosure workflow for vulnerabilities.
- Never hand-edit generated backlog registry/view artifacts.
- Never publish public metrics/claims without linked evidence.

## Success Criteria

- New engineer can produce a valid PR in < 1 day.
- Operator can execute and verify a backlog sync without assistance.
- No onboarding PR is merged without tests/docs/client/changelog coverage.

## Escalation Path

- Build/test failures: platform owner on current lane.
- Governance mismatch: backlog governance owner.
- Security concerns: follow `SECURITY.md` private reporting guidance.
