# Reliability Floor Policy

Status: active policy under the Three Operating Laws.

The reliability floor is the smallest set of paths that must stay boringly dependable for InfRing to be useful.

## Required Paths

- install and repair
- gateway start, status, restart, and recovery
- request-to-response execution
- CodeQL/security alert remediation
- Sentinel finding promotion
- TODO lifecycle and archive flow
- commit, push, release, and version hygiene

## Rules

- A failure on a floor path must have bounded timeout behavior.
- A failure on a floor path must emit a small diagnostic artifact or concrete next action.
- A floor path must not depend on legacy Shell authority.
- A floor path may use Shell projection, but must have a headless or CLI proof path.
- Sentinel may flag recurring floor-path failures as `reliability` law violations.

## Sentinel Expectations

Sentinel findings against the reliability floor should include:

- violated law: `reliability`
- evidence refs
- owner guess
- root-cause hypothesis
- bounded falsification probe
- concrete next action
- release or TODO impact

## Current Priority

The current highest-priority floor failures are:

- Kernel receipt drift and stale receipt evidence confusion
- recurring empty assistant responses in synthetic user harnesses
- stale deterministic Kernel evidence blocking RSI readiness
