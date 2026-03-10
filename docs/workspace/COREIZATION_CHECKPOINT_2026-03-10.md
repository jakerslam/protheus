# Coreization Checkpoint (2026-03-10)

## Scope
- Hard Coreization Wave 1 static verification pass for:
  - `client/runtime/systems/security`
  - `client/runtime/systems/spine`
  - `client/runtime/systems/memory`
  - `client/runtime/systems/autonomy`
  - `client/runtime/systems/workflow`
  - `client/runtime/systems/ops/protheusd.ts`

## Static Audit Result
- Command:
  - `node scripts/ci/coreization_wave1_static_audit.mjs --out artifacts/coreization_wave1_static_audit_2026-03-10.json`
- Result:
  - `pass: true`
  - `violation_count: 0`
  - `warning_count: 0`
- Module summary:
  - `security`: wrappers `197`, non-wrappers `0`
  - `spine`: wrappers `14`, non-wrappers `0`
  - `memory`: wrappers `63`, non-wrappers `0`
  - `autonomy`: wrappers `122`, non-wrappers `0`
  - `workflow`: wrappers `35`, non-wrappers `0`
  - `ops-daemon`: wrappers `1`, non-wrappers `0`

## Cleanup Applied
- Added missing ownership marker in:
  - `client/runtime/systems/security/venom_containment_layer.ts`
- Added wrapper-policy marker token used by layer-placement guard:
  - `client/runtime/systems/security/venom_containment_layer.ts`
- Layer simplification/refactor:
  - Moved 19 high-level V6 feature lanes from `core/layer0/ops/src` to `core/layer2/ops/src`:
    - `opendev_dual_agent`, `company_layer_orchestration`, `wifi_csi_engine`,
      `biological_computing_adapter`, `observability_automation_engine`,
      `persistent_background_runtime`, `workspace_gateway_runtime`, `p2p_gossip_seed`,
      `startup_agency_builder`, `timeseries_receipt_engine`, `webgpu_inference_adapter`,
      `context_doctor`, `discord_swarm_orchestration`, `bookmark_knowledge_pipeline`,
      `public_api_catalog`, `decentralized_data_marketplace`, `autoresearch_loop`,
      `intel_sweep_router`, `gui_drift_manager`.
  - `core/layer0/ops/src/main.rs` dispatch now routes these lanes through `protheus_ops_core_v1` (Layer 2 crate).

## Rust Share
- Command:
  - `npm run -s metrics:rust-share:gate`
- Result:
  - `rust_share_pct: 64.818`
  - `rs: 119441`, `ts: 36898`, `js: 27933`

## Runtime Regression Status
- Runtime execution is now unblocked in-session:
  - `/tmp/hello_c_test` exits `0` with expected stdout.
  - `/tmp/hello_rust_test` exits `0` with expected stdout.
  - `./target/debug/protheus-ops status` returns JSON receipt successfully.
- Verification stack is green:
  - `npm run -s ops:layer-placement:check` -> pass
  - `npm run -s metrics:rust-share:gate` -> pass (`64.818%`)
  - `npm run -s ops:srs:top200:regression` -> pass (`200/200`)
  - `./verify.sh` -> pass
- Regression suite previously blocked by runtime now passes:
  - `npm run -s typecheck:systems`
  - `npm run -s test:ops:source-runtime-classifier`
  - `npm run -s test:ops:subconscious-boundary-guard`
  - `npm run -s test:memory:context-budget`
  - `npm run -s test:memory:matrix`
  - `npm run -s test:memory:auto-recall`
  - `npm run -s test:reflexes`
- Runtime unblock and regression evidence:
  - `artifacts/todo_execution_2026-03-10_resume_runtime_unblocked.json`
  - `artifacts/regression_suite_resume_runtime_unblocked.json`
  - `artifacts/srs_top200_regression_2026-03-10.json`

## Backlog Resume Tranche (Post-Unblock)
- Completed and marked `done`:
  - `V6-LLMN-001..004` (mode registry/parity/path/conformance shield)
  - `V6-MEMORY-013..019` (low-burn retrieval/hydration/budget/ranking/freshness + LensMap annotation shield)
  - `V6-MEMORY-021` (per-query token telemetry + burn-SLO trace lane)
  - `V6-SWARM-001..006` (swarm router closure tranche)
  - `V6-INITIATIVE-013`, `V6-ARCH-ICEBERG-028` (initiative + iceberg live runtime closure tranche)
  - `V6-PRIM-001`, `V6-PRIM-002`, `V6-PRIM-003`, `V6-PRIM-006` (layer1 primitive rust-closure tranche)
  - `V6-PRIM-009` (primitive residual audit refresh)
- Validation evidence:
  - `tests/client-memory-tools/llmn_mode_conformance.test.js`
  - `tests/client-memory-tools/strategy_resolver.test.js`
  - `tests/client-memory-tools/model_router_routing_features.test.js`
  - `tests/client-memory-tools/model_router_variant_policy.test.js`
  - `tests/client-memory-tools/legacy_path_alias_adapters.test.js`
  - `tests/client-memory-tools/memory_recall_context_budget.test.js`
  - `tests/client-memory-tools/conversation_eye_bootstrap.test.js`
  - `tests/client-memory-tools/memory_burn_slo_guard.test.js`
  - `tests/client-memory-tools/memory_efficiency_plane.test.js`
  - `tests/client-memory-tools/memory_matrix.test.js`
  - `tests/client-memory-tools/memory_auto_recall.test.js`
  - `tests/client-memory-tools/memory_index_freshness_gate.test.js`
  - `cargo test --manifest-path core/layer0/swarm_router/Cargo.toml`
  - `cargo test -p protheus-ops-core attention_queue`
  - `cargo test -p execution_core initiative`
  - `npm run -s test:ops:source-runtime-classifier`
  - `npm run -s test:ops:subconscious-boundary-guard`
  - `cargo test -p task`
  - `cargo test -p resource`
  - `cargo test -p isolation`
  - `cargo test -p protheus-observability-core-v1`
- Actionable backlog count after tranche:
  - `artifacts/backlog_actionable_report_2026-03-10_resume.json` -> `actionable_count: 376`
