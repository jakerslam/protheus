#[cfg(test)]
mod tests {
    use super::*;

    fn spawn_options() -> SpawnOptions {
        SpawnOptions {
            verify: true,
            timeout_ms: 100,
            metrics_detailed: true,
            simulate_unreachable: false,
            byzantine: false,
            corruption_type: "data_falsification".to_string(),
            token_budget: None,
            token_warning_threshold: 0.8,
            budget_exhaustion_action: BudgetAction::FailHard,
            adaptive_complexity: false,
            execution_mode: ExecutionMode::TaskOriented,
            role: None,
            capabilities: Vec::new(),
            auto_publish_results: false,
            agent_label: None,
            result_value: None,
            result_text: None,
            result_confidence: 1.0,
            verification_status: "not_verified".to_string(),
        }
    }

    fn calc_result(
        result_id: &str,
        session_id: &str,
        agent_label: &str,
        value: f64,
        timestamp_ms: u64,
    ) -> AgentResult {
        AgentResult {
            result_id: result_id.to_string(),
            session_id: session_id.to_string(),
            agent_label: agent_label.to_string(),
            agent_role: "worker".to_string(),
            task_id: "task-1".to_string(),
            payload: ResultPayload::Calculation { value },
            data: json!({"value": value}),
            confidence: 0.9,
            verification_status: "verified".to_string(),
            timestamp_ms,
            created_at: "2026-01-01T00:00:00Z".to_string(),
        }
    }

    #[test]
    fn recursive_spawn_tracks_parent_and_children() {
        let mut state = SwarmState::default();
        let options = spawn_options();
        let result = recursive_spawn_with_tracking(&mut state, None, "task", 3, 6, &options)
            .expect("recursive spawn should succeed");
        assert_eq!(
            result
                .get("lineage")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(3)
        );

        let lineage = result
            .get("lineage")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let first = lineage
            .first()
            .and_then(|row| row.get("session_id"))
            .and_then(Value::as_str)
            .expect("first session id");
        let second = lineage
            .get(1)
            .and_then(|row| row.get("session_id"))
            .and_then(Value::as_str)
            .expect("second session id");
        let first_session = state.sessions.get(first).expect("first session exists");
        assert_eq!(first_session.children, vec![second.to_string()]);
    }

    #[test]
    fn spawn_verify_fails_when_child_is_unreachable() {
        let mut state = SwarmState::default();
        let mut options = spawn_options();
        options.simulate_unreachable = true;
        let err = spawn_single(&mut state, None, "task", 4, &options).expect_err("must fail");
        assert!(
            err.contains("session_unreachable_timeout"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn consensus_detector_marks_outliers() {
        let reports = vec![
            AgentReport {
                agent_id: "a1".to_string(),
                values: BTreeMap::from([
                    ("file_size".to_string(), json!(1847)),
                    ("word_count".to_string(), json!(292)),
                ]),
            },
            AgentReport {
                agent_id: "a2".to_string(),
                values: BTreeMap::from([
                    ("file_size".to_string(), json!(1847)),
                    ("word_count".to_string(), json!(292)),
                ]),
            },
            AgentReport {
                agent_id: "a3".to_string(),
                values: BTreeMap::from([
                    ("file_size".to_string(), json!(9999)),
                    ("word_count".to_string(), json!(5000)),
                ]),
            },
        ];
        let fields = vec!["file_size".to_string(), "word_count".to_string()];
        let result = evaluate_consensus(&reports, &fields, 0.6);
        assert_eq!(
            result.get("consensus_reached").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            result
                .get("outliers")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
        assert_eq!(
            result.get("reason_code").and_then(Value::as_str),
            Some("majority_with_outliers")
        );
        assert_eq!(
            result.get("recommended_action").and_then(Value::as_str),
            Some("accept_with_outlier_review")
        );
        assert_eq!(
            result.get("confidence_band").and_then(Value::as_str),
            Some("medium")
        );
    }

    #[test]
    fn numeric_outlier_analysis_emits_robust_stats() {
        let results = vec![
            calc_result("r1", "s1", "a1", 10.0, 1),
            calc_result("r2", "s2", "a2", 10.1, 2),
            calc_result("r3", "s3", "a3", 42.0, 3),
        ];
        let outliers = analyze_result_outliers(&results, "value");
        assert_eq!(
            outliers.get("status").and_then(Value::as_str),
            Some("outliers_detected")
        );
        assert!(outliers.get("median").is_some());
        assert!(outliers.get("mad").is_some());
        let first_outlier = outliers
            .get("outliers")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .cloned()
            .unwrap_or(Value::Null);
        assert!(first_outlier.get("robust_z_score").is_some());
    }

    #[test]
    fn byzantine_requires_test_mode() {
        let mut state = SwarmState::default();
        let mut options = spawn_options();
        options.byzantine = true;
        let err = spawn_single(&mut state, None, "task", 5, &options)
            .expect_err("byzantine must fail without test mode");
        assert_eq!(err, "byzantine_test_mode_required");

