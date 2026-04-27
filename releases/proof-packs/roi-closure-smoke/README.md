# Release Proof Pack

- version: roi-closure-smoke
- pack_root: /Users/jay/.openclaw/workspace/releases/proof-packs/roi-closure-smoke
- required_missing: 0
- stale_artifacts: 0
- failed_artifacts: 16
- historical_snapshot_artifacts: 9
- summary_consistency_failures: 0
- category_required_missing_sum: 0
- category_artifact_count_sum: 212
- category_required_total_sum: 133
- release_blocking_issue_count: 25
- top_blocker_count: 20
- primary_blocker_class: mandatory_artifact_failures
- primary_blocker_artifact: core/local/artifacts/eval_regression_guard_current.json
- primary_blocker_action: repair mandatory proof artifact core/local/artifacts/eval_regression_guard_current.json
- primary_blocker_dedupe_key: release_proof_pack:mandatory_artifact_failures:core/local/artifacts/eval_regression_guard_current.json
- primary_blocker_priority_score: 850
- primary_blocker_owner: release_governance/proof_pack
- primary_blocker_target_layer: release_governance
- primary_blocker_escalation_tier: release_blocker
- primary_blocker_release_gate_effect: blocks_release_until_closed
- primary_blocker_operator_next_step: repair mandatory proof artifact core/local/artifacts/eval_regression_guard_current.json; rerun release proof-pack assembly and confirm this dedupe key disappears
- primary_blocker_triage_queue: release_blockers
- primary_blocker_lifecycle_state: candidate_open
- primary_blocker_source_artifact_count: 2
- primary_blocker_closure_verification_command: node client/runtime/lib/ts_entrypoint.ts tests/tooling/scripts/ci/release_proof_pack_assemble.ts --strict=0
- top_blockers_actionable: true
- top_blocker_actionability_failure_count: 0
- top_blocker_action_count: 3

