# local-research-agent State Audit (V11-APP-VALIDATION-001)

**Date:** 2026-04-26
**Author:** Cowork (V11-APP-VALIDATION-001)
**Headline finding:** `apps/local-research-agent/` is a README-only placeholder. The "flagship validation app" we picked for the App Validation Pivot does not exist as code. The audit recommends pivoting the flagship target to `apps/local-rag/`, which has the same thin-shell shape but is wired to a real `rag` domain handler in `core/layer0/ops/`.

---

## 1. State of `apps/local-research-agent/`

```
apps/local-research-agent/
└── README.md                    (544 bytes, last touched 2026-03-23)
```

**There is no source code.** The directory contains exactly one file: a 17-line README sketching intent ("Gather, parse, and summarize sources locally", "Citation-aware summary generation", "Optional handoff into memory hierarchy") and three suggested CLI commands that have no implementation:

- `infring research fetch --url=...`
- `infring research diagnostics`
- `infring think --session-id=research --prompt=...`

The `app.json` manifest used by every other thin-shell app under `apps/**` is **absent**. There is no `run.ts` wrapper. There is no fixture. There are no tests. The README has not been touched since 2026-03-23, predating the App Validation Pivot intake by a month.

This means **V11-APP-VALIDATION-001 cannot proceed with `local-research-agent` as the target**. There is nothing to audit beyond the README.

## 2. Inventory of the `apps/**` Surface

22 directories under `apps/`. File counts (excluding hidden files):

| Tier | Apps | Shape |
|---|---|---|
| **Placeholder (1 file)** | `local-research-agent`, `mcu-sensor-monitor-tiny-max`, `sovereign-memory-os` | README only, no source |
| **Stub (2 files)** | `ad_factory`, `creator_outreach`, `video-ad-factory` | README + `app.json`, no `run.ts` |
| **Thin shell (3 files)** | `chat_starter`, `chat_ui`, `code_engineer`, `graph-toolkit`, `intelligence-nexus`, `local-rag`, `snowball_engine` | Canonical pattern: README + `app.json` + `run.ts` |
| **Slightly larger (4–5 files)** | `habits`, `photo-grit` | 3-file pattern + minor extras |
| **Substantive** | `examples` (15 files), `prism` (116 files) | Real source, not following the thin-shell pattern |
| **Outliers** | `lensmap` (15,975 files — separate product with its own Cargo workspace), `personas/rohan-kapoor/` (4,303 files — sample data, not app code) | Not in scope for App Validation Pivot |

**13 of 22 apps have ≤5 files; 7 of 22 apps follow the canonical 3-file thin-shell pattern; 3 of 22 apps are README-only placeholders.** This contradicts the framing in the project README that "22 thin first-party app surfaces" exist as runnable shells.

## 3. The Canonical Thin-Shell Pattern

The 3-file apps share a strict pattern:

**`run.ts`** — a 5-line wrapper:
```ts
#!/usr/bin/env node
'use strict';
const { runInfringOps } = require('../../client/runtime/systems/ops/run_infring_ops.ts');
const args = process.argv.slice(2);
const commandArgs = args.length === 0 ? ['<default-subcommand>'] : args;
const exit = runInfringOps(['<app-id>', ...commandArgs]);
process.exit(exit);
```

**`app.json`** — a manifest:
```json
{
  "id": "<app-id>",
  "srs": "<comma-separated SRS IDs>",
  "layer": "app",
  "entrypoint": "infring app run <app-id>",
  "authority": "core"
}
```

**`README.md`** — purpose, integration contract, suggested commands.

The runtime path is:
```
apps/<app>/run.ts
  → client/runtime/systems/ops/run_infring_ops.ts (shim)
  → adapters/runtime/run_infring_ops.ts (the file I migrated in V11-OPS-PRD-001-PR1)
  → ops_lane_bridge → resident IPC → core/layer0/ops/<app domain handler>
```

