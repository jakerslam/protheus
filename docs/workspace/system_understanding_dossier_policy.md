# System Understanding Dossier Policy

Owner: Kernel Sentinel / Assimilation / RSI planning
Status: canonical policy
Applies to: external assimilation, internal RSI, architectural incident synthesis, major refactors

## Purpose

Assimilation and RSI are two directions of the same capability: understand a system deeply enough to improve, transfer, or repair it without cargo-culting its syntax.

This policy defines the shared worksheet and reference artifact used before agents assimilate an external system or modify InfRing through RSI-like self-improvement. The dossier is not disposable planning text. Once completed, it becomes durable reference data that future agents must consult and update.

## Core Principle

Understand from highest level to lowest level.

The preferred order is:

1. Soul / philosophy
2. Runtime behavior
3. Ecology / operating environment
4. Authority and truth model
5. Architecture and boundaries
6. Capabilities and affordances
7. Failure model and pressure behavior
8. Transfer or improvement plan
9. Implementation structure
10. Syntax and local code details

Syntax is evidence, not the starting point. Files are dissected only after the living system has been studied at higher levels, or when a specific unknown requires code inspection.

## Unified Target Modes

A System Understanding Dossier supports two target modes.

`external_assimilation`: understand another system well enough to decide what capability, workflow, policy, design taste, or runtime mechanism should be assimilated into InfRing.

`internal_rsi`: understand InfRing well enough to detect drift, failures, opportunities, and safe self-improvement plans without losing the system's own soul or violating authority boundaries.

Both modes use the same worksheet. They differ only in final output.

External assimilation output: capability transfer plan.

Internal RSI output: self-improvement, refactor, or invariant-repair plan.

## Dossier As Reference Data

A completed dossier must be stored as a durable artifact and referenced by future work.

Recommended locations:

`docs/workspace/system_understanding/<target_id>_dossier.md` for durable human-readable dossiers.

`local/state/system_understanding/<target_id>_dossier.json` for generated or machine-consumed snapshots.

`local/state/system_understanding/traces/<target_id>/**` for runtime traces, probe logs, and evidence bundles.

Before an agent performs assimilation or RSI work, it must consult the current dossier for the target. If no adequate dossier exists, the task is understanding, not implementation.

If new evidence contradicts an existing dossier, update the dossier before using it to justify code changes.

## Required Dossier Header

Every dossier must include:

```yaml
dossier_id: <stable-id>
target_mode: external_assimilation | internal_rsi
target_system: <name>
target_version_or_revision: <version-or-commit-if-known>
dossier_version: 1
created_at: <iso8601>
updated_at: <iso8601>
owners: [kernel-sentinel, assimilation, rsi]
status: draft | usable | stale | superseded
confidence_overall: 0.0-1.0
blocking_unknowns: []
evidence_index: []
```

## Worksheet Sections

### 1. Soul / Philosophy

Answer what the system is trying to be before describing how it is built.

Required prompts:

- What is the system's purpose?
- What user or world does it assume?
- What does it optimize for?
- What does it protect?
- What does it refuse to do?
- What tradeoffs does it repeatedly choose?
- What feels distinctive or opinionated about it?
- What would count as violating its soul?

Required fields:

```yaml
soul_confidence: 0.0-1.0
soul_evidence: []
soul_unknowns: []
```

### 2. Runtime Behavior

Runtime behavior is the primary evidence when the target can run.

Required prompts:

- What are the primary workflows?
- What happens during startup, steady state, recovery, and shutdown?
- What are the core loops?
- What state changes are observable?
- What does the system do under pressure?
- Which runtime observations contradict docs or architecture claims?

For external systems, agents should run probes where possible instead of relying on static code reading.

For InfRing, the dossier should use built-in traces, receipts, Sentinel evidence, gateway lifecycle data, orchestration traces, and Kernel artifacts.

Required fields:

```yaml
runtime_confidence: 0.0-1.0
runtime_evidence: []
runtime_unknowns: []
required_next_probes: []
```

