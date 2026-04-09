# Rust Authority Closure List

Generated: 2026-03-18T00:03:54.821Z

Current Rust share: **90.38%**

Raw live queue size: **93**

## Summary

- Migrate to Rust authority: **24**
- Exclude as flexible or non-authority surfaces: **21**
- Queue-cleanup / thin-wrapper removals: **48**

## Migrate To Rust Authority

| Rank | Path | LOC | Impact |
|---:|---|---:|---:|
| 1 | client/runtime/systems/sensory/conversation_eye_synthesizer.ts | 86 | 369.8 |
| 2 | client/lib/conduit_full_lifecycle_probe.ts | 149 | 327.8 |
| 3 | client/lib/trainability_matrix.ts | 146 | 321.2 |
| 4 | client/lib/dynamic_burn_budget_signal.ts | 143 | 314.6 |
| 5 | client/runtime/lib/policy_runtime.ts | 109 | 305.2 |
| 6 | client/runtime/lib/moltbook_api.ts | 102 | 285.6 |
| 7 | client/lib/emergency_stop.ts | 123 | 270.6 |
| 8 | core/layer1/memory_runtime/adaptive/catalog_store.ts | 179 | 268.5 |
| 9 | client/lib/proposal_type_classifier.ts | 117 | 257.4 |
| 10 | client/runtime/systems/memory/rust_memory_transition_lane.ts | 59 | 253.7 |
| 11 | client/runtime/lib/state_artifact_contract.ts | 80 | 224 |
| 15 | client/runtime/systems/ops/f100_readiness_remediation.ts | 44 | 189.2 |
| 17 | client/runtime/lib/runtime_path_registry.ts | 59 | 165.2 |
| 22 | client/runtime/lib/uid.ts | 46 | 128.8 |
| 23 | client/runtime/lib/agent_passport_link.ts | 45 | 126 |
| 24 | client/runtime/lib/tool_compactor_integration.ts | 45 | 126 |
| 25 | client/runtime/lib/eyes_catalog.ts | 42 | 117.6 |
| 27 | client/runtime/lib/ts_entrypoint.ts | 37 | 103.6 |
| 28 | client/runtime/lib/integrity_hash_utility.ts | 32 | 89.6 |
| 29 | client/runtime/lib/command_output_compactor.ts | 30 | 84 |
| 30 | core/layer1/memory_runtime/adaptive/uid.ts | 53 | 79.5 |
| 33 | client/cognition/shared/lib/legacy_retired_wrapper.ts | 38 | 57 |
| 38 | client/lib/protheus_suite_tooling.ts | 18 | 39.6 |
| 39 | client/lib/redaction_classification.ts | 14 | 30.8 |

## Exclude Non-Authority / Flexible Surfaces

| Rank | Path | LOC | Impact |
|---:|---|---:|---:|
| 12 | packages/protheus-npm/scripts/install.ts | 146 | 219 |
| 13 | packages/protheus-edge/index.ts | 143 | 214.5 |
| 14 | adapters/importers/generic_yaml_importer.ts | 93 | 195.3 |
| 16 | packages/protheus-core/index.ts | 123 | 184.5 |
| 18 | packages/protheus-npm/bin/protheus.ts | 96 | 144 |
| 19 | adapters/importers/infring_importer.ts | 67 | 140.7 |
| 20 | adapters/importers/workflow_graph_importer.ts | 67 | 140.7 |
| 21 | adapters/importers/generic_json_importer.ts | 66 | 138.6 |
| 26 | client/cognition/orchestration/cli_shared.ts | 71 | 106.5 |
| 31 | packages/protheus-core/core_profile_contract.ts | 53 | 79.5 |
| 32 | packages/protheus-npm/scripts/smoke.ts | 43 | 64.5 |
| 35 | client/runtime/platform/api/donate_gpu.ts | 29 | 43.5 |
| 36 | adapters/cognition/collectors/bird_x.ts | 19 | 39.9 |
| 37 | adapters/cognition/collectors/ollama_search.ts | 19 | 39.9 |
| 41 | adapters/cognition/skills/imap-smtp-email/scripts/imap.ts | 10 | 21 |
| 42 | adapters/cognition/skills/imap-smtp-email/scripts/smtp.ts | 10 | 21 |
| 43 | adapters/cognition/skills/moltbook/actuation_adapter.ts | 10 | 21 |
| 44 | adapters/cognition/skills/moltbook/moltbook_publish_guard.ts | 10 | 21 |
| 45 | adapters/cognition/skills/moltbook/proposal_template.ts | 10 | 21 |
| 46 | adapters/cognition/skills/moltstack/scripts/publish.ts | 10 | 21 |
| 47 | adapters/cognition/skills/moltstack/scripts/quality-check.ts | 10 | 21 |

