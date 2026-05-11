# Client Runtime Authority Rehome Ledger

Status: active migration inventory
Checkpoint before inventory: `8857270e5`

## Boundary Rule

`client/runtime` is legacy Shell/runtime path compatibility. It may keep presentation, projection, local volatile state, build/deploy packaging, and compatibility mirrors while callers migrate. It should not be the canonical home for Kernel, Orchestration, Tooling, Validation, or Observability authority.

## First Rehome Wave

This wave does not delete client files yet. It creates canonical non-client copies for high-risk web/research authority while leaving client copies as compatibility mirrors until all downstream paths can be retired safely.

| Client Runtime Source | Canonical Non-Client Home | Owner | Status | Notes |
| --- | --- | --- | --- | --- |
| `client/runtime/config/batch_query_policy.json` | `core/layer0/ops/config/batch_query_policy.json` | Core ops | loader prefers canonical | Batch query primitive execution already lives in Core; the Core loader now prefers this policy and falls back to the client mirror only for compatibility. |
| `client/runtime/config/web_conduit_policy.json` | `core/layer0/ops/config/web_conduit_policy.json` | Core ops | loader prefers canonical | Web conduit mechanics and gateway-like fetch behavior belong with Core ops authority; env override remains supported, then canonical Core policy, then legacy mirror. |
| `client/runtime/config/research_plane_policy.json` | `orchestration/config/research_plane_policy.json` | Orchestration | loader prefers canonical | Research strategy/evidence lifecycle belongs to Orchestration; diagnostics now report the path actually loaded. |
| Tavily/Jina web retrieval additions | `orchestration/config/web_research_retrieval_policy.json`, `core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json` | Orchestration + Core Tooling | rehomed | Added after checkpoint `a299d49d6`; moved out of client in commit `4c355921b`. |

## Loader Flip Wave

| Loader / Guard | Canonical Path Now Preferred | Compatibility Fallback | Status |
| --- | --- | --- | --- |
| `core/layer0/ops/src/batch_query_primitive_parts/010-core_parts/000-part.rs` | `core/layer0/ops/config/batch_query_policy.json` | `client/runtime/config/batch_query_policy.json` | flipped |
| `core/layer0/ops/src/batch_query_primitive_parts/010-core.combined_parts/020-policy-rel-to-instruction-tail-regex.rs` | `core/layer0/ops/config/batch_query_policy.json` | `client/runtime/config/batch_query_policy.json` | flipped |
| `core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs` | `core/layer0/ops/config/web_conduit_policy.json` | `client/runtime/config/web_conduit_policy.json` | flipped |
| `core/layer0/ops/src/research_plane_parts/010-usage.rs` | `orchestration/config/research_plane_policy.json` | `client/runtime/config/research_plane_policy.json` | flipped |
| `tests/tooling/scripts/ci/web_retrieval_reliability_closure_guard.ts` | `core/layer0/ops/config/web_conduit_policy.json` | none needed for proof guard | flipped |

## Runtime Inventory Snapshot

| Area | Count / Size | Initial Classification |
| --- | ---: | --- |
| `client/runtime/config` | 768 top-level files | Mixed legacy config, many authority-like policies. Needs staged rehome. |
| `client/runtime/systems` | 259 files at depth 2 | Mixed Shell wrappers and historical runtime authority. Needs subsystem triage. |
| `client/runtime/local` | about 400 MB | Local volatile state/caches/reports. Mostly keep local, but cleanup lifecycle should be Core/Orchestration-owned. |
| `client/runtime/lib` | 62 files | Shared compatibility helpers. Triage after config/systems. |
| `client/runtime/deploy` | 25 files | Packaging/deploy. Likely keep or move to deploy ownership, not runtime authority. |

Rough keyword scan of `client/runtime/config`:

| Keyword | Count |
| --- | ---: |
| `security` | 7 |
| `memory` | 19 |
| `workflow` | 4 |
| `route` / `routing` | 8 |
| `command` | 7 |
| `conduit` | 5 |
| `gateway` | 6 |
| `research` | 3 |
| `web` | 3 |
| `batch` | 5 |
| `tool` | 5 |
| `orchestration` | 6 |
| `policy` | 661 |

The keyword counts are triage hints only. Final ownership must follow behavior and loader use, not filenames.

## Migration Categories

| Category | Destination | Rule |
| --- | --- | --- |
| Shell/projection | `client/runtime/**` | May remain if it only renders, packages, projects, or calls authoritative APIs. |
| Local volatile state | `client/runtime/local/**` or `core/local/**` | May remain local if non-authoritative; cleanup and retention policy should live outside client. |
| Core authority | `core/**` | Execution, storage, memory, security, permissions, tool mechanics, gateway/conduit mechanics. |
| Orchestration authority | `orchestration/**` | Workflow choice, planning, synthesis, evidence lifecycle, research strategy, eval gates. |
| Tool CD | `core/layer2/tooling/tool_cds/**` | Tool capability declarations, request/result contracts, evidence artifact vocabulary. |
| Validation | `validation/**` or `tests/**` | CI guards, scorecards, release gates, fixture policies. |
| Observability | `observability/**` | Telemetry, reports, trace policies, monitoring. |
| Legacy mirror | original `client/runtime/**` path | Allowed temporarily only while load paths migrate; must not receive new authority. |

## Next Action Queue

1. Add a guard that prevents new authority policy additions under `client/runtime/config` unless they are explicitly marked `legacy_mirror`.
2. Triage already-duplicated files that differ between client and canonical homes:
   - `workflow_executor_policy.json`
   - `agent_routing_rules.json`
   - `provider_onboarding_manifest.json`
   - `abac_policy_plane.json`
3. Classify and rehome high-risk security, memory, command, gateway/conduit, workflow, and routing configs in small batches.
4. Triage `client/runtime/systems/*` for Shell wrappers versus authority implementations, prioritizing `security`, `memory`, `workflow`, `routing`, `tools`, and `ops`.

## Guardrails

- Do not bulk-delete client files while live loaders may still depend on them.
- Do not move by filename alone; inspect loader and execution paths first.
- Do not rehome UI projection or dashboard-only code into Core/Orchestration.
- Keep client copies as compatibility mirrors until tests prove canonical loaders work.
- Any new authority discovered during feature work should be placed in Core, Orchestration, Tool CD, Validation, or Observability first.