| artifact | category | required | exists | sha256 |
| --- | --- | :---: | :---: | --- |
| core/local/artifacts/runtime_proof_verify_current.json | runtime_proof | yes | yes | 2527cb65d331fcd182d431736de620f8a0ae4bf84cc97974f5a99b45eb6c0203 |
| core/local/artifacts/runtime_proof_harness_rich_current.json | runtime_proof | yes | yes | 631d0da5facf348b40f24cd180fa9f5862a774a8b29b7f11975b61cd88c5acfb |
| core/local/artifacts/runtime_proof_release_gate_rich_current.json | runtime_proof | yes | yes | aacdc682861b4cf0da945a1f34994365f9b85e00ae5778cf762711e89c17ee39 |
| core/local/artifacts/runtime_proof_release_metrics_rich_current.json | runtime_proof | yes | yes | e6fe803ea76b55d1b08b5c671730f7ed26b8b773a3e16fa4326787ee58cbc850 |
| core/local/artifacts/runtime_proof_harness_pure_current.json | runtime_proof | yes | yes | d16c7bb67393e0f7262a9a8fe9c216ab93ca2fe9a4c040c30eba0f6d1238d679 |
| core/local/artifacts/runtime_proof_release_gate_pure_current.json | runtime_proof | yes | yes | 2dff617b4764491ec97d0a7e9cbea237191f3171f5ad6b059d6d77e91f0c5b60 |
| core/local/artifacts/runtime_proof_release_metrics_pure_current.json | runtime_proof | yes | yes | 18e56815aeb0bdd4b58c36d7440356af1dd30735030369ca0735078396378d23 |
| core/local/artifacts/runtime_proof_harness_tiny-max_current.json | runtime_proof | yes | yes | a4586ac8b52c319d640f0896f1dc775e6cdf8b111fc0e28cc2a5b6a362dd428a |
| core/local/artifacts/runtime_proof_release_gate_tiny-max_current.json | runtime_proof | yes | yes | d3fa250df7bbfb16ee93f9cd5b24c1a654bd5bcab8fb61129aaa97bfb6152df5 |
| core/local/artifacts/runtime_proof_release_metrics_tiny-max_current.json | runtime_proof | yes | yes | bdf76f96d7987dae349c1bf73cabf36fb5b1a88529a24d81b2aa276aeaa3a6cb |
| core/local/artifacts/runtime_boundedness_72h_evidence_current.json | runtime_proof | yes | yes | 8c3c271c40bd679470b31f99d9df7a5674bf75a0fd13b96339c999dae73dbc53 |
| core/local/artifacts/runtime_boundedness_profiles_current.json | runtime_proof | yes | yes | 32a30a7a9145ee4dc9c82538f9e09ea3c244653dcaa9fd1fa556cf5f98d49479 |
| core/local/artifacts/runtime_boundedness_inspect_rich_current.json | runtime_proof | yes | yes | 0839163dc4a299eb4ae88946217aff07ca9cc0305b779d0ef359e055c6ebd5c3 |
| core/local/artifacts/runtime_boundedness_inspect_pure_current.json | runtime_proof | yes | yes | 140193707a785acb4d6df4d204c55154e01f53f21422c4da156573ee74b35596 |
| core/local/artifacts/runtime_boundedness_inspect_tiny-max_current.json | runtime_proof | yes | yes | 010818c7eeb6e0236ffd080681f902fd7a1e614e5f2ddf9c29f4c20bfa03b98a |
| core/local/artifacts/runtime_boundedness_release_gate_current.json | runtime_proof | yes | yes | a27057f809d7a3c51a51c2b8f6cde75f8f304498aab1eb28b1a6bc5046b5ef1d |
| core/local/artifacts/queue_backpressure_policy_gate_current.json | runtime_proof | yes | yes | 51f1de2afddd29fbedb2eb5341db7410fbfb5c981ede002bb98bacf3bdb35229 |
| core/local/artifacts/runtime_multi_day_soak_evidence_current.json | runtime_proof | yes | yes | 1aa08ff2cc8f0238ab75a17fa16e86626daabadc5fbbf85609b4cbf464af8a41 |
| core/local/artifacts/runtime_soak_scenarios_current.json | runtime_proof | yes | yes | a4dbb0498573bfa82a93c1cc8554558c0bd42c98cc88bf2bf19e39de44214a5f |
| core/local/artifacts/runtime_proof_synthetic_canary_current.json | runtime_proof | yes | yes | 677e03070d51df60d7da011e630984e6e57da6c5f76ecd11cc486f7d6678c01b |
| core/local/artifacts/runtime_proof_empirical_release_evidence_current.json | runtime_proof | yes | yes | 8d59e21e0ab7269755120754ca48ed613c01691ffbc8001e18395adbc2d52043 |
| core/local/artifacts/runtime_proof_empirical_trends_current.json | runtime_proof | yes | yes | 7c164a732f7958a8aa59cd2ae4de56f613b0f9ca7dbc29cd387706c2398e9901 |
| core/local/artifacts/runtime_proof_empirical_profile_coverage_current.json | runtime_proof | yes | yes | 2d75a0b431ba79aa4949bf4b3fb1ea21c8021d32cd3830f7f7af6b3a360502d2 |
| core/local/artifacts/runtime_proof_empirical_source_matrix_current.json | runtime_proof | yes | yes | 5636c5f306eea2defd62823b115a496e7d9f610d7fc60cc960d57119c1f9a2eb |
| core/local/artifacts/runtime_proof_empirical_profile_gate_current.json | runtime_proof | yes | yes | 0660ed47f68611728099472ae62447a210b05a98ed68172842c37776a6da67f2 |
| core/local/artifacts/runtime_proof_empirical_profile_gate_failures_current.json | runtime_proof | yes | yes | b14d79700d3d888fe8b81426f6b96f570782d8a6c6a0993a4d42082f47f48e61 |
| core/local/artifacts/runtime_proof_empirical_profile_readiness_current.json | runtime_proof | yes | yes | d0467d7710f3ab1b49be231f80fafeb8bb74916d786d02b56149cd4273427db2 |
| core/local/artifacts/runtime_proof_empirical_minimum_contract_current.json | runtime_proof | yes | yes | 35bf7e04ea79ca18cba45db5b50382776c80c23a167592cb10a4d423903ebaf1 |
| core/local/artifacts/runtime_proof_reality_guard_current.json | runtime_proof | yes | yes | 24bc155b79074bf99d7f7e62392d22130bd6c71684f7a8e3a28e05710e2bfcfc |
| core/local/artifacts/release_proof_checksums_current.json | runtime_proof | yes | yes | a2d203cbbd9f8f6e4962bfb342495e2d20c168ebe8c34542d8844bfa4f3972a2 |
| core/local/artifacts/gateway_runtime_chaos_gate_current.json | adapter_and_orchestration | yes | yes | 2ced78e2d60909b81322ed8fa25b8f38325c2dd6f9a04d0760b31f6d0b3f75fe |
| core/local/artifacts/gateway_support_levels_current.json | adapter_and_orchestration | yes | yes | 2e82be88d0f2e581918169023a23e5557ec955aca2c350633eceb1e56a2c7440 |
| core/local/artifacts/gateway_manifest.json | adapter_and_orchestration | yes | yes | 5027ea24deb5fd17bcf4f82152adc40f4fbf5d8fbbbcc1d1ed4d6bb1ab24ecfd |
| core/local/artifacts/gateway_status_manifest_current.json | adapter_and_orchestration | yes | yes | 5027ea24deb5fd17bcf4f82152adc40f4fbf5d8fbbbcc1d1ed4d6bb1ab24ecfd |
| core/local/artifacts/gateway_graduation_status_snapshot_current.json | adapter_and_orchestration | yes | yes | a2a393bf2bbcd361d3ab8582277e520ff0eb105493d3627917266b2e5a7dfd50 |
| core/local/artifacts/gateway_support_matrix_current.json | adapter_and_orchestration | yes | yes | 03c94f287eb8a45989622204fa24e26fb4775d51d183d9525db7f0327e5d177d |
| core/local/artifacts/gateway_quarantine_recovery_proof_current.json | adapter_and_orchestration | yes | yes | 8f034d3de4f5e9b546cf0a40288c8df2eccf1bcc9253dce9da5b31815fb1baca |
| core/local/artifacts/layer2_parity_matrix.json | adapter_and_orchestration | yes | yes | f313dc6c3acb94166a32fdc8f2074279d98d70b868f51d00354484f34e826b5c |
| core/local/artifacts/layer2_lane_parity_guard_current.json | adapter_and_orchestration | yes | yes | 1d8af264129649bf2a74d42a14643af3b4984c0a8c0c6032b18a56d5f67868f3 |
| core/local/artifacts/layer2_receipt_replay_current.json | adapter_and_orchestration | yes | yes | 78750f839a5c3590d787ee1df0b1f454735b12b63077d2d2e093a0d0516c2867 |
| core/local/artifacts/runtime_trusted_core_report_current.json | release_governance | yes | yes | b68822679c9e704c68ad9833aad7a57481d6a5899dd2532653876e00812726d9 |
| core/local/artifacts/kernel_sentinel_auto_run_current.json | release_governance | yes | yes | 6016c763752288f6fcfd9e0de7b9559b495b65c8d307ed08d22796e55abb797d |
| local/state/kernel_sentinel/kernel_sentinel_report_current.json | ungrouped | yes | yes | e9a4148676e23457b9f8e3b05ff4e9faff93647d2f554e9b1f002c264a43fa3d |
| local/state/kernel_sentinel/kernel_sentinel_verdict.json | ungrouped | yes | yes | e98ae99f3446310ae3942336044e8ddd8fd18c5466e7749d0c29fd6dd7082d80 |
| local/state/kernel_sentinel/rsi_readiness_summary_current.json | ungrouped | yes | yes | e0717ae4f07d057cd108815e4c1e548476f6184d996b608866532c44f9514a9e |
| local/state/kernel_sentinel/feedback_inbox.jsonl | ungrouped | yes | yes | 30a1bb924055610a8ec5f828ecedcee8f4ed567215e0d9fd1dfe5e47be55f384 |
| local/state/kernel_sentinel/trend_history.jsonl | ungrouped | yes | yes | e7beb6e2af2b9940aad7aa7ad15b099dc6f949c3b59eeb968c06685727198a59 |
| local/state/kernel_sentinel/sentinel_trend_report_current.json | ungrouped | yes | yes | 14d10daf8dbf78fc9e565a8c08613d2fd42cc34d2835089efb31a9a96b0696fa |
| local/state/kernel_sentinel/top_system_holes_current.json | ungrouped | yes | yes | b600b478527d43ca5d6ffd19ba348af3bcf51ebc588f087e2e2e6c10a686653e |
| local/state/kernel_sentinel/issues.jsonl | ungrouped | yes | yes | 36c39c912f5e7420fefd4b5d0903ca9fc7bd672966ea256fcbd67e3907e0092e |
| local/state/kernel_sentinel/suggestions.jsonl | ungrouped | yes | yes | 10e1c35eeca8fe1166bb097229f56a68b3ff691cbc7f0ec451304ab855f3b42e |
| local/state/kernel_sentinel/automation_candidates.jsonl | ungrouped | yes | yes | a8c47468f660a2bb463f981f20becf1556249c71019f300d738c352a0ed37bef |
| local/state/kernel_sentinel/daily_report.md | ungrouped | yes | yes | 265be7f48063fb64fdd36de6b3a8459551fb13e96b5ae5971be15a08742b99e5 |
| core/local/artifacts/kernel_nexus_coupling_guard_current.json | release_governance | yes | yes | 1c743261dc0e35b23d7cb9ec85443e5d89b95c98197f12c1145ca9047f1ab598 |
| core/local/artifacts/architecture_nexus_required_artifact_guard_current.json | release_governance | yes | yes | 5eaa1eb7d66d6fcd069b93e795a21431818ea1f1a715cd11b40bf132feb4ca3f |
| core/local/artifacts/layer3_contract_guard_current.json | release_governance | yes | yes | fee3f15033f580f2300d77cf235b6db3773b80e6d5659677fb4ab08c25a2cad0 |
| core/local/artifacts/node_critical_path_inventory_current.json | release_governance | yes | yes | 1fc7d371221be866562e8b833b991327e585c4d92590d733299eff289cf52d5c |
| core/local/artifacts/agent_surface_status_guard_current.json | release_governance | yes | yes | e8a9b21a8e4ff175cb5e735018001cc37c10fa3a7887694d53b9911fcf0b5736 |
| core/local/artifacts/production_readiness_closure_gate_current.json | release_governance | yes | yes | 89f52c6641493f3685ab07110f1ba68a105e9185481f8b41bb6b40984b806824 |
| core/local/artifacts/production_release_gate_closure_audit_current.json | release_governance | yes | yes | 0b8a3ab036a1115306f02f59400447844b3818c6e7fafa58df71dbed548b44d0 |
| core/local/artifacts/support_bundle_latest.json | release_governance | yes | yes | a379893fe96a0217583e7ba0d6cc662f72106a7d6b1fc5970869abf359483066 |
| artifacts/web_tooling_context_soak_report_latest.json | workload_and_quality | yes | yes | 86479220df24865432372677f8f56f85bcd0a1977524abb30bfd1bdcf6b5d0eb |
| artifacts/web_tooling_reliability_latest.json | workload_and_quality | yes | yes | a4af0dff043baa2eeff70e863cf444f9e2218ae5734237bb25a56b197de1b87a |
| artifacts/workflow_failure_recovery_latest.json | workload_and_quality | yes | yes | 768a1f9fdc27b5034e6b717d2f9f3f32c2efba8cf730824653ac5fd104e75ce1 |
| artifacts/workspace_tooling_context_soak_report_latest.json | workload_and_quality | yes | yes | 00af52807ccba933828904e3f376619e25d9c7af7b752b0015fc13045ad85dfc |
| artifacts/workspace_tooling_soak_report_latest.json | workload_and_quality | yes | yes | 00af52807ccba933828904e3f376619e25d9c7af7b752b0015fc13045ad85dfc |
| core/local/artifacts/workspace_tooling_context_soak_report_latest.json | workload_and_quality | yes | yes | 00af52807ccba933828904e3f376619e25d9c7af7b752b0015fc13045ad85dfc |
| core/local/artifacts/web_tooling_reliability_current.json | workload_and_quality | yes | yes | cf860326a6ce52289307fd83f17d400e695d3ae1540e02a2afd83c319b37e1dc |
| core/local/artifacts/web_retrieval_reliability_closure_guard_current.json | workload_and_quality | yes | yes | 06ad1272c925f4d28b48f015a058a9ea56b144cb4a754804082b965169f11790 |
| core/local/artifacts/web_conduit_openclaw_media_closure_guard_current.json | workload_and_quality | yes | yes | 3b9d2ccc0fcea40818dc0500db075c1129240f0e00deea30fe7a777d4837f27f |
| core/local/artifacts/workflow_failure_recovery_current.json | workload_and_quality | yes | yes | bd2a0970dc69b3f86da46dda6d56db46527bfc2508135874ea63770c08737f19 |
| core/local/artifacts/workspace_tooling_context_soak_current.json | workload_and_quality | yes | yes | 00af52807ccba933828904e3f376619e25d9c7af7b752b0015fc13045ad85dfc |
| core/local/artifacts/workspace_tooling_release_proof_current.json | workload_and_quality | yes | yes | 3d68b9226f1c1f4894a364b3778beb6b51fba4ee10aae0f02c132b70eb72e085 |
| docs/client/reports/benchmark_matrix_run_latest.json | workload_and_quality | yes | yes | bb326350837a01b527fc7f9ca25736255cdb8405004c76a1f57858534c2a6d95 |
| client/runtime/local/state/release/scorecard/release_scorecard.json | workload_and_quality | yes | yes | 4da59b1fe7e96e057732c7ba1185026996549fef672274764575e0e0c16d82fe |
| core/local/artifacts/shell_truth_leak_guard_current.json | release_governance | yes | yes | 6f7de146d56cb09b41947920cfecdb978e43c44159dd4cfabe9e56639818ae35 |
| core/local/artifacts/terminology_transition_inventory_current.json | release_governance | yes | yes | b2f29560448aa37da10fb63b66805e9fa548ba71529b68bf868208af62df9801 |
| core/local/artifacts/srs_same_revision_guard_current.json | release_governance | yes | yes | c4f772b70cb9726ea400813fbe16df2a360c488f528a48ac4a2beae83eaf9c33 |
| core/local/artifacts/runtime_closure_board_guard_current.json | release_governance | yes | yes | 18c81637281547d011ded79bc4bf40e7040b0dba64054c7f124d347d05199c0e |
| core/local/artifacts/runtime_closure_feature_alignment_guard_current.json | release_governance | yes | yes | 30843480390dc0ce739110fd1a1ca5b8a88d8b1d5233872e0313c58641e2cc02 |
| core/local/artifacts/capability_proof_burden_guard_current.json | release_governance | yes | yes | 89771698cd6cd395be6bc22697e521fb7f06e1603e1fc38d52032743b2752e23 |
| core/local/artifacts/windows_installer_contract_guard_current.json | release_governance | yes | yes | d7ff6b61e2c71d3de02b04017b4f44f0183aaeef34fc80499930d7ff2d50936a |
| core/local/artifacts/windows_install_reliability_current.json | release_governance | yes | yes | 26618f1a4342a339cfcc3c7c5d56399a271f7da64de3b6a407d85f0496966c36 |
| core/local/artifacts/installer_reliability_closure_guard_current.json | release_governance | yes | yes | 080b6db85c0494621629ae36ec7840882f9f77db454100b135028d523c3dff62 |
| core/local/artifacts/srs_todo_section_guard_current.json | release_governance | yes | yes | 4ea631399ef4252a678831adc04d338c565eb22ae8b8dbb4aab46b907ef6d0a5 |
| core/local/artifacts/test_maturity_registry_current.json | release_governance | yes | yes | a09c57d43ec3b958ad80081eb6b159665c244c6f7b9825eae342872b0dea1808 |
| core/local/artifacts/test_maturity_retirement_backlog_current.json | release_governance | yes | yes | 45e7e66b956ee1a1393e6b3fad010d0f40442ebcae53761295b6f85d92848441 |
| core/local/artifacts/parity_end_to_end_replay_current.json | workload_and_quality | yes | yes | 01d08d614f31fc5430ba02d0c1d58ab40402fbfd1863b89ebbb52e204c4574fc |
| core/local/artifacts/parity_trend_current.json | workload_and_quality | yes | yes | 67d68a159d2c8cfa588f9b58cea1f45cfaca1de6b3f348003266b5fb77b172ea |
| core/local/artifacts/parity_release_gate_current.json | workload_and_quality | yes | yes | e9746305fc4c5d3c508477f589830e25aa8011b3dadbb168a108a2716ce2c0f8 |
| core/local/artifacts/gem_live_provider_smoke_current.json | workload_and_quality | yes | yes | 652edc15d9255965da7a115a2ad5231815662864f0083be2ce59d2c5c2f347df |
| core/local/artifacts/gem_memory_durability_current.json | workload_and_quality | yes | yes | d1c724bdb2c871e1a5fd41adcd17864c26277320c99c7a54ea76496b181487b5 |
| core/local/artifacts/memory_continuity_closure_guard_current.json | workload_and_quality | yes | yes | 872fae805a68ade5ca9e9da9dfa25ebe50b88a8af45d286ff09dbd7e74ee52c5 |
| core/local/artifacts/knowledge_graph_query_acceleration_closure_guard_current.json | workload_and_quality | yes | yes | f6d539c8fe99a3fbe496d1a4f23cb3b5c959e4985a8527e31ef88c50c1c4ccc3 |
| core/local/artifacts/gem_subagent_route_contract_current.json | workload_and_quality | yes | yes | 9865c6896e853391ac2580d17a9600d8f65016d49e9247bbc847b08300c2dcbb |
| core/local/artifacts/gem_feedback_closure_guard_current.json | workload_and_quality | yes | yes | 14f17138544ecf43664bae5d5f7143051ab847de762feb9a9572c8eaa2e4ed2f |
| core/local/artifacts/eval_agent_chat_monitor_guard_current.json | workload_and_quality | yes | yes | aa75df357aff6fc9a93c9d0b2ba1aebf615c6d30ff1ea143b0ed29307cc30d36 |
| core/local/artifacts/eval_quality_metrics_current.json | workload_and_quality | yes | yes | d1ebb210aa824b880e9f6784f71369afe2d75849417f2f32060f8aa24b0091da |
| core/local/artifacts/eval_monitor_slo_current.json | workload_and_quality | yes | yes | 2093218898bd66504eae4d11b951dc9cba362f162da7e07286b427a2ebb22a14 |
| core/local/artifacts/eval_reviewer_feedback_weekly_current.json | workload_and_quality | yes | yes | cdd2d68d6f8c783ac7c9e121f182d378a96a229582d241f495f24768a583a780 |
| core/local/artifacts/eval_quality_gate_v1_current.json | workload_and_quality | yes | yes | 54e18b21398637f05f289c56564f3b624a85e0f7e5ad3bede09519b893cf7fbd |
| core/local/artifacts/eval_judge_human_agreement_current.json | workload_and_quality | yes | yes | 65d80f860e0b81652aab1421c480842bba584f79603594ce52ebef00930214ae |
| core/local/artifacts/eval_adversarial_guard_current.json | workload_and_quality | yes | yes | 77ab16132bd993452581db6e4eec7b0e917bcb463be6195fbb84fa932ea2a01c |
| core/local/artifacts/eval_issue_filing_guard_current.json | workload_and_quality | yes | yes | 22e27d01844ebac8deb77b08b70e8805d828790405ffed9ae9c4c50957161b66 |
| core/local/artifacts/eval_issue_resolution_current.json | workload_and_quality | yes | yes | 5c96b39da30d684a49d7391282265e6308f9530c5ac2ba57ef52a2305fcca97e |
| core/local/artifacts/eval_runtime_authority_guard_current.json | workload_and_quality | yes | yes | 5b3a400ecf24bf9b25bce380cdea62a08af35ba56035c2ed16321b100a6c54a8 |
| core/local/artifacts/eval_regression_guard_current.json | workload_and_quality | yes | yes | d6a598d8d55f4b61326978105746970cc66e17671aa956d9a787b9902ac4c7be |
| core/local/artifacts/eval_feedback_router_current.json | workload_and_quality | yes | yes | 3e04b99369eb7090f7c9f0f7b14e7f6b52d340c32bbd71ec8fc84a40afc68742 |
| core/local/artifacts/eval_autopilot_guard_current.json | workload_and_quality | yes | yes | f3dc76d5eabde7639d7496176e44f2f75052971869470474190a38fad226504e |
| core/local/artifacts/orchestration_planner_quality_guard_current.json | adapter_and_orchestration | yes | yes | c2315f72aa95d7a66502a189285056928d6fff6d73a099040a77b03044412b19 |
| core/local/artifacts/orchestration_runtime_quality_guard_current.json | adapter_and_orchestration | yes | yes | ef75fa114f9c16e186cd360b0c95f60c681b2ac4e4819a0f8897643a3758695f |
| core/local/artifacts/orchestration_gateway_fallback_guard_current.json | adapter_and_orchestration | yes | yes | aeead617a3bd649a8cc08de796a5fbcc23cb130b814b0196eb7a71c59d7e6223 |
| core/local/artifacts/orchestration_workflow_contract_guard_current.json | adapter_and_orchestration | yes | yes | a07a7d3c0ad6e7148188253970c9074acbee4f23a22981d9d81ada1b12912384 |
| core/local/artifacts/orchestration_quality_closure_guard_current.json | release_governance | yes | yes | 9fa218d4d08848dad1f57b80d5626654b0fcdfa87b3e0104898d887fdf449691 |
| core/local/artifacts/typed_probe_contract_matrix_guard_current.json | adapter_and_orchestration | yes | yes | ffec91d1c6b508eb9c5271c9e196ef38ecbff7d5a827b379d6301559ee8646c5 |
| core/local/artifacts/tool_route_decision_current.json | adapter_and_orchestration | yes | yes | f16e975fd65f9baf7e7443720b0900621ff70f051e332856d7aa8da60f5babe2 |
| core/local/artifacts/transport_convergence_guard_current.json | release_governance | yes | yes | c93d7647454d739aadff546ad725202ecda5aad2578521c7dcfb4ee244880f2a |
| core/local/artifacts/transport_spawn_audit_current.json | release_governance | yes | yes | 565717974feaf6b6c39bf374a887080f61831302a33931556c55a1767be1feca |
| core/local/artifacts/release_policy_gate_current.json | release_governance | yes | yes | d9205fd2bdd3f55113db582d20d09d5921aed8f96466b4f6c8d0d3572fdced65 |
| core/local/artifacts/tooling_task_fabric_closure_guard_current.json | release_governance | yes | yes | 97f1e0fb983a939d91c627b5ace34102195238f37d515a77753700476f38bff8 |
| core/local/artifacts/chat_rendering_experience_guard_current.json | workload_and_quality | yes | yes | a112bd56672abcd498f7ad7fcf39d25d002534627b8ba82de0414e8ff455989a |
| core/local/artifacts/client_deauthority_closure_guard_current.json | workload_and_quality | yes | yes | 1dffe900ba213af4e2110c53ae56f6b70629e6db0a262e4dee84c37f96c88d9a |
| core/local/artifacts/dx_public_facade_closure_guard_current.json | workload_and_quality | yes | yes | 4be998fd04761e43a70264b630ca3ff9745451039554d4dde953ae8ae41c6064 |
| core/local/artifacts/effective_loc_metric_current.json | workload_and_quality | yes | yes | ba0d77e34417fee7f2c6706be891f5009f41154e042593031aa85026414ded5b |
| core/local/artifacts/effective_loc_metric_contract_guard_current.json | workload_and_quality | yes | yes | 4ab0a59daf40eaeffae190c8976b7525545c7b53959e61c6bc1f943326b643a4 |
| core/local/artifacts/release_contract_gate_current.json | release_governance | yes | yes | e11278e4d9526ee6cdeb8787f4debf1290dc5734942ff87dd50e9717283f059b |
| core/local/artifacts/issue_candidate_contract_guard_current.json | release_governance | yes | yes | 7f123932cdd0eb49e2cd439e0984fc13fd1ec05ce6163c54fb43a6275d17c071 |
| core/local/artifacts/issue_candidate_backlog_current.json | release_governance | yes | yes | ad4ccddf44f622d3165b36635747ca91e2b227781296d1a6d18f5ae1e5d4ecd7 |
| core/local/artifacts/trust_zone_guard_current.json | release_governance | yes | yes | 3e3f3dd498a36bb379c9c90264eccd306c30781df2113d59804add974e7a4597 |
| core/local/artifacts/self_modification_guard_current.json | release_governance | yes | yes | 4cd4f793edc78a2d60225c5bae28b2fe9be8e2710ebbcab788923b83711ba5ad |
| core/local/artifacts/incident_operations_governance_gate_current.json | release_governance | yes | yes | 2e4179258a42e2090c3f21285acf9a6607b9e7841d7791480e80c77f50cb4f75 |
| core/local/artifacts/incident_governance_closure_guard_current.json | release_governance | yes | yes | e088dcf32ca07cb93b0b8f83e24e64db121ed480960b728646a5a5efa0825441 |
| core/local/artifacts/rust_core_file_size_gate_current.json | release_governance | yes | yes | 0044550cede60480bfe060c1e85d72f5adb7476ca58991e3c6665e71dfa03eb0 |
| local/workspace/reports/RUNTIME_PROOF_RELEASE_GATE_RICH_CURRENT.md | ungrouped | no | yes | b55f4c560b30487014ea58cf668dc6a0e62af6f92265fed3654a75a2dc109381 |
| local/workspace/reports/LAYER2_LANE_PARITY_GUARD_CURRENT.md | ungrouped | no | yes | 7b0a2ef0138c7f086bf363a51c156f75bfd1468457e9b3b672e82714f6af157d |
| local/workspace/reports/LAYER2_RECEIPT_REPLAY_CURRENT.md | ungrouped | no | yes | 436dfef007dd047f7bd9ad62e6aee48a3994a68ce43e3097f0e9512ae85121bd |
| local/workspace/reports/RUNTIME_TRUSTED_CORE_REPORT_CURRENT.md | ungrouped | no | yes | ba0df2a9e9796bf717b08ad3d2fd9d651de6c3a08321c1a0b68ace1f623f2763 |
| local/workspace/reports/GATEWAY_RUNTIME_CHAOS_GATE_CURRENT.md | ungrouped | no | yes | 7c37a16cfe448afcd1fb8f055386466fbd009acda1464912963261d180453cd7 |
| local/workspace/reports/GATEWAY_STATUS_MANIFEST_CURRENT.md | ungrouped | no | yes | 7db07c3a7306dcc212b5ba19c650cb81997301d297281083415707e7125393ae |
| local/workspace/reports/GATEWAY_GRADUATION_STATUS_SNAPSHOT_CURRENT.md | ungrouped | no | yes | ff3dd08d1ff91ac444c0e47f56d37da59bd566952bef01e4ea7b922a83834612 |
| local/workspace/reports/LAYER3_CONTRACT_GUARD_CURRENT.md | ungrouped | no | yes | f47acec6ccdca35e0dee5e290ec63d568f90292f5a72e0fa25aabbf834f97ee1 |
| local/workspace/reports/NODE_CRITICAL_PATH_INVENTORY_CURRENT.md | ungrouped | no | yes | f86215230ae4786760c0d0b9fd930566cd279a4e67a5c9b06235e6ce4342f2c2 |
| local/workspace/reports/AGENT_SURFACE_STATUS_GUARD_CURRENT.md | ungrouped | no | yes | ec016ed1ab04453e627fe67e8adee86d00cc851aeccb18810a53478e04658110 |
| local/workspace/reports/RUNTIME_BOUNDEDNESS_INSPECT_RICH_CURRENT.md | ungrouped | no | yes | dffff384cbc733b474432ae5daf5f30c5afa9f849325645e9c041dacd0a3e54a |
| local/workspace/reports/RUNTIME_BOUNDEDNESS_INSPECT_PURE_CURRENT.md | ungrouped | no | yes | 82cfe27d75dfcb66c6782ba3a92ac2b649221ec9ffa283e8a3e3ddfc4e0f4475 |
| local/workspace/reports/RUNTIME_BOUNDEDNESS_INSPECT_TINY-MAX_CURRENT.md | ungrouped | no | yes | 702bc08c6f1f6726aa229308ba2e61d6e0ce149fc147188559c1b138eaeb0177 |
| local/workspace/reports/RUNTIME_PROOF_EMPIRICAL_RELEASE_EVIDENCE_CURRENT.md | ungrouped | no | yes | 51fe2b8782c842dd9b51a18f79882f8c30aac8b9bab672ebdae10378e25dec32 |
| local/workspace/reports/RUNTIME_PROOF_EMPIRICAL_TRENDS_CURRENT.md | ungrouped | no | yes | ff333aebb305c11a398beb53b7725cba395afe76275f77bd63ddcbce644e40ec |
| local/workspace/reports/RUNTIME_PROOF_EMPIRICAL_PROFILE_COVERAGE_CURRENT.md | ungrouped | no | yes | f3baed4d9f8e5ae8a7563bd7a8f1d73b601172d58181e66b37eecaf6a2dd9166 |
| local/workspace/reports/RUNTIME_PROOF_EMPIRICAL_SOURCE_MATRIX_CURRENT.md | ungrouped | no | yes | 631543cec49d83c8da84fab32823129b6f3530d0d8dda36abea680e2d620929c |
| local/workspace/reports/RUNTIME_PROOF_EMPIRICAL_PROFILE_GATE_CURRENT.md | ungrouped | no | yes | 25d7f1c0a1b353cb4f54407ee5fa573912d1dae0efe16f78e1db8f4c210cb4d9 |
| local/workspace/reports/RUNTIME_PROOF_EMPIRICAL_PROFILE_GATE_FAILURES_CURRENT.md | ungrouped | no | yes | cc393e7afe151c73cfb9ca4c12d1ae388bea8141dd54b2d630298fa920419a9c |
| local/workspace/reports/RUNTIME_PROOF_EMPIRICAL_PROFILE_READINESS_CURRENT.md | ungrouped | no | yes | 2bf51eb334dc5c33ec7178df74d825bd33d57beb838f203824a42535115561d5 |
| local/workspace/reports/RUNTIME_PROOF_EMPIRICAL_MINIMUM_CONTRACT_CURRENT.md | ungrouped | no | yes | 66c81fe4061d2c1dd000e60fab1838088ac1f24fba27eec05277149e589c14e7 |
| local/workspace/reports/RUNTIME_SOAK_SCENARIOS_CURRENT.md | ungrouped | no | yes | 6906de0f2eb6ec87ea470d212baad9c697a00271e0a36ccaf13f62a954fa5f44 |
| local/workspace/reports/RUNTIME_PROOF_REALITY_GUARD_CURRENT.md | ungrouped | no | yes | cca567dc3a4bafe27c700a6771ef0e2ccf6dc115fd4509b977f2c4ab435b8623 |
| local/workspace/reports/WEB_TOOLING_RELIABILITY_CURRENT.md | ungrouped | no | yes | 0632e2e064528ece1e75d41603e540ea2882f8b73b695eef995377857900cbf8 |
| local/workspace/reports/WORKFLOW_FAILURE_RECOVERY_CURRENT.md | ungrouped | no | yes | 401aa2879cf8b4032621159849ea26ceb3e60c415bc83ba7c8696e039debb24c |
| local/workspace/reports/WORKSPACE_TOOLING_CONTEXT_SOAK_CURRENT.md | ungrouped | no | yes | ce3a700f83bcecd29550bb55cb93efffaca825674a9660650ef0cb2465f816f4 |
| local/workspace/reports/WORKSPACE_TOOLING_RELEASE_PROOF_CURRENT.md | ungrouped | no | yes | 8a447bbfc2719e088b3574c0698456959665393ad99be6d953f1f5e75b1c6b5f |
| core/local/state/ops/runtime_proof_empirical_history.jsonl | ungrouped | no | yes | 91747629dcbdea287477300b0f48585624c431adf1659fad43f6d95212de507d |
| local/workspace/reports/RELEASE_SCORECARD_CURRENT.md | ungrouped | no | no | missing |
| local/workspace/reports/PRODUCTION_RELEASE_GATE_CLOSURE_AUDIT_CURRENT.md | ungrouped | no | yes | a16f7787f2d70ac5650f7070a41ae629cdb27be907e90c070914b8bc8a023eeb |
| local/workspace/reports/SHELL_TRUTH_LEAK_GUARD_CURRENT.md | ungrouped | no | yes | 26491448c4989c6b1e8513809c509520a4d9e81df0189e637e91fddf04249a2c |
| local/workspace/reports/TERMINOLOGY_TRANSITION_INVENTORY_CURRENT.md | ungrouped | no | yes | d3f5e6751fc3394cceaf4ec4db858f905418d030d3d7b0be052e4f3007860704 |
| local/workspace/reports/SRS_SAME_REVISION_GUARD_CURRENT.md | ungrouped | no | yes | 9f5b6b73a76d980ab4650fdd671e52344b232e2ac3855aeaaab89d3dce8c0174 |
| local/workspace/reports/RUNTIME_CLOSURE_BOARD_GUARD_CURRENT.md | ungrouped | no | yes | 36ee805e339090c9b85dd166da1481320d2dbb69ce8d76e14f5e898c189b6e9a |
| local/workspace/reports/RUNTIME_CLOSURE_FEATURE_ALIGNMENT_GUARD_CURRENT.md | ungrouped | no | yes | 3bbd3706739870c48c69309ce063ac78065769a4d6a120d13857c5a816aad0a2 |
| local/workspace/reports/CAPABILITY_PROOF_BURDEN_GUARD_CURRENT.md | ungrouped | no | yes | 742a7169c80ccee13310dd3211557f50d81e84325c2993d8babc295a977e69a3 |
| local/workspace/reports/WINDOWS_INSTALL_RELIABILITY_CURRENT.md | ungrouped | no | yes | a9512c44d1fd45d8221b6e619cc6a30bd30f5b508df92f6c6f27fc40def134a4 |
| local/workspace/reports/WINDOWS_INSTALLER_CONTRACT_GUARD_CURRENT.md | ungrouped | no | yes | faeeb24f5fbc17cd709a4df9aa52d155018f9cfa89c1db69d59839d634e06482 |
| local/workspace/reports/SRS_TODO_SECTION_GUARD_CURRENT.md | ungrouped | no | yes | 6817d80b9168d41c302200a77e7cb65ec122e3a6b83f9dea554de6fdd6815aa5 |
| local/workspace/reports/EVAL_QUALITY_METRICS_CURRENT.md | ungrouped | no | yes | 070c0e69601979796b0199d1498938eb62ff3d63324e2d7b77db7426de8be1fb |
| local/workspace/reports/EVAL_MONITOR_SLO_CURRENT.md | ungrouped | no | yes | 3df0fcae55ed65fc2eccd1bc20bf93e239fbab8c63b695ac39a8ccee3d3f070c |
| local/workspace/reports/EVAL_REVIEWER_FEEDBACK_WEEKLY_CURRENT.md | ungrouped | no | yes | cbb3032c653f354c9e2686893e5640cb93e49f924989dcd8155c54a159af75e0 |
| local/workspace/reports/EVAL_JUDGE_HUMAN_AGREEMENT_CURRENT.md | ungrouped | no | yes | 216fa0e479ada0bbc5990ba03607ae992f325f448a7545e8d5ec657bfe0f5b61 |
| local/workspace/reports/EVAL_ADVERSARIAL_GUARD_CURRENT.md | ungrouped | no | yes | 0c843a98facf4b6132e44ee191a2d9baaa6d0ddbfd5fc851b3aca3bb6ba2a303 |
| local/workspace/reports/EVAL_ISSUE_FILING_GUARD_CURRENT.md | ungrouped | no | yes | b57fc5296927d2d0c2a8e7fa048a0c3d3cfbd119441574a56bb3b8c3e20924f5 |
| local/workspace/reports/EVAL_ISSUE_RESOLUTION_CURRENT.md | ungrouped | no | yes | 44db981e6656f271c37556ccc7426cede19ff16fe1adca1949b811060d67e5ba |
| local/workspace/reports/EVAL_QUALITY_GATE_V1_CURRENT.md | ungrouped | no | yes | 02df8700e7b55d538d244bb33ada3bd53eca1875ac5730071b1df90669e52d57 |
| local/workspace/reports/EVAL_RUNTIME_AUTHORITY_GUARD_CURRENT.md | ungrouped | no | yes | 3ce984f20b5ccc3028c46ba3e04beba6ba24f5cf38cfa71df7a491296d04eb9f |
| local/workspace/reports/EVAL_REGRESSION_GUARD_CURRENT.md | ungrouped | no | yes | 050687eae9e6c12895b6889a9a3f660a8402a74a18ee57982a21cf19bbd75f79 |
| local/workspace/reports/EVAL_FEEDBACK_ROUTER_CURRENT.md | ungrouped | no | yes | 05fc81e27937407dfe07cb9ab6ef76eeef5056789bdcefb80201c90df50160b6 |
| local/workspace/reports/KERNEL_NEXUS_COUPLING_GUARD_CURRENT.md | ungrouped | no | yes | 511ada34b069c154f2f6ae299b755ecf4c2b2a7a349adb874aaeac82d22c47aa |
| local/workspace/reports/ARCHITECTURE_NEXUS_REQUIRED_ARTIFACT_GUARD_CURRENT.md | ungrouped | no | yes | be1feb5a7e5f21b1d65883ada9f2786859397c34665676d25d6eddbd264f47bf |
| local/workspace/reports/PARITY_END_TO_END_REPLAY_CURRENT.md | ungrouped | no | yes | 8805e445a7e955d62eafde3badc21ecec38cae48d949682a5a017cea5d64a298 |
| local/workspace/reports/PARITY_TREND_CURRENT.md | ungrouped | no | yes | dd4d060fb2b91df98658248d938deb531ec8daaa21978b9e6cae23ad2ad0f0f3 |
| local/workspace/reports/PARITY_RELEASE_GATE_CURRENT.md | ungrouped | no | yes | e2aa0206203d1f322199965fb3fae0b7a2fe481848201b2658ec4fc2750543c7 |
| local/state/ops/parity/parity_trend_history.jsonl | ungrouped | no | yes | 76525d0e5a4d7f63026f75602128c7b4c911d88cbf685d70ee5bd58f70329fc3 |
| local/workspace/reports/GEM_LIVE_PROVIDER_SMOKE_CURRENT.md | ungrouped | no | yes | 2cf487f0df6555f15a4259079aa830bdcbbf57f882226d99f2abb4d4353a0302 |
| local/workspace/reports/GEM_MEMORY_DURABILITY_CURRENT.md | ungrouped | no | yes | 7093d7a1714bd7ff14e7357439f9bd1aedb79ba06e9f519cefb81cc922d7fb89 |
| local/workspace/reports/GEM_SUBAGENT_ROUTE_CONTRACT_CURRENT.md | ungrouped | no | yes | 9cb8f8f0b2783b7295daf3ca0e0937543e27a26493c927375b3f4ff506b3143b |
| local/workspace/reports/GEM_FEEDBACK_CLOSURE_GUARD_CURRENT.md | ungrouped | no | yes | 82d19daf249f0fa402ea293d1363eae18dc864d44677ee5acf0d6a25cb163d82 |
| local/workspace/reports/EVAL_AGENT_CHAT_MONITOR_GUARD_CURRENT.md | ungrouped | no | yes | a43f471a26a0bb202b62d0907dfd60af26b0e6c0f8171a4c6863df86b48280d4 |
| local/state/ops/gem_live_provider_smoke/latest.json | ungrouped | no | yes | 652edc15d9255965da7a115a2ad5231815662864f0083be2ce59d2c5c2f347df |
| local/state/ops/gem_memory_durability/latest.json | ungrouped | no | yes | d1c724bdb2c871e1a5fd41adcd17864c26277320c99c7a54ea76496b181487b5 |
| local/state/ops/gem_subagent_route_contract/latest.json | ungrouped | no | yes | 9865c6896e853391ac2580d17a9600d8f65016d49e9247bbc847b08300c2dcbb |
| local/state/ops/gem_feedback_closure_guard/latest.json | ungrouped | no | yes | 14f17138544ecf43664bae5d5f7143051ab847de762feb9a9572c8eaa2e4ed2f |
| local/state/ops/eval_agent_chat_monitor/latest.json | ungrouped | no | yes | aa75df357aff6fc9a93c9d0b2ba1aebf615c6d30ff1ea143b0ed29307cc30d36 |
| local/state/ops/eval_agent_chat_monitor/issue_drafts_latest.json | ungrouped | no | yes | 006b7ad6ae39d359384e856101ceac2d5cfa280a3bb895aba5dbfbb626dfdbb3 |
| client/runtime/local/state/ui/infring_dashboard/troubleshooting/eval_issue_resolution_panel.json | ungrouped | no | yes | d89c69d9f6644f5909d309fb0b9fa888f802712c41471534438a739e6fa590fb |
| local/state/ops/eval_quality_gate_v1/history.json | ungrouped | no | yes | a56f667f3235eba5aa4e9e195f262c1cb73bb3b243aab8b1567d7b0a82deac8c |
| artifacts/eval_regression_guard_latest.json | ungrouped | no | yes | d6a598d8d55f4b61326978105746970cc66e17671aa956d9a787b9902ac4c7be |
| artifacts/eval_feedback_router_latest.json | ungrouped | no | yes | 3e04b99369eb7090f7c9f0f7b14e7f6b52d340c32bbd71ec8fc84a40afc68742 |
| local/state/ops/eval_autopilot/latest.json | ungrouped | no | yes | f3dc76d5eabde7639d7496176e44f2f75052971869470474190a38fad226504e |
| local/workspace/reports/EVAL_AUTOPILOT_GUARD_CURRENT.md | ungrouped | no | yes | 117d3dd7ce5ce2a22216ea19e92527513fc6fca05d57b16ea471f26b27b1f232 |
| local/workspace/reports/ISSUE_CANDIDATE_CONTRACT_GUARD_CURRENT.md | ungrouped | no | yes | b63f987287a25dbefe39a3898a4a8dd285722fe6f83b963d8f85a75083cba8ec |
| local/workspace/reports/ISSUE_CANDIDATE_BACKLOG_CURRENT.md | ungrouped | no | yes | ea3c62560e8f1511124008b2375d84ecffda4aff4d7830e4f48190ff0de4f4e5 |
| local/workspace/reports/ORCHESTRATION_QUALITY_CLOSURE_GUARD_CURRENT.md | ungrouped | no | yes | e76cb169845b5eb799897a36bb86325749a9aaff62fb84e3499a22568c9a904d |
| local/workspace/reports/TOOLING_TASK_FABRIC_CLOSURE_GUARD_CURRENT.md | ungrouped | no | yes | 2afae92e323cedc13d095977ffa18305cfc362c8e230b49977f6ae2331cb0150 |
| local/workspace/reports/TOOL_ROUTE_MISDIRECTION_GUARD_CURRENT.md | ungrouped | no | yes | 4f81fbe38484134a2692104e6594489f2645b6cdd86452265b610691e5fc121f |
| local/workspace/reports/CHAT_RENDERING_EXPERIENCE_GUARD_CURRENT.md | ungrouped | no | yes | b75148c7285e7ba0e338f06ec2517e3fb5acf36ace8b4f566f0d1295e97ed3e4 |

