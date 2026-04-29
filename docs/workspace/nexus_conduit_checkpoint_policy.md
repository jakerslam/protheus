# Nexus-Conduit-Checkpoint Policy

Status: Canonical architecture policy
Owner: Jay
Scope: Core, Orchestration Surface, Shell, Gateways, Apps, Packages, and Tests
Effective: April 2026

## Purpose

The Nexus-Conduit-Checkpoint policy makes every cross-boundary relationship visible, governable, detachable, and auditable.

The system must not grow hidden code-to-code dependency webs. When one subsystem talks to another subsystem, that relationship must pass through an explicit Nexus checkpoint surface, travel over Conduit, and carry policy, lease, lifecycle, and receipt context.

This policy is the repo source of truth for the architecture previously described as the hierarchical Nexus routing system, breaker-box routing, Nexus domains, and conduit-backed checkpoints.

## Core Axiom

Modules do work.

Nexuses manage relationships.

Conduits carry traffic.

Kernel policy decides what is allowed.

Receipts prove what happened.

Shell shows and collects; it does not own truth, permission, routing authority, or durable runtime state.

Shell UI projection boundaries are canonicalized in `docs/workspace/shell_ui_projection_policy.md`.

Shell UI message/detail payload boundaries are canonicalized in `docs/workspace/shell_ui_message_detail_contract.md`.

Gateway ingress/egress route class boundaries are canonicalized in `docs/workspace/gateway_ingress_egress_policy.md`.

Interface payload budgets are canonicalized in `docs/workspace/interface_payload_budget_policy.md`.

Shell-independent operation is canonicalized in `docs/workspace/shell_independent_operation_policy.md`.

Conduit/Scrambler security posture is canonicalized in `docs/workspace/conduit_scrambler_posture_policy.md`.

Layered Nexus Federation resolution is canonicalized in `docs/workspace/layered_nexus_federation_resolution_policy.md`; the old exact three-domain federation runtime shape is retired in favor of Layer 2 Nexus primitives plus explicit checkpoint, Conduit, lease, lifecycle, posture, and receipt guards.

Cross-domain Nexus route inventory is canonicalized in `docs/workspace/cross_domain_nexus_route_inventory.md`; declared routes must name source checkpoint, target checkpoint, Conduit path, posture, lease/capability, lifecycle gate, owner of truth, and receipt requirement.

## Required Definitions

### Module

A module is any coherent subsystem that owns local implementation details, local computation, local data structures, and local business logic.

Examples include Kernel primitives, memory, execution, web, task fabric, orchestration coordination surfaces, Shell UI surfaces, gateways, and app wrappers.

### Nexus

A Nexus is a logic-light connection surface. It owns cross-boundary relationship metadata, route registration, route eligibility, lifecycle state, lease validation, and receipt emission.

A Nexus must not own business logic, truth-bearing state, semantic payload interpretation, workflow planning, prompt logic, or payload transformation.

### Nexus Checkpoint Surface

A Nexus checkpoint surface is the only valid cross-boundary contact point for a module.

It can be implemented as a file, module, crate export, endpoint contract, or generated manifest, but it must be explicit, named, auditable, and guardable.

For code placement, "checkpoint file" means the module-local Nexus-facing file or exported surface that all external relationships enter or exit through. Internal files may call each other freely inside the module, but external modules must not bypass the checkpoint surface.

### Conduit

Conduit is the sole transport primitive for authorized cross-boundary traffic. Thin wrappers may exist, but they must delegate to Conduit rather than invent a second transport abstraction.

### Route Lease

A route lease is the only authorization mechanism for cross-module delivery. A lease must be bounded to source, target, allowed schemas/verbs, trust or Verity class, TTL, receipt lineage, lifecycle compatibility, and policy decision reference.

### Scrambler

Scrambler is the security posture applied to sensitive conduit paths. Strong Scrambler is required for authority-bearing Core or Kernel to Orchestration Surface routes and for sensitive detail, recovery, policy, permission, execution, trace, tool, and external-agent ingress routes. The future quantum-resistant Scrambler is deferred until an explicit v1/v2 security milestone; the architecture still reserves that slot without claiming implementation.

The canonical posture vocabulary, route classes, downgrade rules, and quantum-resistance deferral notes live in `docs/workspace/conduit_scrambler_posture_policy.md`.

## Non-Negotiable Rules

1. No direct cross-module code path may bypass a Nexus checkpoint surface.
2. No cross-boundary traffic may bypass Conduit.
3. No Shell path may become authority for policy, permission, truth, execution admission, durable runtime state, or receipt authority.
4. No Orchestration Surface path may become authority for canonical truth, final execution admission, or receipt authority.
5. No Nexus may interpret payload meaning, infer user intent, plan workflows, transform truth-bearing content, or act as a broker brain.
6. No main or central Nexus may relay raw payloads as a payload transport. It may authorize direct delivery, issue leases, and emit receipts.
7. Every cross-boundary route must be revocable through lifecycle or policy state.
8. Every cross-boundary route must declare Conduit/Scrambler posture and must fail closed if a required strong posture silently downgrades to standard transport.
9. Every registration, lease issue, lease renewal, lease revocation, template instantiation, lifecycle transition, conduit attach/detach, route posture decision, posture downgrade, and route denial must be receipt-trackable.
10. Legacy exceptions must be explicit, dated, owned, and tied to a replacement Nexus checkpoint plan.
11. Documentation must not claim a Nexus domain or route is done unless the referenced files and guards exist in the current repo.

