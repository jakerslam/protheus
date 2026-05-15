// Layer ownership: eval/observability (ForgeCode-derived software complexity ladder measurement).
use crate::eval_coding_safety_layer::coding_safety_layer_lab_report;
use crate::eval_local_coding_program_builder::local_coding_program_builder_lab_file_execution_report;
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct ForgeSoftwareComplexityLadderReport {
    pub harness_kind: &'static str,
    pub workflow_under_test: &'static str,
    pub proof_workflow: &'static str,
    pub ok: bool,
    pub current_breakpoint: ForgeSoftwareComplexityBreakpoint,
    pub proven_maximum: ForgeSoftwareComplexityProvenMaximum,
    pub levels: Vec<ForgeSoftwareComplexityLevel>,
    pub recommended_next_live_eval: ForgeSoftwareComplexityLiveEvalRecommendation,
    pub failures: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForgeSoftwareComplexityBreakpoint {
    pub first_unproven_level: &'static str,
    pub reason: &'static str,
    pub required_next_evidence: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForgeSoftwareComplexityProvenMaximum {
    pub level_id: &'static str,
    pub summary: &'static str,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForgeSoftwareComplexityLevel {
    pub level: u64,
    pub level_id: &'static str,
    pub task_shape: &'static str,
    pub expected_file_count_range: &'static str,
    pub expected_capabilities: Vec<&'static str>,
    pub success_criteria: ForgeSoftwareComplexitySuccessCriteria,
    pub status: &'static str,
    pub lab_evidence: Vec<String>,
    pub required_live_evidence: Vec<&'static str>,
    pub promotion_threshold: &'static str,
    pub failure_modes_to_watch: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForgeSoftwareComplexitySuccessCriteria {
    pub required_runtime_evidence: Vec<&'static str>,
    pub completion_checks: Vec<&'static str>,
    pub anti_false_positive_checks: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ForgeSoftwareComplexityLiveEvalRecommendation {
    pub level_id: &'static str,
    pub prompt_shape: &'static str,
    pub run_count: u64,
    pub pass_threshold: u64,
    pub stop_conditions: Vec<&'static str>,
}

pub fn forge_software_complexity_ladder_lab_report() -> ForgeSoftwareComplexityLadderReport {
    let coding_report = local_coding_program_builder_lab_file_execution_report();
    let safety_report = coding_safety_layer_lab_report();
    let mut failures = Vec::new();

    if !coding_report.ok {
        failures.push("local_coding_program_builder_lab_report_failed".to_string());
        failures.extend(coding_report.failures.clone());
    }
    if !safety_report.ok {
        failures.push("coding_safety_layer_lab_report_failed".to_string());
        failures.extend(safety_report.failures.clone());
    }

    let task_ids = coding_report
        .task_executions
        .iter()
        .map(|execution| execution.task_id)
        .collect::<Vec<_>>();
    let single_file = coding_report
        .task_executions
        .iter()
        .find(|execution| execution.task_id == "single_file_utility");
    let existing_project = coding_report
        .task_executions
        .iter()
        .find(|execution| execution.task_id == "initialized_project_modification");
    let small_multi_file = coding_report
        .task_executions
        .iter()
        .find(|execution| execution.task_id == "small_multi_file_app");

    if single_file.is_none() {
        failures.push("missing_complexity_level_evidence:single_file_utility".to_string());
    }
    if existing_project.is_none() {
        failures
            .push("missing_complexity_level_evidence:initialized_project_modification".to_string());
    }
    if small_multi_file.is_none() {
        failures.push("missing_complexity_level_evidence:small_multi_file_app".to_string());
    }

    let levels = vec![
        ForgeSoftwareComplexityLevel {
            level: 1,
            level_id: "single_file_utility",
            task_shape: "Create one focused code file plus a small behavior test or usage artifact.",
            expected_file_count_range: "1-3 files",
            expected_capabilities: vec![
                "single_file_code_write",
                "changed_file_summary",
                "safe_file_write_receipts",
            ],
            success_criteria: ForgeSoftwareComplexitySuccessCriteria {
                required_runtime_evidence: vec![
                    "native_agent_receipt_ok",
                    "native_tool_call_count_greater_than_zero",
                    "successful_file_write_or_patch_receipt",
                ],
                completion_checks: vec![
                    "expected_single_code_file_exists",
                    "behavior_test_or_usage_artifact_exists",
                ],
                anti_false_positive_checks: vec![
                    "original_fixture_state_not_counted_as_success",
                    "empty_model_output_is_failure",
                ],
            },
            status: if single_file.is_some() && !safety_report.write_receipts.is_empty() {
                "lab_pass"
            } else {
                "missing_lab_evidence"
            },
            lab_evidence: single_file
                .map(|execution| execution.changed_files.clone())
                .unwrap_or_default(),
            required_live_evidence: vec![
                "agent_trace_ref",
                "file_change_receipt_ref",
                "19_of_20_live_successes",
            ],
            promotion_threshold: "19/20 live attempts",
            failure_modes_to_watch: vec![
                "fails_to_create_file",
                "omits_changed_file_summary",
                "touches_unrelated_files",
            ],
        },
        ForgeSoftwareComplexityLevel {
            level: 2,
            level_id: "existing_project_feature_slice",
            task_shape: "Modify an initialized project while preserving existing architecture.",
            expected_file_count_range: "2-5 files",
            expected_capabilities: vec![
                "repo_context_assessment",
                "safe_file_patch",
                "architecture_preservation",
                "unrelated_file_preservation",
            ],
            success_criteria: ForgeSoftwareComplexitySuccessCriteria {
                required_runtime_evidence: vec![
                    "native_agent_receipt_ok",
                    "successful_file_patch_or_overwrite_receipt",
                    "read_receipt_for_existing_project_context",
                ],
                completion_checks: vec![
                    "requested_feature_behavior_present",
                    "existing_behavior_still_passes",
                    "changed_files_match_expected_scope",
                ],
                anti_false_positive_checks: vec![
                    "unchanged_existing_tests_alone_do_not_pass_level",
                    "unrelated_file_mutation_fails_level",
                ],
            },
            status: if existing_project.is_some() && !safety_report.patch_receipts.is_empty() {
                "lab_pass"
            } else {
                "missing_lab_evidence"
            },
            lab_evidence: existing_project
                .map(|execution| execution.changed_files.clone())
                .unwrap_or_default(),
            required_live_evidence: vec![
                "agent_trace_ref",
                "patch_tool_use_evidence",
                "unrelated_file_preservation_evidence",
                "19_of_20_live_successes",
            ],
            promotion_threshold: "19/20 live attempts",
            failure_modes_to_watch: vec![
                "architecture_drift",
                "patch_mismatch",
                "unrelated_file_mutation",
            ],
        },
        ForgeSoftwareComplexityLevel {
            level: 3,
            level_id: "small_multi_file_domain_app",
            task_shape: "Build a bounded multi-file local app with domain/app/interface separation.",
            expected_file_count_range: "5-10 files",
            expected_capabilities: vec![
                "architecture_and_slice_planning",
                "multi_file_coding_execution",
                "domain_first_structure",
                "checkpoint_handoff",
            ],
            success_criteria: ForgeSoftwareComplexitySuccessCriteria {
                required_runtime_evidence: vec![
                    "native_agent_receipt_ok",
                    "multiple_successful_file_write_or_patch_receipts",
                    "changed_file_manifest",
                ],
                completion_checks: vec![
                    "domain_app_interface_separation_exists",
                    "tests_or_executable_usage_cover_core_behavior",
                    "project_runs_without_missing_imports",
                ],
                anti_false_positive_checks: vec![
                    "single_file_solution_does_not_pass_multi_file_level",
                    "placeholder_tests_do_not_pass_level",
                ],
            },
            status: if small_multi_file.is_some() {
                "lab_pass"
            } else {
                "missing_lab_evidence"
            },
            lab_evidence: small_multi_file
                .map(|execution| execution.changed_files.clone())
                .unwrap_or_default(),
            required_live_evidence: vec![
                "agent_trace_ref",
                "multi_file_ownership_summary",
                "file_change_receipt_ref",
                "8_of_10_initial_live_successes",
                "18_of_20_promotion_live_successes",
            ],
            promotion_threshold: "8/10 initial, then 18/20 promotion",
            failure_modes_to_watch: vec![
                "cross_file_incoherence",
                "missing_tests_or_usage_artifacts",
                "overbroad_scope",
            ],
        },
        ForgeSoftwareComplexityLevel {
            level: 4,
            level_id: "medium_cli_with_persistence",
            task_shape: "Build a medium CLI app with domain model, command parser, persistence adapter, fixtures, and tests.",
            expected_file_count_range: "10-15 files",
            expected_capabilities: vec![
                "stack_and_architecture_selection",
                "multi_slice_execution",
                "persistence_boundary",
                "bounded_repair_and_validation",
            ],
            success_criteria: ForgeSoftwareComplexitySuccessCriteria {
                required_runtime_evidence: vec![
                    "native_agent_receipt_ok",
                    "successful_read_receipts_for_seed_project",
                    "successful_mutation_receipts_for_source_tests_and_docs",
                ],
                completion_checks: vec![
                    "persistence_adapter_exists",
                    "cli_or_service_exercises_persistence_boundary",
                    "behavior_tests_cover_roundtrip_state",
                ],
                anti_false_positive_checks: vec![
                    "seed_tests_passing_without_new_persistence_fails_level",
                    "readme_only_change_fails_level",
                    "unbounded_repair_loop_fails_level",
                ],
            },
            status: "blocked_live_eval_required",
            lab_evidence: vec![
                "small_multi_file_domain_app_is_lab_proven_but_medium_persistence_boundary_is_not"
                    .to_string(),
            ],
            required_live_evidence: vec![
                "agent_trace_ref",
                "controlled_eval_run_boundary_receipt",
                "executable_eval_result_receipt",
                "multi_file_change_receipts",
                "validation_receipts",
                "forge_level4_software_analysis_report",
                "no_unbounded_repair_loop",
            ],
            promotion_threshold: "first breakpoint probe: 5 live attempts, target 4/5",
            failure_modes_to_watch: vec![
                "architecture_decay",
                "persistence_adapter_leaks_into_domain",
                "validation_loop_sprawl",
                "too_many_files_without_checkpoint",
            ],
        },
        ForgeSoftwareComplexityLevel {
            level: 5,
            level_id: "medium_service_or_web_app",
            task_shape: "Build a medium service or web app with API layer, state boundary, UI or transport adapter, and integration tests.",
            expected_file_count_range: "15-25 files",
            expected_capabilities: vec![
                "architecture_partitioning",
                "adapter_boundaries",
                "integration_validation",
                "multi_checkpoint_handoff",
            ],
            success_criteria: ForgeSoftwareComplexitySuccessCriteria {
                required_runtime_evidence: vec![
                    "native_agent_receipt_ok",
                    "successful_mutation_receipts_across_api_state_and_tests",
                    "integration_validation_receipt",
                ],
                completion_checks: vec![
                    "api_or_transport_layer_exists",
                    "state_boundary_is_not_leaked_into_domain",
                    "integration_tests_execute_requested_flow",
                ],
                anti_false_positive_checks: vec![
                    "generated_scaffold_without_working_flow_fails_level",
                    "unit_tests_only_without_integration_path_fail_level",
                    "empty_success_or_no_write_receipts_fail_level",
                ],
            },
            status: "not_proven",
            lab_evidence: vec![],
            required_live_evidence: vec![
                "multi_checkpoint_live_eval_receipts",
                "integration_validation_receipts",
                "architecture_drift_report",
            ],
            promotion_threshold: "not eligible until level 4 passes",
            failure_modes_to_watch: vec![
                "adapter_domain_coupling",
                "test_surface_missing",
                "checkpoint_boundary_blur",
            ],
        },
        ForgeSoftwareComplexityLevel {
            level: 6,
            level_id: "large_multi_checkpoint_project",
            task_shape: "Build a larger project over several checkpoints with evolving architecture and repair loops.",
            expected_file_count_range: "25-60 files",
            expected_capabilities: vec![
                "multi_checkpoint_planning",
                "stateful_handoff",
                "regression_aware_repair",
                "module_boundary_preservation",
            ],
            success_criteria: ForgeSoftwareComplexitySuccessCriteria {
                required_runtime_evidence: vec![
                    "sequential_native_agent_receipts",
                    "checkpoint_manifest_per_slice",
                    "regression_validation_receipts",
                ],
                completion_checks: vec![
                    "each_checkpoint_has_executable_increment",
                    "architecture_boundaries_survive_followup_slice",
                    "handoff_context_is_sufficient_to_resume",
                ],
                anti_false_positive_checks: vec![
                    "one_large_unvalidated_dump_fails_level",
                    "missing_checkpoint_stop_condition_fails_level",
                    "regression_repair_without_evidence_fails_level",
                ],
            },
            status: "not_proven",
            lab_evidence: vec![],
            required_live_evidence: vec![
                "sequential_checkpoint_receipts",
                "regression_validation_receipts",
                "operator_checkpoint_approval_receipts",
            ],
            promotion_threshold: "not eligible until level 5 passes",
            failure_modes_to_watch: vec![
                "context_decay",
                "unbounded_scope_growth",
                "regression_repair_failure",
            ],
        },
        ForgeSoftwareComplexityLevel {
            level: 7,
            level_id: "production_scale_10k_line_project",
            task_shape: "Sustain a production-scale codebase beyond 10k lines without architecture decay.",
            expected_file_count_range: "60+ files",
            expected_capabilities: vec![
                "long_horizon_architecture_governance",
                "module_ownership_tracking",
                "continuous_eval_feedback",
                "promotion_decision_reporting",
            ],
            success_criteria: ForgeSoftwareComplexitySuccessCriteria {
                required_runtime_evidence: vec![
                    "long_running_native_agent_campaign_receipts",
                    "architecture_decay_metric_receipts",
                    "memory_resume_receipts",
                ],
                completion_checks: vec![
                    "module_ownership_map_remains_current",
                    "resume_context_restores_project_state",
                    "promotion_report_cites_pass_fail_evidence",
                ],
                anti_false_positive_checks: vec![
                    "line_count_growth_without_architecture_evidence_fails_level",
                    "manual_codex_intervention_fails_native_level",
                    "missing_memory_or_resume_evidence_fails_level",
                ],
            },
            status: "not_proven",
            lab_evidence: vec![],
            required_live_evidence: vec![
                "long_running_live_agent_eval_campaign",
                "architecture_decay_metrics",
                "promotion_decision_report_receipts",
            ],
            promotion_threshold: "not eligible until levels 4-6 pass repeatedly",
            failure_modes_to_watch: vec![
                "architecture_decay_after_10k_lines",
                "ownership_boundary_loss",
                "validation_runtime_sprawl",
            ],
        },
    ];

    let current_breakpoint = ForgeSoftwareComplexityBreakpoint {
        first_unproven_level: "medium_cli_with_persistence",
        reason: "levels 1-3 have deterministic lab evidence; level 4 requires live agent traces, persistence-boundary behavior, and validation-loop receipts",
        required_next_evidence: vec![
            "5 live attempts for medium_cli_with_persistence",
            "controlled_eval_run_boundary_receipts",
            "normalized_executable_eval_result_receipts",
            "forge_level4_software_analysis_report",
            "validation_receipts",
            "coverage_and_promotion_decision_receipts",
        ],
    };

    let proven_maximum = ForgeSoftwareComplexityProvenMaximum {
        level_id: "small_multi_file_domain_app",
        summary: "bounded multi-file local app with domain/app/interface separation is lab-proven, but not live-agent proven",
        evidence: small_multi_file
            .map(|execution| execution.changed_files.clone())
            .unwrap_or_default(),
    };

    let recommended_next_live_eval = ForgeSoftwareComplexityLiveEvalRecommendation {
        level_id: "medium_cli_with_persistence",
        prompt_shape: "Create a local CLI task ledger with domain model, command parser, JSON persistence adapter, fixtures, and behavior tests across roughly 10-15 files.",
        run_count: 5,
        pass_threshold: 4,
        stop_conditions: vec![
            "unexpected_dirty_files",
            "unrelated_file_mutation",
            "unbounded_repair_loop",
            "architecture_boundary_collapse",
            "validation_without_authorization",
        ],
    };

    let ok = failures.is_empty()
        && levels
            .iter()
            .take(3)
            .all(|level| level.status == "lab_pass")
        && levels
            .iter()
            .skip(3)
            .all(|level| level.status != "lab_pass")
        && task_ids.len() == 3;

    ForgeSoftwareComplexityLadderReport {
        harness_kind: "forge_software_complexity_ladder_lab_v1",
        workflow_under_test: "local_coding_program_builder",
        proof_workflow: "forge_executable_proof_campaign",
        ok,
        current_breakpoint,
        proven_maximum,
        levels,
        recommended_next_live_eval,
        failures,
    }
}

#[cfg(test)]
mod tests {
    use super::forge_software_complexity_ladder_lab_report;

    #[test]
    fn forge_software_complexity_ladder_identifies_current_breakpoint() {
        let report = forge_software_complexity_ladder_lab_report();
        assert!(report.ok, "{report:#?}");
        assert_eq!(
            report.proven_maximum.level_id,
            "small_multi_file_domain_app"
        );
        assert_eq!(
            report.current_breakpoint.first_unproven_level,
            "medium_cli_with_persistence"
        );
        assert_eq!(report.levels.len(), 7);
        assert_eq!(report.levels[0].status, "lab_pass");
        assert_eq!(report.levels[1].status, "lab_pass");
        assert_eq!(report.levels[2].status, "lab_pass");
        assert_eq!(report.levels[3].status, "blocked_live_eval_required");
    }
}