**Implication for PR1 soak:** every thin-shell app's `run.ts` exercises the same runner I just modified. If PR1 (commit `086ca76b8`) introduces a regression in bridge-failure handling, it surfaces as an exit-1 with structured deny payload from any thin-shell app. That's the right behavior, but worth flagging for the V11-OPS-PRD-001-PR1 soak.

## 4. Backend Wiring Check

| App | Has `run.ts`? | Domain handler in `core/`? | Verdict |
|---|---|---|---|
| `local-research-agent` | ❌ no | ❌ no `research` domain found | Not viable as flagship |
| `local-rag` | ✅ yes | ✅ `rag` route in `core/layer0/ops/src/infringctl_routes_parts/010-command-routing_parts/003-...001-resolve_core_shortcuts_family_ops1_group_1.rs:21` | **Viable** |
| `intelligence-nexus` | ✅ yes | ✅ `intelligence_nexus_keys.rs` in core | **Viable** |
| `chat_starter` | ✅ yes | likely chat handler in core (not verified) | Probably viable |
| `code_engineer` | ✅ yes | likely codex handler in core (not verified) | Probably viable |
| `snowball_engine` | ✅ yes | not verified | Unknown |
| `graph-toolkit` | ✅ yes | not verified | Unknown |

`local-rag` is the cleanest concrete target: thin shell + real backend + real commands documented in its README + exercises memory and retrieval (high-leverage validation surface).

## 5. Critical Findings

### F1 — `local-research-agent` cannot be the flagship validation target [BLOCKER]

The chosen flagship app is README-only. Validation work cannot proceed against this target without first implementing the entire app. **Estimated implementation cost: weeks** (research/fetch CLI + parser + summary generator + memory handoff + receipt emission), and the work would itself be unvalidated since there's no team consensus on what "Local Research Agent" should do.

**Recommended remediation:** pivot the flagship to `apps/local-rag/` for V11-APP-VALIDATION-002 onward. Update SRS V11-APP-VALIDATION-001 through -009 references from `local-research-agent` to `local-rag`. Effort: ~30 minutes of doc edits.

### F2 — Three apps are README-only placeholders [MAJOR]

`local-research-agent`, `mcu-sensor-monitor-tiny-max`, `sovereign-memory-os` are 1-file directories. They occupy the `apps/` surface area without contributing runnable code. They show up in the README's "22 first-party apps" claim but are not first-party apps in any operational sense.

**Recommended remediation:** either implement them (multi-week each), demote to `apps/_placeholder/` so the inventory is honest, or remove them with a tracked retirement entry. Effort: ~2 hours for demotion + retirement record per app.

### F3 — Three apps are 2-file stubs missing `run.ts` [MAJOR]

`ad_factory`, `creator_outreach`, `video-ad-factory` have README + `app.json` but no `run.ts`. Per the canonical pattern, this means they cannot be invoked at all — `app.json` declares an entrypoint that has no wrapper.

**Recommended remediation:** either add the canonical 5-line `run.ts` wrapper (effort: ~5 minutes per app, but only useful if the corresponding core domain handler exists) or demote/retire alongside F2. Effort: ~30 minutes for the wrapper across all three; add as a follow-up after backend wiring is confirmed.

### F4 — Backend wiring is unverified for most thin-shell apps [MAJOR]

Of the seven 3-file thin-shell apps, I verified only two (`local-rag`, `intelligence-nexus`) have working core domain handlers. The other five (`chat_starter`, `chat_ui`, `code_engineer`, `graph-toolkit`, `snowball_engine`) probably do but I did not confirm. Without backend wiring, the thin shell is decorative.

**Recommended remediation:** add a `tool_routing_authority` check that asserts every `apps/*/app.json` with `"layer": "app"` has a matching domain handler in `core/layer0/ops/`. Effort: ~half-day for the guard. Tracked as a new follow-up below.

### F5 — Suggested commands in `local-research-agent/README.md` reference a non-existent `research` domain [MINOR]

