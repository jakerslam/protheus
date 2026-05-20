# Special-Case Promotion Policy

Status: active repo-wide policy  
Parent doctrine: `docs/workspace/primitive_first_system_doctrine.md`

## Purpose

Special cases are not automatically bad. Hidden special cases in production are bad.

This policy defines how a narrow case becomes a reusable engine capability without poisoning shared primitives.

## Quarantine first

If a case is only needed for one test, eval, fixture, or reproduction, keep it there.

Allowed places:

- `tests/**`
- `validation/evals/**`
- `validation/benchmarks/**`
- `validation/regression/**`
- `**/fixtures/**`
- Golden files and explicit reproduction tests

## Promote when repeated or structural

Promote a case when it:

- Recurs across tasks
- Repairs a real user workflow
- Represents a missing reusable capability
- Affects multiple workflows, tools, or agents
- Is needed for engine reliability rather than one benchmark result

## Promotion targets

Promote into the smallest correct target:

- Primitive workflow or Tool CD
- Composite workflow
- Schema validator
- Policy
- Adapter metadata
- Profile/config pack
- Kernel authority primitive
- Orchestration coordination primitive
- Gateway translation contract
- Shell projection contract

## Anti-pattern

Do not patch special cases into:

- Shared prompt text
- Runtime phrase matching
- Global controllers
- Primitive contracts that become benchmark-shaped
- Shell authority
- Provider examples masquerading as engine behavior

## Completion rule

A promoted special case is not complete until:

- The general primitive or contract is named.
- The production path no longer depends on hidden case identity.
- Fixture-specific details remain in tests/evals.
- The capability is registered when it becomes a reusable engine surface.
- Lower-level dependent paths do not regress.
