# Self-Maintenance Noise Discipline Policy

## Purpose

Self-maintenance recommendations sit downstream of planner degradation, blocked-precondition states, clarification-bound plans, and adapter-fallback-driven plans. If the self-maintenance dedupe and rate-limit window are not tighter than ordinary planner roughness, the layer becomes a noise amplifier instead of a triage tool.

This policy fixes a quantitative discipline so the noise-amplifier failure mode is detectable and gateable.

## Canonical Machine-Readable Form

`surface/orchestration/config/self_maintenance_noise_discipline_policy.json` is the source of truth. This document mirrors it.

## Discipline (Quantitative)

### Rate-Limit Window

Self-maintenance recommendation rate-limit window MUST be at least **2×** the planner's typical clarification-rate window.

- Planner-roughness reference: classification rates emitted in `RuntimeQualitySignals` (clarification candidate count, degraded candidate count, zero-executable candidate state).
- Concrete floor: rate-limit window ≥ **30 minutes** (1,800,000 ms). This is the current `ObserveOnly`-mode window in `self_maintenance/executor.rs`.
- The window MUST NOT be lowered without an accompanying tightening of dedupe scope.

### Dedupe Scope

Recommendations clustered by signature MUST share the same `(component, recommendation_kind, source_observation_signature)` triple. Two recommendations with the same signature inside the rate-limit window are deduped to one emission.

### Critical Severity Bypass

Single-occurrence recommendations from `critical` severity sources may bypass dedupe but MUST still travel through the same `ObserveOnly`-mode emission path so receipts are uniform.

## Fail-Closed Rule

A pull request that:

- lowers the rate-limit window below the 30-minute floor without a written exemption,
- removes signature-based dedupe,
- routes any recommendation through a path that is not `ObserveOnly` mode,

is not admissible.

## CI Guard Contract

`surface/orchestration/src/tool_routing_authority.rs::self_maintenance_noise_discipline_declared` (added alongside this policy) reads:

- the source of `surface/orchestration/src/self_maintenance/executor.rs` and asserts the `ObserveOnly` discriminant + the 30-minute window constant remain present,
- the source of this policy file and the JSON config file and asserts both exist with the discipline tokens declared.

The guard appends a row to the existing `tool_routing_authority_guard_current.json` artifact under the standard checks vec. Any drift (window shortened below floor, dedupe removed, mode lowered to `Active`) fails the guard.

## Integration Notes

- This policy is the V11-EXT-CHATGPT2-001 finding from the second-pass external review (2026-04-26).
- Companion to V11-EXT-CHATGPT-008 (issue-candidate dedupe + threshold) — same noise-amplifier concern, different layer.
- The 2× rule is intentionally simple. If real metrics show the planner clarification-rate window is itself unstable, tighten the floor here rather than weakening the multiplier.