## Category summary
- runtime_proof: present=30/30;required=30/30;required_missing=0;required_completeness=1.000;required_min=1.000
- adapter_and_orchestration: present=16/16;required=16/16;required_missing=0;required_completeness=1.000;required_min=1.000
- release_governance: present=35/35;required=35/35;required_missing=0;required_completeness=1.000;required_min=1.000
- ungrouped: present=89/90;required=11/11;required_missing=0;required_completeness=1.000
- workload_and_quality: present=41/41;required=41/41;required_missing=0;required_completeness=1.000;required_min=1.000

## Operator summary
- pass: false
- primary_blocker: core/local/artifacts/eval_regression_guard_current.json
- issue_candidate_ready: true
- next_actions: 36

## Issue candidate
- title: Release proof-pack is not release-ready
- severity: release_blocking
- fingerprint: release_proof_pack:roi-closure-smoke:core/local/artifacts/runtime_proof_verify_current.json|core/local/artifacts/runtime_trusted_core_report_current.json|core/local/artifacts/kernel_sentinel_auto_run_current.json|local/state/kernel_sentinel/kernel_sentinel_report_current.json|local/state/kernel_sentinel/kernel_sentinel_verdict.json|core/local/artifacts/production_readiness_closure_gate_current.json|core/local/artifacts/support_bundle_latest.json|artifacts/web_tooling_context_soak_report_latest.json|client/runtime/local/state/release/scorecard/release_scorecard.json|core/local/artifacts/eval_quality_gate_v1_current.json|core/local/artifacts/eval_regression_guard_current.json|core/local/artifacts/transport_spawn_audit_current.json|core/local/artifacts/release_contract_gate_current.json|core/local/artifacts/issue_candidate_backlog_current.json|core/local/artifacts/gateway_manifest.json|core/local/artifacts/layer2_parity_matrix.json|local/state/kernel_sentinel/kernel_sentinel_verdict.json|local/state/kernel_sentinel/feedback_inbox.jsonl|local/state/kernel_sentinel/trend_history.jsonl|local/state/kernel_sentinel/issues.jsonl|local/state/kernel_sentinel/suggestions.jsonl|local/state/kernel_sentinel/automation_candidates.jsonl|local/state/kernel_sentinel/daily_report.md|optional_artifacts:local/state/kernel_sentinel/kernel_sentinel_report_current.json|optional_artifacts:local/state/kernel_sentinel/kernel_sentinel_verdict.json|optional_artifacts:local/state/kernel_sentinel/feedback_inbox.jsonl|optional_artifacts:local/state/kernel_sentinel/trend_history.jsonl|optional_artifacts:local/state/kernel_sentinel/sentinel_trend_report_current.json|optional_artifacts:local/state/kernel_sentinel/rsi_readiness_summary_current.json|optional_artifacts:local/state/kernel_sentinel/top_system_holes_current.json|optional_artifacts:local/state/kernel_sentinel/issues.jsonl|optional_artifacts:local/state/kernel_sentinel/suggestions.jsonl|optional_artifacts:local/state/kernel_sentinel/automation_candidates.jsonl|optional_artifacts:local/state/kernel_sentinel/daily_report.md
- next_actions: 36