### 3. Ecology / Operating Environment

Required prompts:

- What ecosystem does the system live in?
- What dependencies shape its behavior?
- What external services, APIs, models, filesystems, terminals, browsers, or processes does it assume?
- What pressures shaped it: latency, safety, cost, autonomy, UX, portability, security?
- What threat model is implied by behavior?

Required fields:

```yaml
ecology_confidence: 0.0-1.0
ecology_evidence: []
ecology_unknowns: []
```

### 4. Authority / Truth Model

This section is mandatory before any transfer or self-improvement plan.

Required prompts:

- Which subsystem owns truth?
- Which subsystem coordinates without owning truth?
- Which subsystem presents or collects input?
- Which subsystem owns external boundaries?
- Where can authority re-emerge through data shape, lifecycle affordance, fallback paths, or compatibility shims?
- Which parts are authority-shaped residue even if the old syntax was removed?
- What would prove that authority is misplaced?

Required fields:

```yaml
authority_confidence: 0.0-1.0
authority_evidence: []
authority_unknowns: []
authority_risks: []
```

### 5. Architecture and Boundaries

Architecture is a theory that must be checked against runtime evidence.

Required prompts:

- What are the major organs?
- What are the boundaries?
- What data crosses those boundaries?
- What lifecycle states exist?
- What does the declared architecture say?
- What does runtime behavior prove instead?
- Where do architecture and runtime disagree?

Required fields:

```yaml
architecture_confidence: 0.0-1.0
architecture_evidence: []
architecture_unknowns: []
runtime_architecture_mismatches: []
```

### 6. Capability and Affordance Map

Required prompts:

- What capabilities does the system actually have?
- Which capabilities are raw runtime mechanics?
- Which capabilities are workflows?
- Which capabilities are policy or governance ideas?
- Which capabilities are UX/presentation ideas?
- Which capabilities fit InfRing's soul?
- Which capabilities conflict with InfRing's authority model?
- Which capabilities are incidental and should not be copied?

Required fields:

```yaml
capability_confidence: 0.0-1.0
capabilities: []
rejected_capabilities: []
capability_unknowns: []
```

Each capability row should include:

```yaml
id: <capability-id>
kind: raw_runtime | workflow | policy | ux | architecture | tooling | evidence
value: low | medium | high | critical
evidence: []
runtime_proof: []
transfer_target: kernel | orchestration | shell | gateway | workflow_json | docs | tests | reject
fit_rationale: <why this belongs or does not belong>
```

### 7. Failure Model

Required prompts:

- How does the system fail?
- What does failure look like at event, component, boundary, policy, architecture, and self-model levels?
- Which failures are symptoms?
- Which failures imply a violated invariant?
- Which failures require stopping local patching and reframing the architecture?

Required fields:

```yaml
failure_model_confidence: 0.0-1.0
known_failure_modes: []
violated_invariants: []
stop_patching_triggers: []
```

### 8. Transfer or Improvement Plan

External assimilation plans must map capabilities into InfRing.

Internal RSI plans must map observed gaps into safe improvements.

Required prompts:

- What should be changed or assimilated?
- What should be rejected?
- Which layer owns the change?
- Which invariant does the change strengthen?
- What proof would show success?
- What rollback or containment path exists?

Required fields:

```yaml
transfer_confidence: 0.0-1.0
implementation_items: []
proof_requirements: []
rollback_plan: []
```

### 9. Implementation Structure

Only inspect implementation structure after higher-level sections have enough evidence, or when a blocking unknown requires code inspection.

Required prompts:

- Which files or modules implement the observed behavior?
- Which structures are essential?
- Which structures are incidental?
- Which structures are compatibility residue?
- Which structures create authority-shaped cavities?

Required fields:

```yaml
implementation_confidence: 0.0-1.0
files_inspected: []
implementation_unknowns: []
```

### 10. Syntax / Local Detail

Syntax is the lowest-priority layer. It is used to implement or verify a mapped capability, not to discover the system's soul.

