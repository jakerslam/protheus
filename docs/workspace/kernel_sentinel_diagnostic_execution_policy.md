# Kernel Sentinel Diagnostic Execution Policy

Owner: Kernel Sentinel
Status: canonical policy
Applies to: Kernel Sentinel diagnostic probes, targeted test execution, replay execution, topology checks, evidence refresh

## Purpose

Kernel Sentinel must be able to investigate failures, not only observe them.

This policy allows Sentinel to run tightly-governed diagnostic probes that improve diagnosis confidence while preventing it from becoming a noisy open-ended test runner or an unauthorized self-modification path.

## Core Principle

Sentinel may run diagnostic probes only as bounded evidence-gathering actions tied to a live failure hypothesis.

Allowed shape:

`evidence -> hypothesis -> authorized probe -> confidence update -> stop`

Forbidden shape:

`something looks wrong -> run random tests until something feels explanatory`

## Authority Boundary

Kernel Sentinel owns:

- diagnostic authorization policy
- diagnostic request and result contracts
- probe-class allowlist
- budget enforcement
- stop conditions
- diagnostic receipts and artifacts

Kernel Sentinel does not own:

- open-ended command execution
- broad repository test sweeps
- destructive mutation workflows
- autonomous patch application
- release-gate escalation outside explicit policy

Diagnostic execution is evidence gathering, not repair authority.

## Allowed Probe Families

Sentinel may use only pre-authorized probe families:

1. `diagnostic_topology_probe`
2. `diagnostic_evidence_refresh`
3. `diagnostic_replay`
4. `diagnostic_contract_probe`
5. `diagnostic_test`

Definitions:

- `diagnostic_topology_probe`: read-only topology, health, listener, lifecycle, process, or readiness inspection
- `diagnostic_evidence_refresh`: re-read or refresh a bounded source artifact or truth surface
- `diagnostic_replay`: execute a bounded golden fixture or deterministic replay path
- `diagnostic_contract_probe`: run a bounded contract/invariant check for a named subsystem boundary
- `diagnostic_test`: run an exact targeted regression already mapped to a recognized failure signature

## Canonical Probe Class Contract

These five classes are the only canonical Sentinel diagnostic probe classes.

Each class has a distinct role and may not silently collapse into another class.

### `diagnostic_topology_probe`

Purpose:

- inspect live runtime topology and readiness truth

Allowed actions:

- listener probes
- process/lifecycle checks
- health/readiness checks
- watchdog/topology state reads

Not allowed:

- replay execution
- contract/test execution
- mutating restart/repair behavior

### `diagnostic_evidence_refresh`

Purpose:

- refresh a bounded truth source or source artifact already known to matter

Allowed actions:

- re-read Sentinel source artifacts
- refresh a bounded report or receipt
- refresh a deterministic evidence file

Not allowed:

- topology discovery outside the targeted artifact
- replay/test execution
- broad filesystem sweeps

### `diagnostic_replay`

Purpose:

- re-run a deterministic golden fixture or replay path to confirm a known failure shape

Allowed actions:

- golden fixture execution
- deterministic scenario replay
- bounded replay artifact regeneration

Not allowed:

- open-ended runtime exploration
- unrelated test suites
- performance or soak execution

### `diagnostic_contract_probe`

Purpose:

- check a named boundary, invariant, or contract without broad execution drift

Allowed actions:

- named contract guards
- invariant checks
- bounded policy/receipt consistency checks

Not allowed:

- full workflow execution unrelated to the contract
- shell-led probe selection
- mutation-heavy repair routines

### `diagnostic_test`

Purpose:

- run an exact targeted regression already mapped to a recognized failure signature

Allowed actions:

- exact test filter execution
- exact regression binary execution
- exact scenario-specific test invocation

Not allowed:

- whole-suite sweeps
- “closest match” test guessing
- broad benchmark/perf/stress runs

## Class Invariants

Every canonical probe class must remain:

- enumerable
- auditable
- bounded
- non-overlapping in purpose
- subordinate to runtime truth

Sentinel may compose multiple probes in one run, but each probe step must still declare exactly one canonical class.

If a future diagnostic action does not fit one of these classes cleanly, policy must be updated first instead of silently expanding an existing class.

## Forbidden By Default

Sentinel must fail closed on:

- full-repository test sweeps
- fuzzing without an explicit future policy
- performance or soak suites
- mutation-heavy setup/repair flows
- open-ended shell command generation
- broad benchmark refresh loops
- repeated retries without confidence gain
- any probe requiring broader authority than the current failure class allows

