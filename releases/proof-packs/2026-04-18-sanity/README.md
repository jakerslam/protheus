# Release Proof Pack

- version: 2026-04-18-sanity
- pack_root: /Users/jay/.openclaw/workspace/releases/proof-packs/2026-04-18-sanity
- required_missing: 3

| artifact | category | required | exists | sha256 |
| --- | --- | :---: | :---: | --- |
| core/local/artifacts/runtime_proof_verify_current.json | runtime_proof | yes | yes | 729344249afe43ff84e1c7e7dbe78201aeb1ff2b79c16f948179a88a1218c963 |
| core/local/artifacts/runtime_proof_harness_current.json | runtime_proof | yes | yes | f2aadc2c5df7c4d52e904969b23676be9e6076c3775c565f7a646fb172da38a6 |
| core/local/artifacts/runtime_proof_release_gate_current.json | runtime_proof | yes | yes | 198a3a61ee21038f4f8cc42bb6b291d08e4b6e99695bb0f4ac874bb2a6f38159 |
| core/local/artifacts/runtime_proof_release_metrics_rich_current.json | runtime_proof | yes | yes | 8e66264ee65ef9e438c46deb96a9b9bf368cf0b32d09fe2295b7276d2a0a06a0 |
| core/local/artifacts/adapter_runtime_chaos_gate_current.json | adapter_and_orchestration | yes | yes | 342f94422c0705a8e882c7a0b98be177ab20ad10ae6de1e079cd874f60f95e44 |
| core/local/artifacts/layer2_lane_parity_guard_current.json | adapter_and_orchestration | yes | no | missing |
| core/local/artifacts/layer2_receipt_replay_current.json | adapter_and_orchestration | yes | no | missing |
| core/local/artifacts/runtime_trusted_core_report_current.json | release_governance | yes | no | missing |
| core/local/artifacts/production_readiness_closure_gate_current.json | release_governance | yes | yes | d4a305e48ea5dc858abc79e8a09fdb73fd6f0d2cb03ccbd8f3c2841e294dc0e0 |
| core/local/artifacts/support_bundle_latest.json | release_governance | yes | yes | 7edd217962f004c94909047ecb015ee443666002c648689cf450614e039ed8b6 |
| artifacts/web_tooling_context_soak_report_latest.json | workload_and_quality | yes | yes | 514badb9fa4e6c9dbb5e6077727d15596c800aaa6ac8c89d9164cae4c544b706 |
| docs/client/reports/benchmark_matrix_run_latest.json | workload_and_quality | yes | yes | a9e801e90c46cb62ec2b2f77866db03930bcf0e2318bf74acf48cfb1eb338238 |
| client/runtime/local/state/release/scorecard/release_scorecard.json | workload_and_quality | yes | yes | 03fc120d84cc8ede3c637d6342bf5c82781e712711f63d5ed20b786ee1244c79 |
| local/workspace/reports/RUNTIME_PROOF_RELEASE_GATE_RICH_CURRENT.md | ungrouped | no | yes | ea8c8a3eccf6a55f50b4766d3a7b2a93da671763933f36b59cdbf96ac9a651f8 |
| local/workspace/reports/LAYER2_LANE_PARITY_GUARD_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/LAYER2_RECEIPT_REPLAY_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/RUNTIME_TRUSTED_CORE_REPORT_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/RELEASE_SCORECARD_CURRENT.md | ungrouped | no | no | missing |

## Category summary
- runtime_proof: present=4/4;required_missing=0
- adapter_and_orchestration: present=1/3;required_missing=2
- release_governance: present=2/3;required_missing=1
- workload_and_quality: present=3/3;required_missing=0
- ungrouped: present=1/5;required_missing=0

