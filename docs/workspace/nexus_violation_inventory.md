# Nexus Violation Inventory

Status: Canonical working inventory
Owner: Jay
Scope: Non-Shell Nexus / Conduit / checkpoint violations, exemptions, and proof gaps
Updated: 2026-05-01

## Purpose

This document is the live inventory of known non-Shell violations against the Nexus Checkpoint policy.

Use it to answer:

- what active hard violations currently exist;
- what temporary exemptions still permit direct non-Nexus coupling;
- what enforcement gaps still prevent the repo from claiming full Nexus closure.

Shell-specific boundary debt is intentionally excluded from this document while the Shell purge is in flight. Shell violations should stay in Shell-specific guards and debt ledgers until the Shell authority purge closes.

## Canonical Rule

The target rule is simple:

- every cross-module or cross-domain relationship must enter through an explicit source Nexus checkpoint surface;
- travel over an explicit Conduit;
- exit through an explicit target Nexus checkpoint surface;
- carry lease/capability, lifecycle, posture, policy, and receipt context.

Direct cross-boundary code paths outside that shape are violations or explicit migration debt. They are not accepted steady-state architecture.

Primary policy: [nexus_conduit_checkpoint_policy.md](/Users/jay/.openclaw/workspace/docs/workspace/nexus_conduit_checkpoint_policy.md)

## Current Snapshot

- revision: `9377febd38c0dda80a86647ee988b876eb93e7c0`
- generated_at:
  - kernel nexus coupling guard: `2026-05-01T04:31:01.148Z`
  - architecture boundary conformance: `2026-05-01T04:28:56.525Z`
  - gateway boundary guard: `2026-05-01T03:25:41.051Z`
- shell coupling intentionally excluded from this inventory
- temporary exemptions remaining: `5`
- closed this wave: `core/layer1/primitives` no longer depends on `core/layer1/provenance` for receipt types; both now use the shared `infring_types` receipt contract.

## Active Hard Violations

Current non-Shell hard violations detected by the enforced guards:

- `none`

Evidence:

- [kernel_nexus_coupling_guard_current.json](/Users/jay/.openclaw/workspace/core/local/artifacts/kernel_nexus_coupling_guard_current.json)
  - `import_violations: 0`
  - `cargo_path_violations: 0`
  - `expired_exemptions: 0`
  - `stale_exemptions: 0`
- [arch_boundary_conformance_current.json](/Users/jay/.openclaw/workspace/core/local/artifacts/arch_boundary_conformance_current.json)
  - `hard_violation_count: 0`
- gateway boundary proof:
  - `ops:gateway-boundary:guard` passed with `pass: true`

## Explicit Migration Debt / Temporary Exemptions

These are not hard-pass violations today because they are explicitly allowlisted, owned, dated, and tied to replacement Nexus extraction work. They still count as architectural debt.

All current exemptions below are owned by `jay`, expire on `2026-06-15`, and cite the same replacement plan:

- replacement plan: `infring_nexus_core_v1 contract extraction`

### Kernel Memory Runtime

1. `core/layer0/memory_runtime/Cargo.toml` -> `../../layer2/memory`
   - reason: memory runtime depends on Layer 2 memory heap while Nexus replacement contract is extracted

2. `core/layer0/memory_runtime/Cargo.toml` -> `../../layer1/memory_runtime`
   - reason: memory runtime depends on Layer 1 recall policy while Nexus replacement contract is extracted

### Kernel Red Legion

3. `core/layer0/red_legion/Cargo.toml` -> `../../layer1/observability`
   - reason: red legion chaos receipts depend on observability report types pending shared contract extraction

### Kernel Vault

4. `core/layer0/vault/Cargo.toml` -> `../memory`
   - reason: vault policy embeds memory-owned policy blobs pending shared contract extraction

### Kernel Observability

5. `core/layer1/observability/Cargo.toml` -> `../../layer0/memory`
   - reason: observability profile embeds memory-owned profile blobs pending shared contract extraction

Canonical exemption source:

- [kernel_nexus_coupling_policy.json](/Users/jay/.openclaw/workspace/tests/tooling/config/kernel_nexus_coupling_policy.json)

## Remaining Enforcement Gaps

These are not active hard violations, but they still block a full claim of universal Nexus closure:

1. Repo-wide proof is incomplete.
   - The policy itself still states that the repo does not yet have complete proof that every module, Gateway, and Orchestration surface routes only through Nexus checkpoint surfaces.
   - Source: [nexus_conduit_checkpoint_policy.md](/Users/jay/.openclaw/workspace/docs/workspace/nexus_conduit_checkpoint_policy.md)

2. Current proof is strongest in Kernel, not universal across every domain.
   - Kernel coupling is strongly guarded.
   - Architecture import rules are guarded.
   - Gateway boundary behavior is guarded.
   - Full “every cross-boundary route is Nexus-only” proof is still broader than the currently passing guard set.

3. Shell is excluded from this inventory for now.
   - This is intentional, not closure.
   - Shell-specific debt must not be mistaken for resolved Nexus closure.

## Enforcement Commands

Use these as the canonical non-Shell Nexus enforcement commands:

- `npm run -s ops:nexus:kernel-coupling:guard`
- `npm run -s ops:arch:conformance`
- `npm run -s ops:gateway-boundary:guard`
- `npm run -s ops:nexus:route-inventory:guard`
- `npm run -s ops:nexus:governance`

## Enforcement Rule For This Inventory

Update this document whenever any of the following changes:

- a new hard non-Shell Nexus violation appears;
- an exemption is added, removed, or expires;
- a replacement Nexus extraction closes one of the listed debts;
- repo-wide proof scope materially improves.

If a guard reports zero hard violations but this document still lists resolved debt, update the inventory in the same change that closes the debt.

## Closure Target

This inventory is complete only when all of the following are true:

- active hard violations = `0`
- temporary exemptions = `0`
- repo-wide proof gap = closed
- Shell has its own closure and can be folded back into universal Nexus proof without special exclusion

Until then, “Nexus-only architecture” is the law, and this inventory is the list of remaining exceptions to that law.