## Hypothesis Requirement

Every diagnostic probe must name:

- the triggering incident or failure signature
- the hypothesis being tested
- the competing explanation it is trying to rule out
- the expected confidence gain

No hypothesis means no probe.

## Budget Rules

Every diagnostic run must enforce bounded budgets.

Canonical default budgets:

- `max_probes_per_incident: 3`
- `max_probe_runtime_seconds: 90`
- `max_total_diagnostic_runtime_seconds_per_run: 240`
- `max_scope_escalation_depth: 2`

Meaning:

- a single incident may trigger at most three probes in one diagnostic run
- no one probe may run longer than ninety seconds
- the full Sentinel diagnostic phase may not exceed four minutes in one run
- Sentinel may escalate at most two steps beyond the cheapest qualifying probe class

Escalation interpretation:

- moving from `diagnostic_topology_probe` to `diagnostic_evidence_refresh` is one step
- moving from `diagnostic_evidence_refresh` to `diagnostic_replay` is one step
- moving from `diagnostic_replay` to `diagnostic_contract_probe` is one step
- moving from `diagnostic_contract_probe` to `diagnostic_test` is one step

Required tracked budgets:

- probe count used
- probe runtime consumed
- total diagnostic runtime consumed
- escalation depth consumed

Budget exhaustion is a stop condition, not a prompt to continue elsewhere.

Policy exception rule:

- tighter budgets are allowed
- broader budgets require an explicit future policy update
- implementation may not silently widen these defaults

## Authorization Model

Every probe must resolve through a deterministic authorization map:

`failure signature -> allowed probe families -> allowed concrete probes`

Sentinel may not invent a probe outside this map.

If no authorized probe exists, the correct result is:

- `probe_unavailable`
- `confidence unchanged`
- `required_next_probe_authoring`

## Fail-Closed Rules

Sentinel must refuse a probe when any of the following are true:

- probe class is not on the allowlist
- failure signature is unmapped
- requested concrete probe is unmapped
- budget would be exceeded
- requested authority exceeds policy
- probe would mutate system state beyond allowed diagnostic scope
- evidence contradiction remains unresolved after bounded attempts

## Stop Conditions

Sentinel must stop diagnostic execution when:

- confidence does not improve after the allowed bounded attempts
- probe budget is exhausted
- the best remaining probes are unauthorized
- gathered evidence remains contradictory without a safe next step
- the incident is already classified confidently enough for action without more probing

Stopping is success when further probing would only add noise.

## Ordering Policy

Sentinel should prefer the cheapest and safest probe that can disambiguate the hypothesis.

Preferred rollout order:

1. `diagnostic_topology_probe`
2. `diagnostic_evidence_refresh`
3. `diagnostic_replay`
4. `diagnostic_contract_probe`
5. `diagnostic_test`

Higher-cost probes should require stronger justification.

## Receipt And Artifact Requirements

Every Sentinel diagnostic execution must persist a diagnostic artifact that records:

- incident identifier
- failure signature
- hypothesis
- selected probe
- authorization reason
- probe result
- confidence before and after
- artifacts consulted or emitted
- stop reason

These artifacts are inputs to:

- issue synthesis
- self-dossier
- RSI handoff
- operator-visible Sentinel reports

## Test Policy

When Sentinel runs tests, they must be:

- exact
- targeted
- pre-authorized
- tied to a live failure signature
- non-destructive within diagnostic scope

Sentinel tests are diagnostic probes, not general validation sweeps.

## Safety Posture

Sentinel diagnostic execution is conservative by policy:

- investigate first
- explain second
- recommend third
- never auto-patch from this policy

Autonomous repair requires a separate stronger authority policy.

## Operator Contract

Operators should be able to see:

- which probes Sentinel ran
- which probes it refused
- why each probe was chosen
- whether confidence increased
- when Sentinel intentionally stopped

The correct goal is traceable diagnosis, not hidden cleverness.

## Acceptance Standard

This policy is satisfied only when future implementation enforces all of the following:

1. Sentinel diagnostic requests are explicit and typed.
2. Probe execution is policy-authorized, not freeform.
3. Probe classes are bounded and enumerable.
4. Budget exhaustion fails closed.
5. No-confidence-gain loops stop automatically.
6. Diagnostic artifacts are persisted and reusable.
7. Exact tests and replays remain subordinate to runtime truth, not replacements for it.
