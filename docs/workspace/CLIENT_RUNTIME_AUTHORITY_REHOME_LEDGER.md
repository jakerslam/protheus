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
| `client/runtime/config/provider_network_policy.json` | `core/layer0/ops/config/provider_network_policy.json` | Core ops | loader prefers canonical | Provider outbound network policy is Core execution/egress authority; provider runtime and image-tool provider paths now prefer the Core policy and fall back to the client mirror only for compatibility. |
| `client/runtime/config/secret_broker_policy.json` | `core/layer0/ops/config/secret_broker_policy.json` | Core ops | loader prefers canonical | Secret lookup, handle issuance, and rotation-health policy are Core security/ops authority; explicit policy path overrides and legacy mirrors remain supported during migration. |
| `client/runtime/config/rust_source_of_truth_policy.json` | `core/layer0/ops/config/rust_source_of_truth_policy.json` | Core ops | loader prefers canonical | Rust authority and TS-wrapper boundary contracts are Core ops authority; client copy is now only a compatibility mirror. |
| `client/runtime/config/security_layer_inventory.json` | `validation/release_gates/contracts/security_layer_inventory.json` | Validation release gates | loader prefers canonical | Security-layer evidence inventory is Validation authority; the Core gate now prefers the release-gate contract while the client copy remains a compatibility mirror. |
| `client/runtime/config/command_registry_policy.json` | `tests/tooling/config/command_registry_policy.json` | Tests/tooling | loader prefers canonical | Command-surface governance is tooling authority; the client copy is now a compatibility mirror. |
| `client/runtime/config/command_registry.json` | `tests/tooling/config/command_registry.json` | Tests/tooling | loader prefers canonical | Curated command registry belongs with tooling contracts; the client copy is now a compatibility mirror. |
| `client/runtime/config/lane_command_registry.json` | `tests/tooling/config/lane_command_registry.json` | Tests/tooling | loader prefers canonical | Lane dispatch registry is tooling authority; active tooling dispatch now defaults to the canonical copy. |
| `client/runtime/config/spawn_policy.json` | `orchestration/config/spawn_policy.json` | Orchestration | loader prefers canonical | Spawn pool and quota settings are orchestration-control policy; dashboard compatibility helpers now read the orchestration copy first. |
| `client/runtime/config/child_organ_runtime_policy.json` | `orchestration/config/child_organ_runtime_policy.json` | Orchestration | loader prefers canonical | Child-lane bounds and rollback rules are orchestration-control policy; the client copy is a compatibility mirror. |
| `client/runtime/config/orchestron_policy.json` | `orchestration/config/orchestron_policy.json` | Orchestration | loader prefers canonical | Orchestron promotion/evolution gates are orchestration-control policy; dashboard compatibility helpers now read the orchestration copy first. |
| `client/runtime/config/guard_check_registry.json` | `validation/release_gates/contracts/guard_check_registry.json` | Validation release gates | loader prefers canonical | Merge/release guard check registry is Validation authority; Core/Conduit defaults now point at the release-gate contract while the client copy remains a compatibility mirror. |
| Tavily/Jina web retrieval additions | `orchestration/config/web_research_retrieval_policy.json`, `core/layer2/tooling/tool_cds/web_retrieval_v0.tool.json` | Orchestration + Core Tooling | rehomed | Added after checkpoint `a299d49d6`; moved out of client in commit `4c355921b`. |

## Loader Flip Wave

