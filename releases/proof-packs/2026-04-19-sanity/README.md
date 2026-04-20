# Release Proof Pack

- version: 2026-04-19-sanity
- pack_root: /Users/jay/.openclaw/workspace/releases/proof-packs/2026-04-19-sanity
- required_missing: 2

| artifact | category | required | exists | sha256 |
| --- | --- | :---: | :---: | --- |
| core/local/artifacts/runtime_proof_verify_current.json | runtime_proof | yes | yes | 729344249afe43ff84e1c7e7dbe78201aeb1ff2b79c16f948179a88a1218c963 |
| core/local/artifacts/runtime_proof_harness_current.json | runtime_proof | yes | yes | 4a4023f8c7159bf672e4fceea9a854dce7706e23c8c248cf00c7202031ad8a72 |
| core/local/artifacts/runtime_proof_release_gate_current.json | runtime_proof | yes | yes | 9b08a506684b3ee90b43858e9617e55b5c4bd5e620dc2634d2e9e6947c6b0d26 |
| core/local/artifacts/runtime_proof_release_metrics_rich_current.json | runtime_proof | yes | yes | 8e66264ee65ef9e438c46deb96a9b9bf368cf0b32d09fe2295b7276d2a0a06a0 |
| core/local/artifacts/adapter_runtime_chaos_gate_current.json | adapter_and_orchestration | yes | yes | c0700372f0d92f990b7e86625f94969d03c3699356b5fba9c609fdbfd15dd06a |
| core/local/artifacts/layer2_lane_parity_guard_current.json | adapter_and_orchestration | yes | yes | 04256ceaaf2179975e2c74f85063d8aac470c5c5482339d15cfb47afc5cf0bb5 |
| core/local/artifacts/layer2_receipt_replay_current.json | adapter_and_orchestration | yes | no | missing |
| core/local/artifacts/runtime_trusted_core_report_current.json | release_governance | yes | no | missing |
| core/local/artifacts/node_critical_path_inventory_current.json | release_governance | yes | yes | ed9d474f7aa4cf3e86a7548903960811ae2f5eec0c6ba791f4376b160c5b03be |
| core/local/artifacts/agent_surface_status_guard_current.json | release_governance | yes | yes | 666b561718135086d9b5f831a321acf33992991944361a60f47110d25ed497f9 |
| core/local/artifacts/production_readiness_closure_gate_current.json | release_governance | yes | yes | d4a305e48ea5dc858abc79e8a09fdb73fd6f0d2cb03ccbd8f3c2841e294dc0e0 |
| core/local/artifacts/support_bundle_latest.json | release_governance | yes | yes | 7edd217962f004c94909047ecb015ee443666002c648689cf450614e039ed8b6 |
| artifacts/web_tooling_context_soak_report_latest.json | workload_and_quality | yes | yes | 514badb9fa4e6c9dbb5e6077727d15596c800aaa6ac8c89d9164cae4c544b706 |
| docs/client/reports/benchmark_matrix_run_latest.json | workload_and_quality | yes | yes | a9e801e90c46cb62ec2b2f77866db03930bcf0e2318bf74acf48cfb1eb338238 |
| client/runtime/local/state/release/scorecard/release_scorecard.json | workload_and_quality | yes | yes | 03fc120d84cc8ede3c637d6342bf5c82781e712711f63d5ed20b786ee1244c79 |
| local/workspace/reports/RUNTIME_PROOF_RELEASE_GATE_RICH_CURRENT.md | ungrouped | no | yes | ea8c8a3eccf6a55f50b4766d3a7b2a93da671763933f36b59cdbf96ac9a651f8 |
| local/workspace/reports/LAYER2_LANE_PARITY_GUARD_CURRENT.md | ungrouped | no | yes | e0e1ae683cef40c4211c58e99675afcb4bc44e95a3a0a4c5b83ec99927218541 |
| local/workspace/reports/LAYER2_RECEIPT_REPLAY_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/RUNTIME_TRUSTED_CORE_REPORT_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/NODE_CRITICAL_PATH_INVENTORY_CURRENT.md | ungrouped | no | yes | da7e6707dbd59926e5aba074205034c0efcb8641e15b321fdee45794c8312a5f |
| local/workspace/reports/AGENT_SURFACE_STATUS_GUARD_CURRENT.md | ungrouped | no | yes | cb37cce039a78956c7dcfc09f54892fa12d7d5048af3666275082cc07dc02a68 |
| local/workspace/reports/RELEASE_SCORECARD_CURRENT.md | ungrouped | no | no | missing |

## Category summary
- runtime_proof: present=4/4;required=4/4;required_missing=0;required_completeness=1.000;required_min=1.000
- adapter_and_orchestration: present=2/3;required=2/3;required_missing=1;required_completeness=0.667;required_min=1.000
- release_governance: present=4/5;required=4/5;required_missing=1;required_completeness=0.800;required_min=1.000
- workload_and_quality: present=3/3;required=3/3;required_missing=0;required_completeness=1.000;required_min=1.000
- ungrouped: present=4/7;required=0/0;required_missing=0;required_completeness=1.000