## Boundary Shape

### Inside a Module

Internal implementation is local. Files inside one module may organize themselves however needed.

### Between Modules

All cross-module interaction must enter through the source module's Nexus checkpoint surface and exit through the target module's Nexus checkpoint surface.

The allowed flow is:

```text
source module internals
  -> source Nexus checkpoint surface
  -> Conduit with policy/lease/receipt context
  -> target Nexus checkpoint surface
  -> target module internals
```

The Conduit segment must declare whether the route uses `standard_conduit`, `strong_scrambler`, or a reserved future quantum-resistant posture marker. Sensitive authority-bearing routes must use `strong_scrambler` until a stronger audited posture exists.

### Between Domains

The system has three top-level authority domains:

- Kernel or Core domain: truth, permission, canonical state, execution admission, and receipts.
- Orchestration Surface domain: non-canonical coordination, sequencing, clarification, recovery, and packaging.
- Shell domain: presentation, input collection, local UX state, and user-visible controls.

Cross-domain routing must go through the central Nexus checkpoint surfaces for those domains. Direct Shell to Core paths are prohibited unless an explicitly approved ingress contract exists, and even then the path must be conduit-backed, lease-checked, and receipt-trackable.

Shell-facing routes must expose bounded projections by default. Detail fetches for traces, raw tool outputs, workflow internals, or execution observations must travel through explicit gateway/conduit routes by reference rather than being bundled into default Shell state.

Default cross-boundary responses must declare bounded byte, array, depth, string, cursor, detail-ref, audit, and Nexus ceilings before they enter Shell-facing state.

Core, Orchestration Surface, CLI, and Gateway status must not require browser Shell assets to build or operate.

Sensitive Core/Orchestration and Gateway detail routes must declare `strong_scrambler` posture unless a dated migration exception explicitly documents owner, expiry, replacement plan, and receipt trail.

## Allowed Gate Dimensions

Nexus route gates may use only:

- lifecycle state,
- enabled or disabled state,
- source and target registration,
- lease validity,
- schema and verb allowlists,
- trust, Verity, or policy class,
- source and target domain,
- control-plane-only versus payload-allowed status,
- explicit operator-approved maintenance or quarantine state.

## Disallowed Gate Dimensions

Nexus route gates must not use:

- semantic payload interpretation,
- business meaning,
- truth of claims,
- user intent inference,
- workflow planning,
- prompt content analysis,
- LLM judgment,
- UI convenience state,
- hidden Shell state.

## Lifecycle Semantics

Every Nexus checkpoint surface must be able to express lifecycle status through explicit state rather than anonymous booleans.

Required lifecycle states are:

- active: allows normal traffic and new lease issuance.
- draining: refuses new leases or renewals, but may allow in-flight completion until a deadline.
- quiesced: allows no payload traffic.
- detached: revokes leases and removes routing presence.
- maintenance: allows only policy-approved control-plane schemas and verbs; payload delivery remains disabled.

## Implementation Status Contract

The current repo has a real Layer 2 Nexus primitive at `core/layer2/nexus`.

The old exact Core/Orchestration/Shell federation runtime shape is retired by `docs/workspace/layered_nexus_federation_resolution_policy.md`. Layered federation now means auditable cross-domain route relationships, not a separate central raw-payload broker runtime.

The initial cross-domain route inventory lives in `client/runtime/config/cross_domain_nexus_route_inventory.json` and is guarded by `ops:nexus:route-inventory:guard`.

The current repo has partial enforcement through Nexus coupling, import/export boundary, architecture boundary, and Sentinel evidence guards.

The current repo does not yet have complete repo-wide proof that every module, Shell surface, gateway, and orchestration surface routes only through Nexus checkpoint surfaces. Until that proof exists, any direct cross-boundary imports or calls outside guarded exceptions are migration debt, not accepted architecture.

## Enforcement Requirements

At minimum, enforcement must verify:

- this policy document exists and contains the canonical axioms;
- `docs/workspace/codex_enforcer.md`, `docs/workspace/orchestration_ownership_policy.md`, `docs/client/architecture/LAYER_RULEBOOK.md`, and `docs/SYSTEM-ARCHITECTURE-SPECS.md` link to this policy;
- `ops:nexus:governance` includes a policy guard;
- `ops:nexus:governance` includes the cross-domain route inventory guard;
- `ops:arch:governance` includes `ops:policy-refinement:governance`;
- `ops:policy-refinement:governance` runs the Shell projection, Shell UI message/detail, Gateway interface, Interface Payload Budget, Shell amputation, Conduit/Scrambler posture, and cross-domain Nexus route inventory guards together;
- the current Nexus source root matches the filesystem;
- the cross-domain route inventory exists and covers required domains and route classes;
- SRS rows do not cite missing Nexus implementation paths as done evidence;
- no new cross-module bypass is accepted without an explicit owner, expiry, and replacement Nexus checkpoint plan.

## Practical Placement Test

For any new or modified call path, ask:

1. Is this call crossing a module or domain boundary?
2. If yes, what is the source Nexus checkpoint surface?
3. What is the target Nexus checkpoint surface?
4. What Conduit path carries it?
5. What lease or capability authorizes it?
6. What lifecycle state can block it?
7. What receipt proves it?
8. If the Shell vanished, would Core and Orchestration still operate through the same authoritative route?

If any answer is missing, the path is not compliant.
