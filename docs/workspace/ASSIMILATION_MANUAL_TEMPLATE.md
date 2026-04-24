# Manual Assimilation Template (Map + Ledger + Queue)

Use this template when assimilation is manual and we need clear structure, prioritization, and execution discipline.

## Related Docs

- Wave execution runbook: `docs/workspace/CODEX_ASSIMILATION_WAVE_RUNBOOK.md`
  - Use this template to design and prioritize assimilation.
  - Use the runbook to execute 4-8 file waves from the active queue.

## Core Concept

Do not ingest target repositories blindly. Assimilation runs through a three-layer system:

1. `Assimilation Map` (strategic)
2. `Priority Ledger` (tactical)
3. `Active Queue` (execution)

This keeps quality high, reduces churn, and makes decisions auditable.

## Three-Layer Model

### 1) Assimilation Map (Strategic / Coordination)

Purpose:

- map target architecture/components/functions to InfRing architecture
- identify value, compatibility, and risk
- define integration strategy per subsystem

Key question:

- what is worth taking, and how should it fit into InfRing?

### 2) Priority Ledger (Tactical / Prioritization)

Purpose:

- ordered backlog derived from the Assimilation Map
- rank by impact, urgency, dependency, and risk
- provide one tactical source of truth for upcoming work

Key question:

- what should be assimilated next?

### 3) Active Queue (Execution)

Purpose:

- small, focused set pulled from top of Priority Ledger
- keeps in-flight work manageable and verifiable
- only items in this queue are actively executed

Key question:

- what are we doing right now?

## Canonical Folder Structure

Create this exact structure for each new target:

```text
[Target-Name]-Assimilation/
├── target-repo/                  ← Full clone of target repo
├── assimilation-map.md           ← Strategic mapping and analysis
├── priority-ledger.md            ← Prioritized assimilation backlog
├── active-queue.md               ← In-flight execution items
├── decisions-log.md              ← Accept/reject decisions and rationale
├── integration-notes.md          ← Mapping into InfRing components
├── source-burn-down.tsv          ← Source-file burn-down tracker (mandatory)
└── archive/                      ← Closed snapshots/maps/ledgers
```

Trade-secret note:

- The `[Target-Name]-Assimilation/` working folder should be gitignored by default.
- Do not commit target-repo clones or sensitive assimilation working sets.

## Workflow Rules

1. Update `assimilation-map.md` whenever understanding changes.
2. Treat `priority-ledger.md` as the single tactical source of truth.
3. Keep `active-queue.md` small (`5-15` items maximum).
4. Execute only from `active-queue.md`.
5. On completion, move closed sections/snapshots to `archive/`.
6. Record every accept/reject in `decisions-log.md` with reason.
7. Record integration consequences in `integration-notes.md`.
8. Burn down source files as they are assimilated:
   - update `source-burn-down.tsv` in the same wave
   - every assimilated source file must reach `status=burned_down`
   - when using physical burn-down, move to `target-repo/.assimilation_deleted/<path>` and record `deleted_to`

## Source Burn-Down Contract

Purpose:

- prevent repeated reprocessing of already-assimilated source files
- make assimilation progress auditable at source-file granularity
- keep active intake surface shrinking over time

Required status progression:

- `queued` -> `in_review` -> `assimilated` -> `burned_down`

Required fields:

- `source_path`
- `status`
- `first_batch`
- `last_batch`
- `deleted_to` (or `in_place` for logical burn-down)
- `notes`

## Starter Markdown Scaffolds

Use these sections as the initial content for each file.

### `assimilation-map.md`

```md
# [Target Name] Assimilation Map

## 1. Target Overview
- Repository:
- Domain:
- Primary strengths:
- Suspected weak points:

## 2. Architecture Mapping (Target -> InfRing)
| Target Component | Target Purpose | InfRing Destination | Fit | Notes |
|---|---|---|---|---|

## 3. Value Assessment
- High-value subsystems:
- Medium-value subsystems:
- Reject candidates:

## 4. Compatibility + Risk
- Contract/authority risks:
- Runtime/perf risks:
- Security/policy risks:
- Integration complexity:

## 5. Assimilation Strategy
- Direct assimilation:
- Adaptation required:
- Defer/reject:

## 6. Open Questions
- Q1:
- Q2:
```

### `priority-ledger.md`

```md
# [Target Name] Priority Ledger

## Status Legend
- queued
- in_progress
- blocked
- done
- existing-coverage-validated

## Ordered Backlog
| ID | Status | Priority | Item | Source Map Section | Destination | Dependencies | Notes |
|---|---|---|---|---|---|---|---|

## Dependency Notes
- 
```

### `active-queue.md`

```md
# [Target Name] Active Queue

## Queue Policy
- Keep 5-15 items max.
- Pull from top of Priority Ledger only.
- Do not execute items not listed here.

## In Progress
| ID | Owner | Started | Expected Output | Verification |
|---|---|---|---|---|

## Ready Next
| ID | Reason Selected | Blocking Risk |
|---|---|---|
```

### `decisions-log.md`

```md
# [Target Name] Decisions Log

| Date | Decision ID | Type (accept/reject/defer) | Scope | Rationale | Impact | Revisit Trigger |
|---|---|---|---|---|---|---|
```

### `integration-notes.md`

```md
# [Target Name] Integration Notes

## Placement Notes
- Kernel:
- Orchestration:
- Shell:
- Gateways:

## Contract Notes
- Authority boundaries:
- Receipt/proof implications:
- Runtime guard implications:

## Follow-up Patches
- 
```

### `source-burn-down.tsv`

```tsv
source_path	status	first_batch	last_batch	deleted_to	notes
README.md	queued			in_place	
```

## Operator Checklist

Before execution:

1. Assimilation Map drafted.
2. Priority Ledger populated and ordered.
3. Active Queue seeded from top priorities.
4. Decision logging enabled.
5. Integration notes initialized.
6. Source burn-down tracker initialized (`source-burn-down.tsv`).