## Top blockers
- release_blocking: mandatory_artifact_failures core/local/artifacts/eval_regression_guard_current.json -> repair mandatory proof artifact core/local/artifacts/eval_regression_guard_current.json
- release_blocking: mandatory_artifact_failures core/local/artifacts/issue_candidate_backlog_current.json -> repair mandatory proof artifact core/local/artifacts/issue_candidate_backlog_current.json
- release_blocking: required_failed_artifacts core/local/artifacts/runtime_proof_verify_current.json -> repair failing required artifact core/local/artifacts/runtime_proof_verify_current.json
- release_blocking: required_failed_artifacts core/local/artifacts/runtime_trusted_core_report_current.json -> repair failing required artifact core/local/artifacts/runtime_trusted_core_report_current.json
- release_blocking: required_failed_artifacts core/local/artifacts/kernel_sentinel_auto_run_current.json -> repair failing required artifact core/local/artifacts/kernel_sentinel_auto_run_current.json
- release_blocking: required_failed_artifacts local/state/kernel_sentinel/kernel_sentinel_report_current.json -> repair failing required artifact local/state/kernel_sentinel/kernel_sentinel_report_current.json
- release_blocking: required_failed_artifacts local/state/kernel_sentinel/kernel_sentinel_verdict.json -> repair failing required artifact local/state/kernel_sentinel/kernel_sentinel_verdict.json
- release_blocking: required_failed_artifacts core/local/artifacts/production_readiness_closure_gate_current.json -> repair failing required artifact core/local/artifacts/production_readiness_closure_gate_current.json
- release_blocking: required_failed_artifacts core/local/artifacts/support_bundle_latest.json -> repair failing required artifact core/local/artifacts/support_bundle_latest.json
- release_blocking: required_failed_artifacts artifacts/web_tooling_context_soak_report_latest.json -> repair failing required artifact artifacts/web_tooling_context_soak_report_latest.json

