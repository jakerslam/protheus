# Layered Nexus Federation Resolution Policy

Status: Canonical architecture resolution
Owner: Jay
Scope: Core, Orchestration Surface, Shell, Gateway, Apps, Packages, and Tests
Effective: April 2026

## Purpose

This policy resolves the old Layered Nexus Federation ambiguity.

Earlier architecture notes described a three-domain federation runtime with separate Core, Orchestration Surface, and Shell federation authority. The current repository does not implement that exact runtime shape, and it must not be treated as required architecture or cited as completed evidence.

The canonical path is now:

```text
domain internals
  -> explicit Nexus checkpoint surface
  -> Conduit with posture, lease, lifecycle, policy, and receipt context
  -> explicit target Nexus checkpoint surface
  -> target internals
```

Layered federation remains a design intent: every domain relationship must be visible, governable, detachable, and auditable. It is not a mandate to add a second central broker or a missing three-domain federation crate.

## Decision

The exact historical three-domain Layered Nexus Federation runtime shape is retired.

Do not implement a new central federation runtime solely to satisfy the old wording. Do not reintroduce a central Nexus that relays raw payloads, interprets payloads, plans workflows, owns truth, or becomes a broker brain.

The current canonical implementation basis is the Layer 2 Nexus primitive at `core/layer2/nexus`, plus explicit Nexus checkpoint surfaces, Conduit-only cross-boundary transport, route leases/capabilities, lifecycle gates, posture declarations, and receipt evidence.

## Compatibility Meaning

The phrase "Layered Nexus Federation" may remain in historical rows and architecture discussion only as a compatibility label for the auditable relationship model.

When used in new documentation, it means:

- Core, Orchestration Surface, Shell, Gateway, Apps, and Packages have explicit boundary routes.
- Each cross-domain route declares source checkpoint, target checkpoint, Conduit path, posture, lease/capability, lifecycle gate, and receipt trail.
- Default Shell-facing routes expose bounded projections, not full runtime mirrors.
- Heavy details travel by reference through explicit detail routes.
- Central Nexus surfaces authorize, register, revoke, and receipt routes; they do not relay or transform raw payloads.

## Non-Canonical Shapes

The following are retired and must not be introduced without a new explicit architecture decision:

- A separate Core/Orchestration/Shell federation runtime whose only purpose is to satisfy old wording.
- A central broker that carries raw payloads between domains.
- A Nexus that interprets business meaning, user intent, workflow semantics, policy truth, or execution truth.
- A Shell-owned route mirror that replaces Core or Orchestration authority.
- Documentation that marks absent federation files as completed evidence.

## Required Closure Model

Layered Nexus Federation closure is satisfied by:

1. The canonical Nexus-Conduit-Checkpoint policy.
2. The Layer 2 Nexus primitive and its control-plane, lease, lifecycle, template, policy, registry, and receipt surfaces.
3. Shell projection and payload-budget policies that prevent Shell from becoming a runtime mirror.
4. Gateway ingress/egress policies that split request ingress, event egress, health/status, detail fetch, and bounded search.
5. Conduit/Scrambler posture declarations for sensitive routes.
6. A cross-domain route inventory that enumerates real routes and proves each one enters and exits through checkpoint surfaces.

The cross-domain route inventory lives in `docs/workspace/cross_domain_nexus_route_inventory.md` with the machine-readable contract at `client/runtime/config/cross_domain_nexus_route_inventory.json`.

## Evidence Rules

SRS or TODO rows may cite this resolution only when they also cite concrete current evidence:

- `docs/workspace/nexus_conduit_checkpoint_policy.md`
- `docs/workspace/layered_nexus_federation_resolution_policy.md`
- `core/layer2/nexus`
- `ops:nexus:checkpoint-policy:guard`
- `ops:nexus:governance`
- `ops:conduit:scrambler-posture:guard`

Rows must not cite missing retired federation files as done evidence. If a future implementation needs a larger federation layer, it must be introduced as a new architecture item with explicit authority boundaries, payload-bypass prevention, and guard coverage.

## Relationship To Route Inventory

This policy answers "what architecture shape is canonical?"

The cross-domain route inventory answers "which routes currently exist, and are they compliant?"

The initial inventory covers the declared high-value domain routes. Untracked direct cross-boundary paths outside that inventory remain migration debt under the Nexus-Conduit-Checkpoint policy. This retirement decision does not legalize bypasses; it prevents a missing old runtime shape from blocking the current checkpoint architecture.
