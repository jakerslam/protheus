# Primitive Capability Registry Policy

Status: active repo-wide policy  
Parent doctrine: `docs/workspace/primitive_first_system_doctrine.md`

## Purpose

The primitive capability registry is the engine ledger for Infring. It records the reusable primitives, composites, schemas, policies, adapters, profiles, Workflow CDs, and Tool CDs that more specific behavior is allowed to build on top of.

If Infring is an AI engine, this registry is part of its standard library map.

## Registry

Canonical registry:

- `validation/conformance/contracts/primitive_capability_registry.json`

## Required entry shape

Every registered capability must declare:

- `id`
- `name`
- `kind`
- `owner_domain`
- `layer`
- `path`
- `contract_surface`
- `extension_surface`
- `promotion_status`

The point is not bureaucracy. The point is to make every reusable capability nameable, reviewable, and composable.

## Promotion rule

When a special case recurs or becomes important enough to affect production behavior, do not patch it into a shared controller.

Promote it through this path:

1. Identify the underlying general capability.
2. Decide whether it is a primitive, composite, adapter/profile, policy, schema, Tool CD, or Workflow CD.
3. Add or extend the smallest owning contract.
4. Register it in the primitive capability registry.
5. Keep any fixture-specific details in tests/evals.
6. Prove lower-level consumers remain valid before promoting higher-level behavior.

## Monotonic regression rule

A higher-level change must not regress lower-level primitives or stable lower-level eval gates that use the same capability family.

If a higher-level eval fix breaks a lower-level path, classify the failure as:

```text
primitive_composition_boundary_violation
```

Then repair the abstraction boundary before adding more special cases.

## Review blocker

Every production change should answer:

```text
Does this belong in the engine, or should it be content/config/profile/eval-specific behavior built on top of the engine?
```

If the answer is unclear, do not add hidden runtime branches. Add a primitive, contract, policy, adapter/profile config, or eval fixture boundary instead.