## Manifest hygiene
- duplicate_warnings: 11
- optional_artifacts: local/state/kernel_sentinel/kernel_sentinel_report_current.json count=1; remove optional duplicate because this path is already required evidence
- optional_artifacts: local/state/kernel_sentinel/kernel_sentinel_verdict.json count=1; remove optional duplicate because this path is already required evidence
- optional_artifacts: local/state/kernel_sentinel/feedback_inbox.jsonl count=1; remove optional duplicate because this path is already required evidence
- optional_artifacts: local/state/kernel_sentinel/trend_history.jsonl count=1; remove optional duplicate because this path is already required evidence
- optional_artifacts: local/state/kernel_sentinel/sentinel_trend_report_current.json count=1; remove optional duplicate because this path is already required evidence
- optional_artifacts: local/state/kernel_sentinel/rsi_readiness_summary_current.json count=1; remove optional duplicate because this path is already required evidence
- optional_artifacts: local/state/kernel_sentinel/top_system_holes_current.json count=1; remove optional duplicate because this path is already required evidence
- optional_artifacts: local/state/kernel_sentinel/issues.jsonl count=1; remove optional duplicate because this path is already required evidence
- optional_artifacts: local/state/kernel_sentinel/suggestions.jsonl count=1; remove optional duplicate because this path is already required evidence
- optional_artifacts: local/state/kernel_sentinel/automation_candidates.jsonl count=1; remove optional duplicate because this path is already required evidence
- optional_artifacts: local/state/kernel_sentinel/daily_report.md count=1; remove optional duplicate because this path is already required evidence
