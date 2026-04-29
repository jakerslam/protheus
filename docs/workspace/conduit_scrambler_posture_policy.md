# Conduit/Scrambler Posture Policy

Status: Canonical architecture policy
Owner: Jay
Scope: Core, Orchestration Surface, Shell, Gateways, Apps, Packages, and Tests
Effective: April 2026

## Purpose

Conduit makes cross-boundary traffic visible, leased, lifecycle-gated, and receipt-trackable.

Scrambler defines the security posture of that traffic when the route shape itself could expose authority, policy structure, capability schema, or sensitive runtime internals.

This policy prevents "plain transport by accident" while avoiding false claims that quantum-resistant scrambling already exists.

## Core Axiom

Every cross-boundary route uses Conduit.

Every Conduit route declares its security posture.

Sensitive routes use strong Scrambler posture.

Quantum-resistant Scrambler is reserved architecture, not current implementation.

No route may silently downgrade its posture because a caller, Shell surface, adapter, or legacy wrapper finds strong posture inconvenient.

## Required Posture Vocabulary

### Standard Conduit

`standard_conduit` is the baseline for cross-boundary traffic.

It requires:

- explicit source and target Nexus checkpoint surfaces;
- route lease or capability context;
- lifecycle state compatibility;
- schema and verb allowlists;
- bounded payload shape;
- deterministic receipt or auditable receipt reference;
- fail-closed handling for missing policy, lease, lifecycle, or receipt context.

Standard Conduit is not "raw transport." It is the ordinary governed route for bounded low-sensitivity projections and control-plane metadata.

### Strong Scrambler

`strong_scrambler` is required when a route could expose or help reconstruct authority surfaces.

Strong Scrambler requires Standard Conduit plus:

- posture declaration on the route contract;
- schema minimization before crossing the boundary;
- opaque IDs or refs instead of internal object graphs;
- capability, nonce, correlation, and receipt binding;
- replay/downgrade detection;
- redaction of secrets, raw policy internals, raw traces, and raw tool results unless a detail route explicitly authorizes them;
- fail-closed behavior when the required posture cannot be satisfied.

Strong Scrambler in the current architecture is a structural defense posture. It is not a claim of post-quantum cryptography.

### Quantum-Resistant Scrambler

`quantum_resistant_scrambler` is reserved for a future security milestone.

The repo must not claim quantum-resistant scrambling is implemented until a dedicated v1/v2 security milestone defines:

- the cryptographic or protocol primitive;
- threat model;
- downgrade path;
- receipt proof shape;
- performance budget;
- migration plan from strong Scrambler.

Until then, routes that would eventually require quantum-resistant posture must declare `quantum_resistant_reserved=true` and run at `strong_scrambler` at most.

## Route Classes

| Route class | Minimum posture | Notes |
|---|---:|---|
| `display_projection` | `standard_conduit` | Bounded Shell-facing output projections, health labels, UI previews, and visible status rows. |
| `health_status` | `standard_conduit` | Readiness projections from the owner of truth; must not expose raw runtime state. |
| `bounded_search_query` | `standard_conduit` | Search over previews, snippets, counts, and refs. Sensitive scopes upgrade to `strong_scrambler`. |
| `request_ingress` | `standard_conduit` | Shell, CLI, SDK, or adapter request intake. Upgrade to `strong_scrambler` before authority-bearing Core or Orchestration admission. |
| `control_plane_coordination` | `strong_scrambler` | Orchestration Surface to Core/Kernel coordination, sequencing, recovery, tool routing, and workflow stage exchange. |
| `kernel_authority` | `strong_scrambler` | Policy, permission, execution admission, receipt authority, scheduler truth, queue truth, memory authority, and safety transitions. |
| `detail_fetch_sensitive` | `strong_scrambler` | Trace, tool result, artifact body, workflow detail, execution observation, eval detail, or raw diagnostic expansion by ID. |
| `external_agent_or_plugin_ingress` | `strong_scrambler` | Third-party agent, plugin, app, package, or adapter traffic entering authority-bearing routes. |
| `emergency_recovery` | `strong_scrambler` | Recovery, maintenance, quarantine, detach, unlock, and break-glass actions. |
| `reserved_quantum_security` | `strong_scrambler` with `quantum_resistant_reserved=true` | Future v1/v2 route class; current implementation must not overclaim quantum resistance. |

