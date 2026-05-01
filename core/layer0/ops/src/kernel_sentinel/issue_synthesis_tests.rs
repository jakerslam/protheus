    use super::*;
    use crate::kernel_sentinel::{
        KernelSentinelFindingCategory, KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
    };
    use std::fs;

    fn repeated_finding() -> KernelSentinelFinding {
        KernelSentinelFinding {
            schema_version: KERNEL_SENTINEL_FINDING_SCHEMA_VERSION,
            id: "finding-1".to_string(),
            severity: KernelSentinelSeverity::High,
            category: KernelSentinelFindingCategory::GatewayIsolation,
            fingerprint: "gateway_isolation:gateway_missing_quarantine:ollama".to_string(),
            evidence: vec!["gateway://ollama/flap".to_string()],
            summary: "ollama gateway flapped without quarantine".to_string(),
            recommended_action: "quarantine the gateway".to_string(),
            status: "open".to_string(),
        }
    }

    #[test]
    fn repeated_fingerprint_produces_one_issue_draft() {
        let finding = repeated_finding();
        let report = build_issue_synthesis(&[finding.clone(), finding], &[]);
        assert_eq!(report["active_issue_window_count"], Value::from(1));
        assert_eq!(report["issue_drafts"][0]["occurrence_count"], Value::from(2));
        assert_eq!(
            report["issue_drafts"][0]["failure_level"],
            "L2_boundary_contract_breach"
        );
        assert_eq!(
            report["issue_drafts"][0]["root_frame"],
            "cross_boundary_contract"
        );
        assert_eq!(
            report["issue_drafts"][0]["remediation_level"],
            "boundary_repair"
        );
        assert_eq!(report["issue_drafts"][0]["component"], "ollama");
        assert_eq!(
            report["issue_drafts"][0]["observed_failure"],
            "ollama gateway flapped without quarantine"
        );
        assert!(report["issue_drafts"][0]["root_cause_hypothesis"]
            .as_str()
            .unwrap()
            .contains("cross_boundary_contract"));
        assert_eq!(report["issue_drafts"][0]["repair_type"], "boundary_repair");
        assert_eq!(report["issue_drafts"][0]["todo_actionability"]["todo_ready"], true);
        assert_eq!(
            report["issue_drafts"][0]["validation_route"][0]["route"],
            "kernel_sentinel_regression"
        );
    }

    #[test]
    fn singleton_fingerprint_is_rate_limited_by_default() {
        let report = build_issue_synthesis(&[repeated_finding()], &[]);
        assert_eq!(report["active_issue_window_count"], Value::from(0));
        assert_eq!(report["rate_limited_cluster_count"], Value::from(1));
    }

    #[test]
    fn raw_source_fingerprints_are_not_issue_drafts_by_default() {
        let mut finding = repeated_finding();
        finding.fingerprint = "gateway_health:ollama:gateway_health".to_string();
        let report = build_issue_synthesis(&[finding.clone(), finding], &[]);
        assert_eq!(report["cluster_count"], Value::from(0));
        assert_eq!(report["issue_draft_count"], Value::from(0));
    }

    #[test]
    fn synthetic_round_failures_collapse_into_one_issue_family() {
        let mut round_1 = repeated_finding();
        round_1.category = KernelSentinelFindingCategory::RuntimeCorrectness;
        round_1.fingerprint = "runtime_correctness:misty_simulated_round01_failures".to_string();
        round_1.evidence = vec![
            "runtime://misty;session=synthetic-user-chat;surface=chat;receipt_type=tool_route"
                .to_string(),
        ];
        round_1.summary = "misty synthetic chat harness failed during simulated round 01".to_string();
        round_1.recommended_action =
            "collapse repeated synthetic round failures into one issue family".to_string();

        let mut round_2 = round_1.clone();
        round_2.id = "finding-2".to_string();
        round_2.fingerprint = "runtime_correctness:misty_simulated_round02_failures".to_string();
        round_2.summary = "misty synthetic chat harness failed during simulated round 02".to_string();

        let report = build_issue_synthesis(&[round_1, round_2], &[]);
        assert_eq!(report["cluster_count"], Value::from(1));
        assert_eq!(report["active_issue_window_count"], Value::from(1));
        assert_eq!(report["issue_draft_count"], Value::from(1));
        assert_eq!(
            report["issue_drafts"][0]["fingerprint"],
            "synthetic_user_chat_harness:misty_simulated_failures"
        );
        assert_eq!(report["issue_drafts"][0]["occurrence_count"], Value::from(2));
        assert_eq!(
            report["issue_drafts"][0]["exemplar_fingerprint"],
            "runtime_correctness:misty_simulated_round01_failures"
        );
    }

    #[test]
    fn synthetic_round_failures_collapse_across_sessions_into_scenario_issue_candidate() {
        let mut round_1 = repeated_finding();
        round_1.category = KernelSentinelFindingCategory::RuntimeCorrectness;
        round_1.fingerprint = "runtime_correctness:misty_simulated_round01_failures".to_string();
        round_1.evidence = vec![
            "runtime://misty;session=synthetic-user-chat-a;surface=chat;receipt_type=tool_route"
                .to_string(),
        ];
        round_1.summary = "misty synthetic chat harness failed during simulated round 01".to_string();
        round_1.recommended_action =
            "collapse repeated synthetic round failures into one scenario issue".to_string();

        let mut round_2 = round_1.clone();
        round_2.id = "finding-2".to_string();
        round_2.fingerprint = "runtime_correctness:misty_simulated_round02_failures".to_string();
        round_2.evidence = vec![
            "runtime://misty;session=synthetic-user-chat-b;surface=chat;receipt_type=final_response"
                .to_string(),
        ];

        let report = build_issue_synthesis(&[round_1, round_2], &[]);
        let draft = &report["issue_drafts"][0];

        assert_eq!(report["cluster_count"], Value::from(1));
        assert_eq!(draft["scenario_level"], true);
        assert_eq!(draft["issue_family_kind"], "synthetic_scenario");
        assert_eq!(draft["scenario_id"], "misty_simulated_failures");
        assert_eq!(
            draft["cluster_key"],
            "scenario=misty_simulated_failures|fingerprint=synthetic_user_chat_harness:misty_simulated_failures"
        );
        assert_eq!(draft["occurrence_count"], Value::from(2));
    }

    #[test]
    fn cluster_key_collapses_sessions_when_root_frame_and_invariant_match() {
        let mut first = repeated_finding();
        first.evidence = vec!["gateway://ollama/flap;session=a;surface=gateway;receipt_type=quarantine".to_string()];
        let mut second = first.clone();
        second.evidence = vec!["gateway://ollama/flap;session=b;surface=gateway;receipt_type=quarantine".to_string()];
        let report = build_issue_synthesis(&[first, second], &[]);
        assert_eq!(report["cluster_count"], Value::from(1));
        assert_eq!(report["active_issue_window_count"], Value::from(1));
        assert_eq!(report["rate_limited_cluster_count"], Value::from(0));
        assert_eq!(
            report["issue_drafts"][0]["cluster_key"],
            "root_frame=cross_boundary_contract|violated_invariants=unknown_invariant"
        );
    }

    #[test]
    fn repeated_visible_symptom_patches_are_flagged_as_structural_loop() {
        let mut first = repeated_finding();
        first.fingerprint = "gateway_isolation:taskbar_offline_symptom".to_string();
        first.summary = "visible symptom patch left the taskbar connectivity indicator offline".to_string();
        first.recommended_action =
            "stop_patching symptoms and repair the gateway boundary".to_string();

        let mut second = first.clone();
        second.id = "finding-2".to_string();
        second.fingerprint = "gateway_isolation:duplicate_dashboard_restart_symptom".to_string();
        second.summary = "local patch hid one startup symptom but duplicate gateway restart remained"
            .to_string();

        let report = build_issue_synthesis(&[first, second], &[]);
        let draft = &report["issue_drafts"][0];

        assert_eq!(report["active_issue_window_count"], Value::from(1));
        assert_eq!(report["anti_patching_loop_count"], Value::from(1));
        assert_eq!(
            draft["anti_patching"]["symptom_patching_loop_detected"],
            true
        );
        assert_eq!(draft["anti_patching"]["structural_root_required"], true);
        assert_eq!(
            draft["anti_patching"]["distinct_symptom_fingerprint_count"],
            Value::from(2)
        );
        assert_eq!(
            draft["anti_patching"]["next_action"],
            "stop_local_symptom_patching_and_repair_structural_root"
        );
    }

    #[test]
    fn issue_quality_guard_rejects_advisory_only_vague_drafts() {
        let failures = issue_quality_failures(&[json!({
            "title": "issue",
            "fingerprint": "semantic_monitor:maybe",
            "evidence": ["semantic://summary-only"],
            "impact": "",
            "recommended_fix": "",
            "acceptance_criteria": ["look again"],
            "component": "",
            "observed_failure": "",
            "root_cause_hypothesis": "",
            "repair_type": "",
            "validation_route": []
        })]);
        assert_eq!(failures.len(), 1);
        assert!(failures[0]["reasons"].as_array().unwrap().contains(&Value::from("missing_deterministic_evidence")));
        assert!(failures[0]["reasons"].as_array().unwrap().contains(&Value::from("missing_component")));
        assert!(failures[0]["reasons"].as_array().unwrap().contains(&Value::from("missing_validation_route")));
        assert_eq!(failures[0]["blocks_release"], true);
    }

    #[test]
    fn write_issue_drafts_jsonl_attaches_matching_diagnostic_context() {
        let finding = repeated_finding();
        let issue_synthesis = build_issue_synthesis(&[finding.clone(), finding], &[]);
        let report = json!({
            "issue_synthesis": issue_synthesis
        });
        let out = std::env::temp_dir().join(format!(
            "kernel-sentinel-issue-jsonl-{}.jsonl",
            crate::deterministic_receipt_hash(&json!({
                "test": "issue-diagnostic-context"
            }))
        ));
        write_issue_drafts_jsonl(
            &out,
            &report,
            Some(&json!({
                "type": "kernel_sentinel_diagnostic_run",
                "probe_requests": [
                    {
                        "incident_cluster_key": "root_frame=cross_boundary_contract|violated_invariants=unknown_invariant",
                        "root_frame": "cross_boundary_contract",
                        "violated_invariants": ["unknown_invariant"],
                        "failure_signature": "gateway_missing_quarantine",
                        "selected_probe": "topology://ollama/gateway",
                        "expected_confidence_gain": 0.42,
                        "authorization_state": "authorized"
                    }
                ],
                "authorized_probe_count": 1,
                "refused_probe_count": 0,
                "total_expected_confidence_gain": 0.42,
                "recurring_inconclusive_patterns": []
            })),
        )
        .expect("issue synthesis should write jsonl");
        let row = fs::read_to_string(&out)
            .expect("jsonl should exist")
            .lines()
            .next()
            .map(|line| serde_json::from_str::<Value>(line).expect("row should be valid json"))
            .expect("row should be present");
        assert_eq!(
            row["diagnostic_run_artifact_path"],
            "local/state/kernel_sentinel/kernel_sentinel_diagnostic_run_current.json"
        );
        assert_eq!(
            row["diagnostic_evidence"].as_array().unwrap().len(),
            Value::from(1)
        );
        assert_eq!(
            row["diagnostic_evidence"][0]["selected_probe"],
            "topology://ollama/gateway"
        );
        let _ = fs::remove_file(out);
    }

    #[test]
    fn issue_draft_has_github_ready_title_and_summary() {
        let finding = repeated_finding();
        let report = build_issue_synthesis(&[finding.clone(), finding], &[]);
        let draft = &report["issue_drafts"][0];
        let title = draft["title"].as_str().unwrap();
        let summary = draft["summary"].as_str().unwrap();

        assert!(title.starts_with("[Kernel Sentinel][High/GatewayIsolation]"));
        assert!(title.contains("ollama gateway flapped without quarantine"));
        assert!(!title.contains('\n'));
        assert!(title.len() <= 150);
        assert!(summary.contains("2 occurrence"));
        assert!(summary.contains("gateway_isolation:gateway_missing_quarantine:ollama"));
        assert!(summary.contains("quarantine"));
        assert!(summary.contains("Exemplar: ollama gateway flapped without quarantine"));
    }

    #[test]
    fn issue_draft_carries_todo_specific_repair_contract() {
        let finding = repeated_finding();
        let report = build_issue_synthesis(&[finding.clone(), finding], &[]);
        let draft = &report["issue_drafts"][0];

        for key in [
            "component",
            "observed_failure",
            "root_cause_hypothesis",
            "repair_type",
            "validation_route",
            "evidence",
        ] {
            assert!(draft.get(key).is_some(), "missing {key}");
        }
        assert_eq!(draft["todo_actionability"]["component_present"], true);
        assert_eq!(draft["todo_actionability"]["validation_route_present"], true);
        assert_eq!(draft["todo_actionability"]["safe_to_mutate_todo"], false);
        assert!(draft["acceptance_criteria"]
            .as_array()
            .unwrap()
            .iter()
            .any(|row| row.as_str().unwrap().contains("validation route passes")));
    }