| Loader / Guard | Canonical Path Now Preferred | Compatibility Fallback | Status |
| --- | --- | --- | --- |
| `core/layer0/ops/src/batch_query_primitive_parts/010-core_parts/000-part.rs` | `core/layer0/ops/config/batch_query_policy.json` | `client/runtime/config/batch_query_policy.json` | flipped |
| `core/layer0/ops/src/batch_query_primitive_parts/010-core.combined_parts/020-policy-rel-to-instruction-tail-regex.rs` | `core/layer0/ops/config/batch_query_policy.json` | `client/runtime/config/batch_query_policy.json` | flipped |
| `core/layer0/ops/src/web_conduit_parts/010-prelude-and-policy.rs` | `core/layer0/ops/config/web_conduit_policy.json` | `client/runtime/config/web_conduit_policy.json` | flipped |
| `core/layer0/ops/src/dashboard_provider_runtime_parts/022-provider-adapters_parts/010-const-provider-inference-receipts-rel.rs` | `core/layer0/ops/config/provider_network_policy.json` | `client/runtime/config/provider_network_policy.json` | flipped |
| `core/layer0/ops/src/web_conduit_parts/091-image-tool-provider-execution.rs` | `core/layer0/ops/config/provider_network_policy.json` | `client/runtime/config/provider_network_policy.json` | flipped |
| `core/layer0/ops/src/secret_broker_kernel_parts/010-secret-broker-state.rs` | `core/layer0/ops/config/secret_broker_policy.json` | `client/runtime/config/secret_broker_policy.json` | flipped |
| `core/layer0/ops/src/health_status_parts/010-receipt-hash.rs` / `020-audit-cron-delivery.rs` | `core/layer0/ops/config/secret_broker_policy.json` | `client/runtime/config/secret_broker_policy.json` | flipped |
| Core rust-source-of-truth readers | `core/layer0/ops/config/rust_source_of_truth_policy.json` | `client/runtime/config/rust_source_of_truth_policy.json` | flipped |
| `core/layer0/ops/src/security_layer_inventory_gate_kernel.rs` | `validation/release_gates/contracts/security_layer_inventory.json` | `client/runtime/config/security_layer_inventory.json` | flipped |
| Tooling command registry scripts | `tests/tooling/config/command_registry_policy.json`, `tests/tooling/config/command_registry.json`, `tests/tooling/config/lane_command_registry.json` | matching `client/runtime/config/*.json` mirrors | flipped |
| `core/layer0/ops/src/dashboard_compat_api_parts/set_config_payload_parts/120-part.rs` | `orchestration/config/spawn_policy.json`, `orchestration/config/child_organ_runtime_policy.json`, `orchestration/config/orchestron_policy.json` | matching `client/runtime/config/*.json` mirrors | flipped |
| Core/Conduit guard-registry defaults | `validation/release_gates/contracts/guard_check_registry.json` | `client/runtime/config/guard_check_registry.json` mirror kept for legacy callers | partial |
| `core/layer0/ops/src/research_plane_parts/010-usage.rs` | `orchestration/config/research_plane_policy.json` | `client/runtime/config/research_plane_policy.json` | flipped |
| `tests/tooling/scripts/ci/web_retrieval_reliability_closure_guard.ts` | `core/layer0/ops/config/web_conduit_policy.json` | none needed for proof guard | flipped |
| `tests/tooling/scripts/ci/shell_authority_config_guard.ts` | mirror declarations for batch query, web conduit, and research plane policies | branch-diff check rejects new authority-shaped client config JSON unless explicitly mirror-marked | guarded |

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

1. Classify and rehome high-risk security, memory, command, gateway/conduit, workflow, and routing configs in small batches.
2. Triage `client/runtime/systems/*` for Shell wrappers versus authority implementations, prioritizing `security`, `memory`, `workflow`, `routing`, `tools`, and `ops`.

Completed queue item:

- Guard against new client-runtime authority configs: `ops:shell:authority-config:guard` now checks the branch diff for new authority-shaped JSON under `client/runtime/config` and requires explicit mirror metadata (`compatibility_mirror` or `legacy_mirror` plus canonical path/owner).
- Existing duplicate policy mirrors triaged: `workflow_executor_policy.json`, `agent_routing_rules.json`, `provider_onboarding_manifest.json`, and `abac_policy_plane.json` normalize identically against their canonical homes after mirror metadata is removed. No behavioral drift found.

## Guardrails

- Do not bulk-delete client files while live loaders may still depend on them.
- Do not move by filename alone; inspect loader and execution paths first.
- Do not rehome UI projection or dashboard-only code into Core/Orchestration.
- Keep client copies as compatibility mirrors until tests prove canonical loaders work.
- Any new authority discovered during feature work should be placed in Core, Orchestration, Tool CD, Validation, or Observability first.