## Mandatory Declarations

Each cross-boundary route contract must declare:

- `route_class`;
- `conduit_security_posture`;
- `source_checkpoint`;
- `target_checkpoint`;
- `lease_or_capability_required`;
- `lifecycle_gate`;
- `receipt_required`;
- `downgrade_allowed`;
- `downgrade_owner`;
- `downgrade_expiry`;
- `quantum_resistant_reserved`.

`downgrade_allowed` must default to `false`.

If a temporary downgrade is approved, it must have an owner, expiry date, replacement plan, receipt trail, and an explicit reason why the route cannot currently meet the stronger posture.

## Downgrade Rules

The following are prohibited:

- silently changing `strong_scrambler` to `standard_conduit`;
- treating Shell convenience, UI latency, or local cache shape as a downgrade reason;
- carrying raw policy internals, execution observations, full traces, full tool outputs, or internal object graphs over a standard route;
- claiming quantum resistance without the future milestone evidence.

The following are allowed:

- standard Conduit for bounded display projections and health/status rows;
- strong Scrambler for sensitive detail fetches by ID;
- explicit temporary downgrade records for migration debt when an owner, expiry, and replacement plan exist.

## Relationship To Other Policies

The Nexus-Conduit-Checkpoint policy defines where routes enter and exit.

The Gateway Ingress/Egress Interface policy defines route classes for ingress, egress, status, detail fetch, and bounded search/query.

The Interface Payload Budget policy defines default payload ceilings.

The Shell UI Projection and Message Detail policies define what the Shell may receive by default and what must be fetched lazily by ref.

This policy defines how sensitive Conduit routes declare and enforce their security posture.

## Version Posture

### Current v0

The current repo must treat `standard_conduit` and `strong_scrambler` as required declaration classes.

Strong Scrambler means structural minimization, opaque references, lease/capability binding, receipt binding, downgrade detection, and fail-closed behavior.

### v1 Security Milestone

The v1 milestone should turn strong Scrambler posture into an executable audit for sensitive Core/Orchestration routes and require route manifests to prove no silent downgrade.

### v2 Security Milestone

The v2 milestone may introduce quantum-resistant Scrambler primitives if the threat model and performance budget justify them.

Until that milestone lands, quantum-resistant Scrambler remains a reserved slot only.

## Enforcement Requirements

Enforcement command: `npm run -s ops:conduit:scrambler-posture:guard`.

Architecture policy governance command: `npm run -s ops:policy-refinement:governance`.

The audit checks `client/runtime/config/conduit_scrambler_posture_contract.json` so sensitive Core/Orchestration routes declare the required posture and do not silently downgrade to standard transport.

The audit fails closed when:

- a sensitive route lacks `conduit_security_posture`;
- a Core/Orchestration authority route declares less than `strong_scrambler`;
- a downgrade is present without owner, expiry, replacement plan, and receipt context;
- any route claims quantum-resistant posture without milestone evidence.

## Practical Placement Test

For any route, ask:

1. Does this cross a module or domain boundary?
2. Which Nexus checkpoint surfaces does it use?
3. Is it only bounded display/status/query projection?
4. Could it expose policy, permission, execution, workflow, memory, trace, or tool internals?
5. If yes, why is it not `strong_scrambler`?
6. Could an external caller infer Core schema or authority structure from this route?
7. What receipt proves the posture used at runtime?

If the posture answer is missing, the route is not compliant.
