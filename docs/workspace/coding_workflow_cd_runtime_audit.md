# Coding Workflow CD Runtime Audit

Purpose: track where coding workflow behavior is controlled by JSON/CD versus Rust primitive/runtime code.

## Current boundary

JSON/CD owns:

- coding workflow success criteria
- prompt policy text
- tool menu/capability declarations
- no-fake-success rules
- promotion gates and eval metadata
- final-output evidence expectations

Rust runtime owns:

- native file and command tool implementation
- permission enforcement
- receipt generation
- deterministic command execution
- evidence-gap detection for generic prompt-derived requirements
- eval seeding/judging binaries

## Acceptable Rust primitive lane

Native file/command tools are trusted local-execution primitives. They may remain Rust-native as long as they are permissioned, receipt-backed, fail-closed, and workflow-configurable through declared capability packs and success criteria.

## Migration-debt rule

If changing `coding_project_operator.workflow.json` cannot alter coding workflow interaction behavior without editing Rust, classify it as migration debt unless the behavior is one of:

- primitive tool implementation
- safety/permission enforcement
- schema validation
- deterministic receipt generation
- eval harness implementation

## Current coding-lane action

`missing_product_mutation_receipt` is intentionally implemented as a Rust evidence-gap primitive because it protects the generic native execution substrate from fake success. The workflow JSON owns how that gap is explained and repaired.
