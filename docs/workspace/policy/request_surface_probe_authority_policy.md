# Request Surface Probe Authority Policy

## Purpose

Distinguish probe-authoritative request surfaces from legacy compatibility surfaces, so non-legacy lanes can never silently degrade into payload shortcuts or heuristic inference. The orchestration planner has two planning modes — authoritative (typed probes) and heuristic (compatibility tail). This policy makes that split explicit and enforceable.

## Canonical Machine-Readable Form

`surface/orchestration/config/request_surface_probe_authority_policy.json` is the source of truth. This document is a human-readable mirror; if the two diverge, the JSON wins.

## Scope

Applies to every planning decision in `surface/orchestration/src/planner/**` and `surface/orchestration/src/sequencing.rs` that depends on capability availability, transport availability, or policy admission.

## Two-Tier Contract

### Adapted Surfaces (Probe-Authoritative)

`RequestSurface::Sdk`, `RequestSurface::Gateway`, `RequestSurface::Dashboard`, and `RequestSurface::TypedCli` are probe-authoritative:

- Capability availability, transport availability, and policy admission MUST be derived from `CoreProbeEnvelope` or `CoreExecutionObservation` only.
- Payload probe shortcuts (e.g., reading `request.payload["transport_available"]`) are forbidden.
- Heuristic probe fallback (e.g., inferring transport availability from resource/operation hints) is forbidden.
- Missing required probes fail closed with `missing_probe: <capability>.<field>` and `probe.required_for_typed_surface.<capability>.<field>` diagnostics. The planner does not infer.

### Compatibility Lane (Legacy)

`RequestSurface::Legacy` is the only lane allowed to use payload probe shortcuts or heuristic inference. Legacy is bounded:

- Required marker: `RequestSurface::Legacy`. Any heuristic or shortcut path must be gated behind this discriminant.
- Compatibility burndown is tracked separately under V11-EXT-CHATGPT-003 (compatibility-tail budget).

## Implementation Anchors

| Concern | Anchor |
|---|---|
| Surface-tier classifier (single source of truth) | `allow_payload_probe_shortcuts(request)` and `allow_heuristic_probe_fallback(request)` in `surface/orchestration/src/planner/preconditions.rs` (lines 16–22). Both return `true` only for `RequestSurface::Legacy`. |
| Probe-envelope read | `envelope_probe_bool(request, capability, field)` in `preconditions.rs` (lines 46–78). |
| Heuristic source label | `heuristic.transport_hints_or_operation` (legacy-only). |
| Missing-probe diagnostic source | `missing_probe: <capability>.<field>` (typed-surface refusal). |
| Required-probe diagnostic | `probe.required_for_typed_surface.<capability>.<field>`. |

## Fail-Closed Rule

Any planner code path that runs for a non-legacy `RequestSurface` and that depends on capability/transport/policy state MUST fail closed when its probe is missing rather than fall back to a payload read or heuristic.

A planning decision is not admissible when:
- it depends on `request.payload[...]` for capability state on a non-legacy surface,
- it inferes capability state from resource/operation hints on a non-legacy surface,
- it silently treats a missing probe as a positive signal,
- or a new `RequestSurface` discriminant is added without an explicit policy entry.

## CI Guard Contract

Two coordinated guards live in `surface/orchestration/src/tool_routing_authority.rs`:

- `planner_payload_decision_audit_enforced` (line 977) — counts `request.payload` reads in each planner/sequencing source file. Non-legacy files must have zero. Covers `plan_candidates.rs`, `plan_candidates/common.rs`, `plan_candidates/chain.rs`, `plan_candidates/strategy.rs`, and `sequencing.rs`. `preconditions.rs` is allowed reads behind the legacy gate.
- `heuristic_probe_fallbacks_compatibility_fenced` (line 558) — asserts the heuristic fallback function exists, gates on `RequestSurface::Legacy`, and that the heuristic source labels (`heuristic.policy_scope_and_mutability`, `heuristic.transport_hints_or_operation`) appear only inside the legacy fence.
- `request_surface_probe_authority_policy_declared` (line 586) and `request_surface_probe_authority_policy_semantics_declared` (line 613) — assert the JSON config exists and covers every adapted surface plus the legacy lane with the right semantics.

Artifacts:
- `core/local/artifacts/tool_routing_authority_guard_current.json`
- `local/workspace/reports/TOOL_ROUTING_AUTHORITY_GUARD_CURRENT.md`

The operator summary's `release_blocking` flag must remain wired into the release verdict path.

## Adding a New Adapted Surface

1. Add the `RequestSurface` discriminant in `contracts.rs`.
2. Update `request_surface_probe_authority_policy.json` `adapted_surfaces` with `probe_authoritative: true`, `payload_probe_shortcuts_allowed: false`, `heuristic_probe_fallback_allowed: false`, and the two `missing_*_probe_behavior` fields set to refuse.
3. Confirm the new surface is NOT added to `allow_payload_probe_shortcuts` or `allow_heuristic_probe_fallback`.
4. Run `tooling:run` against the tool-routing-authority guard and confirm `request_surface_probe_authority_policy_declared` and `_semantics_declared` both pass.

## Adding a New Legacy Surface

Don't. The legacy lane is closed for new entrants. New external integrations land on an adapted surface or are blocked.

## Integration Notes

- This policy complements, but does not replace, the planner's existing precondition contract.
- The compatibility tail (legacy-lane usage in production) is governed by the burn-down budget tracked under V11-EXT-CHATGPT-003.
- Probe schema additions (new fields on `CapabilityProbeSnapshot`) require updates to `canonical_probe_fields` in the JSON config and a corresponding `envelope_probe_bool` arm in `preconditions.rs`.