`infring research fetch`, `infring research diagnostics`, and `infring think` are the three documented commands. None of them have core domain handlers. The README sets expectations the codebase cannot meet, which is itself a contributing reason this app has gone implementation-orphaned for a month.

**Recommended remediation:** delete or revise the suggested-commands section once the pivot lands. Effort: ~5 minutes.

### F6 — `apps/personas/rohan-kapoor/projects/` and `apps/lensmap/` skew the inventory [INFORMATIONAL]

`personas` (4,303 files) is a per-persona sample-data directory, not an app. `lensmap` (15,975 files) is a separate Cargo workspace product. Both inflate the apparent `apps/` surface area without being first-party app code.

**Recommended remediation:** none required, but worth noting in the playbook (V11-APP-VALIDATION-009) that these aren't validation candidates. The 22-app claim should be qualified.

## 6. Recommended Pivot

Pivot the App Validation Pivot flagship from `local-research-agent` to `local-rag`.

**Rationale:**
- `local-rag` already has the canonical 3-file shape, so V11-APP-VALIDATION-001..002 work is significantly reduced.
- `local-rag` has a real backend route (`rag` → `core://rag`) which means V11-APP-VALIDATION-005 (wire end-to-end) is closer to "verify and harden" than "implement from zero".
- `local-rag` exercises memory ingestion, retrieval, and chat — three high-value subsystems that the orchestration contracts and transient observation invariants were built to support. Validating `local-rag` validates a much wider surface than validating `local-research-agent` would have.
- The README documents specific commands (`start`, `ingest`, `search`, `chat`, `memory search`) that map onto well-defined intents.

**Cost of pivot:** update SRS V11-APP-VALIDATION-002 through -009 to reference `local-rag` instead of `local-research-agent`. Update the task list subjects/descriptions accordingly. ~30 minutes of doc work.

**Effect on V11-APP-VALIDATION-001:** this audit doc closes V11-APP-VALIDATION-001 with the headline finding "flagship target is non-viable; pivot to `local-rag`". The audit deliverable is satisfied; the audit's primary recommendation is the pivot.

## 7. Severity-Ranked Findings (machine-readable form)

See `local_research_agent_state_audit.json` sidecar for the structured form.

## 8. What This Audit Did NOT Cover

- I did not run the install path on a fresh machine — that's V11-APP-VALIDATION-003 and is best done with a real fresh environment, not the bash sandbox.
- I did not verify backend wiring for `chat_starter`, `chat_ui`, `code_engineer`, `graph-toolkit`, `snowball_engine`. Worth a half-day audit before declaring the thin-shell pattern complete.
- I did not assess `examples/` or `prism/` as alternative flagship targets. They have substantive source but don't follow the thin-shell pattern, so they're a different validation shape.

## 9. Action Items Out of This Audit

1. **Pivot V11-APP-VALIDATION-002 through -009 from `local-research-agent` to `local-rag`** — 30-minute doc edit. Should land before any other V11-APP-VALIDATION work begins.
2. **Decide what to do with `local-research-agent`, `mcu-sensor-monitor-tiny-max`, `sovereign-memory-os`** — either implement (weeks) or demote/retire (~2h each). Track as a separate decision rather than blocking the pivot.
3. **Add a backend-wiring CI guard** asserting every `apps/*/app.json` has a matching `core/layer0/ops/` domain handler. ~half-day of Rust. Track as V11-APP-VALIDATION-001-FOLLOWUP-001.
4. **Audit the five unverified thin-shell apps** (`chat_starter`, `chat_ui`, `code_engineer`, `graph-toolkit`, `snowball_engine`) for backend wiring. ~half-day. Probably folds into the guard from item 3.
5. **Update the project README's "22 first-party apps" claim** to reflect the actual viable count once the placeholder/stub disposition is decided. Add to V11-APP-VALIDATION-009 (playbook) as a credibility-restoration item.