Required prompts:

- What exact code mechanism matters?
- Why is this mechanism necessary?
- Which higher-level behavior does it support?
- What proof connects the syntax to the behavior?

Required fields:

```yaml
syntax_confidence: 0.0-1.0
syntax_evidence: []
syntax_unknowns: []
```

## Confidence Gates

Agents must record confidence for each section. Low confidence is acceptable only when unknowns and next probes are explicit.

Default thresholds before implementation:

- Soul / philosophy: 0.60 minimum
- Runtime behavior: 0.70 minimum when target can run
- Authority / truth model: 0.80 minimum
- Architecture and boundaries: 0.70 minimum
- Capability map: 0.70 minimum
- Transfer or improvement plan: 0.80 minimum

If a threshold is not met, the next task is to gather evidence or run probes, not to implement.

## Probe Requirements

When runtime access exists, use runtime evidence before static file burn-down.

External target probes may include:

- install/run smoke test
- primary workflow capture
- startup/shutdown trace
- error/recovery trace
- API route exercise
- process and lifecycle observation
- log and receipt capture
- stress or failure scenario

Internal InfRing probes may include:

- Kernel Sentinel reports
- lifecycle receipts
- gateway/watchdog state
- orchestration decision traces
- Shell projection state
- proof-pack artifacts
- runtime route probes
- process ownership observations

## Stop-Patching Rule

If symptoms span multiple layers, agents must pause local patching and complete or update the dossier's failure model.

Stop-patching triggers include:

- Shell state contradicts Gateway or Kernel state.
- Gateway command success contradicts listener/process reality.
- API route behavior contradicts request semantics.
- Watchdog observes duplicate or stale ownership.
- A hydration or boot-critical route depends on heavy mutable runtime state.
- The same root symptom family appears in three or more layers.
- Local fixes improve one symptom while preserving the architectural contradiction.

When triggered, produce an architectural incident synthesis before further code changes.

## Relationship To Sentinel

Kernel Sentinel should use this policy as the framework for big-picture incident synthesis.

Sentinel should classify failures by level:

- `L0_local_defect`
- `L1_component_regression`
- `L2_boundary_contract_breach`
- `L3_policy_truth_failure`
- `L4_architectural_misalignment`
- `L5_self_model_failure`

Sentinel should prefer root-frame issue candidates over endpoint-only issue candidates when evidence crosses boundaries.

## Relationship To Assimilation

Assimilation must not be ledger burn-down. A file can only be assimilated when it maps to a capability, behavior, or design principle captured in the dossier.

No file-level assimilation row should be completed unless it answers:

- Which higher-level capability does this support?
- What runtime or architectural evidence proves it matters?
- Why does this fit InfRing?
- Where does it belong?
- How will we prove it after transfer?

## Relationship To RSI

RSI work must not start from local code edits. It starts from InfRing's self-dossier.

A self-improvement proposal must state:

- Which part of InfRing's soul it preserves or strengthens.
- Which runtime behavior proves the gap.
- Which invariant failed or can be improved.
- Which layer owns the change.
- Why the change is structural rather than symptom patching.
- How the system will observe the outcome.

## Minimal Dossier Template

```markdown
# System Understanding Dossier: <Target>

## Header
<yaml header>

## Soul / Philosophy

## Runtime Behavior

## Ecology / Operating Environment

## Authority / Truth Model

## Architecture and Boundaries

## Capability and Affordance Map

## Failure Model

## Transfer or Improvement Plan

## Implementation Structure

## Syntax / Local Detail

## Unknowns And Next Probes

## Evidence Index
```

## Enforcement Summary

Dossier first. Implementation second.

If the dossier is missing, stale, or below confidence thresholds, the correct work is system understanding.

If a local bug cluster implies a policy or architecture failure, stop patching and synthesize the violated invariant.

If a file or mechanism cannot be tied back to soul, runtime behavior, capability, or invariant repair, do not assimilate or modify it.
