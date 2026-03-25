# Changelog

All notable changes to this repository are documented in this file.

This project follows a strict evidence-backed changelog model:
- Every entry must map to merged code/docs in the same date window.
- Public-facing claims must reference verifiable artifacts.
- Backlog IDs should be included when work is driven by roadmap tracks.

## [Unreleased]

### Added
- `V4-SELF-001` self-audit lane baseline:
  - Rust scanner core: `client/runtime/systems/self_audit/illusion_integrity_auditor.rs`
  - Lane orchestrator: `client/runtime/systems/self_audit/illusion_integrity_lane.ts`
  - Policy and docs surface:
    - `client/runtime/config/illusion_integrity_auditor_policy.json`
    - `docs/client/ILLUSION_INTEGRITY_AUDITOR.md`
  - Control-plane trigger integration (`startup`, `promotion`, `protheusctl audit illusion`)

### Changed
- Replaced root `README.md` with an evidence-first control-plane overview aligned to the Empty Fort artifact standard (operator onboarding, governance surfaces, and quality/security gates mapped to real scripts/docs).
- OSS readiness uplift:
  - Non-commercial legal posture standardized to `Protheus Non-Commercial License v1.0` across root/npm/python manifests.
  - Governance links surfaced in `README.md` (Code of Conduct + issue/PR templates).
  - Added release-prep version bump to `0.2.1-alpha.1` for alpha cut gating.
- Alpha-readiness hardening:
  - Local migration now imports legacy root continuity + `memory/**` with conflict-safe archive semantics (`client/runtime/systems/ops/local_runtime_partitioner.ts`).
  - CLI wrapper resiliency improved with cargo fallback and explicit launcher failure diagnostics (`client/cli/bin/protheus`, `client/cli/bin/protheusctl`, `client/cli/bin/protheusd`, `client/cli/bin/protheus-top`).
  - Benchmark snapshot moved to explicit competitor reference data (OpenFang/OpenHands baseline) and reproducible refresh command (`npm run ops:benchmark:refresh`).
  - Removed machine-specific absolute path leakage from active configs/scripts/docs by migrating to `${WORKSPACE_ROOT}` tokenized paths.
  - Repository legal posture changed to Protheus Non-Commercial License v1.0 (`LICENSE`, `README.md`, `SECURITY.md`, `package.json`).

## [2026-03-25]

### Added
- UI source-run grouping contract for multi-origin chat rendering:
  - Source-aware first/last run logic in `client/runtime/systems/ui/openclaw_static/js/pages/chat.ts` (`messageSourceKey`, `isFirstInSourceRun`, `isLastInSourceRun`).
  - System-tail style support in `client/runtime/systems/ui/openclaw_static/css/components.css`.

### Changed
- Runtime chat event metadata now preserves source identity for system/agent-origin rows:
  - `system_origin`, `source_agent_id`, `source_agent_name`, and related fields propagated through session payloads in `client/runtime/systems/ui/infring_dashboard.ts`.
- Dashboard chat cache versioning now invalidates stale pre-format cache data to prevent old grouping behavior from persisting (`conversationCacheVersion` in `chat.ts`).
- Dashboard launch resilience hardened with startup retry/backoff and deterministic server error status output (`client/runtime/systems/ops/protheus_status_dashboard.ts`, `client/runtime/systems/ui/infring_dashboard.ts`).

## [2026-03-24]

### Added
- Backfilled release note for previously merged swarm-runtime functionality:
  - Runtime swarm recommendation execution path with deterministic role/task dispatch and receipt-backed outcomes in `client/runtime/systems/ui/infring_dashboard.ts` (`executeRuntimeSwarmRecommendation`, `queueAgentTask`).

### Changed
- Runtime auto-heal orchestration expanded for sustained swarm operation:
  - Conduit watchdog auto-restart/drain policy handling.
  - Predictive drain launch/release behavior under queue pressure.
  - Cockpit stale-lane remediation flow.
  - Evidence surfaces retained in dashboard snapshot/runtime recommendation payloads.

## [2026-03-02]

### Added
- V4-FORT artifact baseline for enterprise-grade presentation:
  - UI surface maturity matrix and update cadence (`docs/client/UI_SURFACE_MATURITY_MATRIX.md`)
  - Role-based onboarding playbook (`docs/client/ONBOARDING_PLAYBOOK.md`)
  - History cleanliness and release hygiene policy (`docs/client/HISTORY_CLEANLINESS.md`)
  - Public collaboration triage contract (`docs/client/PUBLIC_COLLABORATION_TRIAGE.md`)
  - Claim-evidence policy guard (`docs/client/CLAIM_EVIDENCE_POLICY.md`)
  - GitHub issue templates for bug/feature/security routing (`.github/ISSUE_TEMPLATE/*`)

### Changed
- Repository navigation docs updated to surface launch-polish artifacts:
  - `README.md`
  - `CONTRIBUTING.md`
  - `.github/pull_request_template.md`

### Governance
- Backlog source-of-truth expanded with `V4-FORT-001..006` in `UPGRADE_BACKLOG.md`.
