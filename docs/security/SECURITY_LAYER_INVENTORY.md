# Security Layer Inventory

Generated: 2026-03-06T09:38:40.032Z

This inventory maps each security layer to enforceable implementation paths, policy contracts, runtime guard checks, and test evidence.

| Layer | Implementation | Policy | Guard Checks | Test Evidence |
|---|---|---|---|---|
| `constitution_policy_core`<br>Constitution + policy contract enforcement | `AGENT-CONSTITUTION.md`<br>`crates/ops/src/contract_check.rs` | `config/guard_check_registry.json`<br>`config/directives/T0_invariants.yaml` | `contract_check`: `node systems/spine/contract_check.js`<br>`formal_invariant_engine`: `node systems/security/formal_invariant_engine.js run --strict=1`<br>`critical_path_formal_verifier`: `node systems/security/critical_path_formal_verifier.js run --strict=1` | `crates/ops/src/contract_check.rs`<br>`memory/tools/tests/critical_path_formal_verifier.test.js` |
| `conduit_boundary`<br>Rust conduit boundary and command security | `crates/conduit/src/lib.rs`<br>`crates/conduit-security/src/lib.rs` | `config/guard_check_registry.json`<br>`config/runtime_scheduler_policy.json` | `formal_invariant_engine`: `node systems/security/formal_invariant_engine.js run --strict=1`<br>`critical_path_formal_verifier`: `node systems/security/critical_path_formal_verifier.js run --strict=1` | `crates/conduit/tests/invariants.rs`<br>`crates/conduit/tests/certification.rs` |
| `sandbox_isolation_and_egress`<br>Sandbox isolation and egress guardrails | `systems/security/execution_sandbox_envelope.ts`<br>`systems/security/egress_gateway.ts` | `config/execution_sandbox_envelope_policy.json`<br>`config/egress_gateway_policy.json` | `execution_sandbox_envelope_status`: `node systems/security/execution_sandbox_envelope.js status`<br>`repository_access_auditor_status`: `node systems/security/repository_access_auditor.js status --strict=1` | `memory/tools/tests/execution_sandbox_envelope.test.js`<br>`memory/tools/tests/egress_gateway.test.js` |
| `supply_chain_trust`<br>Supply-chain trust verification plane | `systems/security/supply_chain_trust_plane.ts` | `config/supply_chain_trust_policy.json` | `supply_chain_trust_plane`: `node systems/security/supply_chain_trust_plane.js run --strict=1 --verify-only=1` | `memory/tools/tests/supply_chain_trust_plane.test.js` |
| `key_lifecycle_and_pq`<br>Key lifecycle governance and post-quantum migration | `systems/security/key_lifecycle_governor.ts`<br>`systems/security/post_quantum_migration_lane.ts` | `config/key_lifecycle_policy.json`<br>`config/post_quantum_migration_policy.json` | `key_lifecycle_verify`: `node systems/security/key_lifecycle_governor.js verify --strict=1`<br>`post_quantum_migration_status`: `node systems/security/post_quantum_migration_lane.js status` | `memory/tools/tests/key_lifecycle_governor.test.js`<br>`memory/tools/tests/post_quantum_migration_lane.test.js` |
| `heartbeat_terms_and_repo_access`<br>Secure heartbeat endpoint + operator terms + repo access | `systems/security/secure_heartbeat_endpoint.ts`<br>`systems/security/operator_terms_ack.ts`<br>`systems/security/repository_access_auditor.ts` | `config/secure_heartbeat_endpoint_policy.json`<br>`config/operator_terms_ack_policy.json`<br>`config/repository_access_policy.json` | `secure_heartbeat_endpoint_verify`: `node systems/security/secure_heartbeat_endpoint.js verify --strict=1`<br>`operator_terms_ack_status`: `node systems/security/operator_terms_ack.js status`<br>`repository_access_auditor_status`: `node systems/security/repository_access_auditor.js status --strict=1` | `memory/tools/tests/secure_heartbeat_endpoint.test.js`<br>`memory/tools/tests/operator_terms_ack.test.js`<br>`memory/tools/tests/repository_access_auditor.test.js` |
| `state_kernel_integrity`<br>State kernel integrity and replay guardrails | `systems/ops/state_kernel.js`<br>`systems/ops/state_kernel_cutover.js` | `config/state_kernel_policy.json`<br>`config/state_kernel_cutover_policy.json` | `state_kernel_status`: `node systems/ops/state_kernel.js status`<br>`state_kernel_parity`: `node systems/ops/state_kernel.js verify-parity`<br>`state_kernel_replay_verify`: `node systems/ops/state_kernel.js replay-verify --profiles=phone,desktop,cluster`<br>`state_kernel_cutover_status`: `node systems/ops/state_kernel_cutover.js status` | `memory/tools/tests/state_kernel.test.js` |

## Verification Summary

- Layers checked: 7
- Missing paths: 0
- Missing guard checks: 0
- Contract status: PASS
- Receipt hash: `42b439e9126fd9324abddb33402943a427b8e0ca39c257eded7541a34b655031`