## Queue Cleanup / Thin Wrappers

| Rank | Path | LOC | Impact |
|---:|---|---:|---:|
| 34 | client/cli/bin/protheus-graph.ts | 37 | 55.5 |
| 40 | vitest.config.ts | 19 | 28.5 |
| 48 | client/types/node_compat.d.ts | 12 | 18 |
| 49 | client/lib/ts_entrypoint.ts | 6 | 13.2 |
| 50 | client/lib/action_envelope.ts | 5 | 11 |
| 51 | client/lib/action_receipts.ts | 5 | 11 |
| 52 | client/lib/agent_passport_link.ts | 5 | 11 |
| 53 | client/lib/approval_gate.ts | 5 | 11 |
| 54 | client/lib/command_output_compactor.ts | 5 | 11 |
| 55 | client/lib/directive_resolver.ts | 5 | 11 |
| 56 | client/lib/duality_seed.ts | 5 | 11 |
| 57 | client/lib/egress_gateway.ts | 5 | 11 |
| 58 | client/lib/exec_compacted.ts | 5 | 11 |
| 59 | client/lib/eyes_catalog.ts | 5 | 11 |
| 60 | client/lib/integrity_hash_utility.ts | 5 | 11 |
| 61 | client/lib/legacy_alias_adapter.ts | 5 | 11 |
| 62 | client/lib/legacy_conduit_proxy.ts | 5 | 11 |
| 63 | client/lib/mech_suit_mode.ts | 5 | 11 |
| 64 | client/lib/ops_domain_conduit_runner.ts | 5 | 11 |
| 65 | client/lib/passport_iteration_chain.ts | 5 | 11 |
| 66 | client/lib/policy_runtime.ts | 5 | 11 |
| 67 | client/lib/queued_backlog_runtime.ts | 5 | 11 |
| 68 | client/lib/quorum_validator.ts | 5 | 11 |
| 69 | client/lib/runtime_path_registry.ts | 5 | 11 |
| 70 | client/lib/secret_broker.ts | 5 | 11 |
| 71 | client/lib/security_integrity.ts | 5 | 11 |
| 72 | client/lib/state_artifact_contract.ts | 5 | 11 |
| 73 | client/lib/strategy_resolver.ts | 5 | 11 |
| 74 | client/lib/success_criteria_compiler.ts | 5 | 11 |
| 75 | client/lib/success_criteria_verifier.ts | 5 | 11 |
| 76 | client/lib/ternary_belief_engine.ts | 5 | 11 |
| 77 | client/lib/tool_compactor_integration.ts | 5 | 11 |
| 78 | client/lib/tool_response_compactor.ts | 5 | 11 |
| 79 | client/lib/ts_bootstrap.ts | 5 | 11 |
| 80 | client/lib/uid.ts | 5 | 11 |
| 81 | client/lib/upgrade_lane_runtime.ts | 5 | 11 |
| 82 | client/cli/bin/protheus-bootstrap.ts | 6 | 9 |
| 83 | client/cli/bin/protheus-econ.ts | 6 | 9 |
| 84 | client/cli/bin/protheus-forge.ts | 6 | 9 |
| 85 | client/cli/bin/protheus-mem.ts | 6 | 9 |
| 86 | client/cli/bin/protheus-pinnacle.ts | 6 | 9 |
| 87 | client/cli/bin/protheus-redlegion.ts | 6 | 9 |
| 88 | client/cli/bin/protheus-soul.ts | 6 | 9 |
| 89 | client/cli/bin/protheus-swarm.ts | 6 | 9 |
| 90 | client/cli/bin/protheus-telemetry.ts | 6 | 9 |
| 91 | client/cli/bin/protheus-vault.ts | 6 | 9 |
| 92 | client/cognition/shared/adaptive/rsi/rsi_bootstrap.ts | 5 | 7.5 |
| 93 | client/cognition/shared/adaptive/rsi/rsi_integrity_chain_guard.ts | 5 | 7.5 |

