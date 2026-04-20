fn dashboard_response_gate_checks(response_gate: &Value) -> Value {
    let response_gate_bool = |pointer: &str, default: bool| -> bool {
        response_gate
            .pointer(pointer)
            .and_then(Value::as_bool)
            .unwrap_or(default)
    };
    json!({
        "tooling_response_gate_ready": response_gate_bool("/ready", false),
        "tooling_response_gate_not_blocked": response_gate
            .pointer("/severity")
            .and_then(Value::as_str)
            .is_some_and(|row| row != "blocked"),
        "tooling_response_gate_score_consistent": response_gate_bool("/score_consistent", true),
        "tooling_response_gate_score_band_consistent": response_gate_bool("/score_band_consistent", true),
        "tooling_response_gate_score_band_known": response_gate_bool("/score_band_known", true),
        "tooling_response_gate_score_vector_consistent": response_gate_bool("/score_vector_consistent", true),
        "tooling_response_gate_score_band_vector_consistent": response_gate_bool("/score_band_vector_consistent", true),
        "tooling_response_gate_score_band_severity_consistent": response_gate_bool("/score_band_severity_consistent", true),
        "tooling_response_gate_score_band_severity_bucket_consistent": response_gate_bool("/score_band_severity_bucket_consistent", true),
        "tooling_response_gate_score_band_severity_bucket_known": response_gate_bool("/score_band_severity_bucket_known", true),
        "tooling_response_gate_escalation_routable": response_gate
            .pointer("/escalation_lane")
            .and_then(Value::as_str)
            .is_some_and(|row| row != "none"),
        "tooling_response_gate_escalation_lane_known": response_gate_bool("/escalation_lane_known", true),
        "tooling_response_gate_escalation_reason_known": response_gate_bool("/escalation_reason_known", true),
        "tooling_response_gate_escalation_vector_known": response_gate_bool("/escalation_vector_known", true),
        "tooling_response_gate_escalation_signature_consistent": response_gate_bool("/escalation_signature_consistent", true),
        "tooling_response_gate_next_action_command_consistent": response_gate_bool("/next_action_command_consistent", true),
        "tooling_response_gate_next_action_command_known": response_gate_bool("/next_action_command_known", true),
        "tooling_response_gate_next_action_lane_consistent": response_gate_bool("/next_action_lane_consistent", true),
        "tooling_response_gate_decision_vector_known": response_gate_bool("/decision_vector_known", true),
        "tooling_response_gate_decision_signature_consistent": response_gate_bool("/decision_signature_consistent", true),
        "tooling_response_gate_blocker_budget_consistent": response_gate_bool("/blocker_budget_consistent", true),
        "tooling_response_gate_manual_review_signature_consistent": response_gate_bool("/manual_review_signature_consistent", true),
        "tooling_response_gate_manual_review_reason_consistent": response_gate_bool("/manual_review_reason_consistent", true),
        "tooling_response_gate_manual_review_reason_known": response_gate_bool("/manual_review_reason_known", true),
        "tooling_response_gate_manual_review_vector_consistent": response_gate_bool("/manual_review_vector_consistent", true),
        "tooling_response_gate_manual_review_vector_known": response_gate_bool("/manual_review_vector_known", true),
        "tooling_response_gate_primary_blocker_known": response_gate_bool("/primary_blocker_known", true),
        "tooling_response_gate_blockers_consistent": response_gate_bool("/blockers_consistent", true),
        "tooling_response_gate_severity_consistent": response_gate_bool("/severity_consistent", true),
        "tooling_response_gate_manual_review_consistent": response_gate_bool("/manual_review_consistent", true),
        "tooling_response_gate_blocker_priority_consistent": response_gate_bool("/blocker_priority_consistent", true),
        "tooling_response_gate_blocker_set_consistent": response_gate_bool("/blocker_set_consistent", true),
        "tooling_response_gate_blocker_set_key_consistent": response_gate_bool("/blocker_set_key_consistent", true),
        "tooling_response_gate_blocker_count_key_consistent": response_gate_bool("/blocker_count_key_consistent", true),
        "tooling_response_gate_expected_blocker_count_matches": response_gate_bool("/expected_blocker_count_matches", true),
        "tooling_response_gate_blocker_vector_consistent": response_gate_bool("/blocker_vector_consistent", true),
        "tooling_response_gate_signature_consistent": response_gate_bool("/signature_consistent", true),
        "tooling_response_gate_blocker_flags_consistent": response_gate_bool("/blocker_flags_consistent", true),
        "tooling_response_gate_retry_class_consistent": response_gate_bool("/retry_class_consistent", true),
        "tooling_response_gate_retry_class_known": response_gate_bool("/retry_class_known", true),
        "tooling_response_gate_retry_command_consistent": response_gate_bool("/retry_command_consistent", true),
        "tooling_response_gate_retry_window_consistent": response_gate_bool("/retry_window_consistent", true),
        "tooling_response_gate_retry_signature_consistent": response_gate_bool("/retry_signature_consistent", true),
        "tooling_response_gate_retry_signature_known": response_gate_bool("/retry_signature_known", true),
        "tooling_response_gate_lane_retry_window_consistent": response_gate_bool("/lane_retry_window_consistent", true),
        "tooling_response_gate_retry_band_consistent": response_gate_bool("/retry_band_consistent", true),
        "tooling_response_gate_retry_contract_after_seconds_class_consistent": response_gate_bool("/retry_contract_after_seconds_class_consistent", true),
        "tooling_response_gate_retry_contract_after_seconds_score_band_consistent": response_gate_bool("/retry_contract_after_seconds_score_band_consistent", true),
        "tooling_response_gate_retry_contract_after_seconds_lane_consistent": response_gate_bool("/retry_contract_after_seconds_lane_consistent", true),
        "tooling_response_gate_retry_contract_after_seconds_next_action_window_consistent": response_gate_bool("/retry_contract_after_seconds_next_action_window_consistent", true),
        "tooling_response_gate_retry_contract_lane_command_consistent": response_gate_bool("/retry_contract_lane_command_consistent", true),
        "tooling_response_gate_retry_contract_after_seconds_lane_command_consistent": response_gate_bool("/retry_contract_after_seconds_lane_command_consistent", true),
        "tooling_response_gate_retry_contract_after_seconds_command_consistent": response_gate_bool("/retry_contract_after_seconds_command_consistent", true),
        "tooling_response_gate_retry_mode_consistent": response_gate_bool("/retry_mode_consistent", true),
        "tooling_response_gate_retry_mode_known": response_gate_bool("/retry_mode_known", true),
        "tooling_response_gate_retry_contract_lane_mode_consistent": response_gate_bool("/retry_contract_lane_mode_consistent", true),
        "tooling_response_gate_retry_contract_after_seconds_mode_consistent": response_gate_bool("/retry_contract_after_seconds_mode_consistent", true),
        "tooling_response_gate_retry_contract_after_seconds_lane_mode_consistent": response_gate_bool("/retry_contract_after_seconds_lane_mode_consistent", true),
        "tooling_response_gate_retry_contract_lane_command_mode_consistent": response_gate_bool("/retry_contract_lane_command_mode_consistent", true),
        "tooling_response_gate_retry_contract_after_seconds_lane_command_mode_consistent": response_gate_bool("/retry_contract_after_seconds_lane_command_mode_consistent", true),
        "tooling_response_gate_retry_contract_after_seconds_command_mode_consistent": response_gate_bool("/retry_contract_after_seconds_command_mode_consistent", true),
        "tooling_response_gate_retry_action_vector_consistent": response_gate_bool("/retry_action_vector_consistent", true),
        "tooling_response_gate_retry_budget_consistent": response_gate_bool("/retry_budget_consistent", true),
        "tooling_response_gate_retry_budget_non_negative": response_gate_bool("/retry_budget_non_negative", true),
        "tooling_response_gate_retry_budget_band_consistent": response_gate_bool("/retry_budget_band_consistent", true),
        "tooling_response_gate_retry_budget_expected_band_consistent": response_gate_bool("/retry_budget_expected_band_consistent", true),
        "tooling_response_gate_retry_budget_range_consistent": response_gate_bool("/retry_budget_range_consistent", true),
        "tooling_response_gate_retry_budget_mode_consistent": response_gate_bool("/retry_budget_mode_consistent", true),
        "tooling_response_gate_retry_contract_after_seconds_budget_consistent": response_gate_bool("/retry_contract_after_seconds_budget_consistent", true),
        "tooling_response_gate_retry_pressure_tier_consistent": response_gate_bool("/retry_pressure_tier_consistent", true),
        "tooling_response_gate_retry_pressure_tier_known": response_gate_bool("/retry_pressure_tier_known", true),
        "tooling_response_gate_retry_contract_after_seconds_pressure_consistent": response_gate_bool("/retry_contract_after_seconds_pressure_consistent", true),
        "tooling_response_gate_retry_budget_vector_consistent": response_gate_bool("/retry_budget_vector_consistent", true),
        "tooling_response_gate_retry_budget_vector_known": response_gate_bool("/retry_budget_vector_known", true),
        "tooling_response_gate_retry_tier_window_consistent": response_gate_bool("/retry_tier_window_consistent", true),
        "tooling_response_gate_retry_tier_mode_consistent": response_gate_bool("/retry_tier_mode_consistent", true),
        "tooling_response_gate_retry_tier_vector_consistent": response_gate_bool("/retry_tier_vector_consistent", true),
        "tooling_response_gate_retry_tier_vector_known": response_gate_bool("/retry_tier_vector_known", true),
        "tooling_response_gate_retry_contract_vector_consistent": response_gate_bool("/retry_contract_vector_consistent", true),
        "tooling_response_gate_retry_contract_vector_known": response_gate_bool("/retry_contract_vector_known", true),
        "tooling_response_gate_retry_contract_family_consistent": response_gate_bool("/retry_contract_family_consistent", true),
        "tooling_response_gate_retry_contract_severity_consistent": response_gate_bool("/retry_contract_severity_consistent", true),
        "tooling_response_gate_retry_contract_coherence_consistent": response_gate_bool("/retry_contract_coherence_consistent", true),
        "tooling_response_gate_retry_contract_lane_class_consistent": response_gate_bool("/retry_contract_lane_class_consistent", true),
        "tooling_response_gate_retry_contract_command_class_consistent": response_gate_bool("/retry_contract_command_class_consistent", true),
        "tooling_response_gate_retry_contract_expected_class_consistent": response_gate_bool("/retry_contract_expected_class_consistent", true),
        "tooling_response_gate_retry_contract_pressure_class_consistent": response_gate_bool("/retry_contract_pressure_class_consistent", true),
        "tooling_response_gate_retry_contract_expected_pressure_class_consistent": response_gate_bool("/retry_contract_expected_pressure_class_consistent", true),
        "tooling_response_gate_retry_contract_expected_command_class_consistent": response_gate_bool("/retry_contract_expected_command_class_consistent", true),
        "tooling_response_gate_retry_contract_expected_mode_class_consistent": response_gate_bool("/retry_contract_expected_mode_class_consistent", true),
        "tooling_response_gate_retry_contract_expected_after_seconds_class_consistent": response_gate_bool("/retry_contract_expected_after_seconds_class_consistent", true),
        "tooling_response_gate_retry_contract_expected_after_seconds_band_class_consistent": response_gate_bool("/retry_contract_expected_after_seconds_band_class_consistent", true),
        "tooling_response_gate_retry_contract_expected_after_seconds_pressure_consistent": response_gate_bool("/retry_contract_expected_after_seconds_pressure_consistent", true),
        "tooling_response_gate_retry_contract_expected_mode_pressure_consistent": response_gate_bool("/retry_contract_expected_mode_pressure_consistent", true),
        "tooling_response_gate_retry_contract_expected_lane_pressure_consistent": response_gate_bool("/retry_contract_expected_lane_pressure_consistent", true),
        "tooling_response_gate_retry_contract_expected_command_pressure_consistent": response_gate_bool("/retry_contract_expected_command_pressure_consistent", true),
        "tooling_response_gate_retry_contract_expected_pressure_class_inverse_consistent": response_gate_bool("/retry_contract_expected_pressure_class_inverse_consistent", true),
        "tooling_response_gate_retry_contract_expected_command_mode_consistent": response_gate_bool("/retry_contract_expected_command_mode_consistent", true),
        "tooling_response_gate_retry_contract_expected_after_seconds_mode_consistent": response_gate_bool("/retry_contract_expected_after_seconds_mode_consistent", true),
        "tooling_response_gate_retry_contract_expected_lane_after_seconds_consistent": response_gate_bool("/retry_contract_expected_lane_after_seconds_consistent", true),
        "tooling_response_gate_retry_contract_expected_command_after_seconds_consistent": response_gate_bool("/retry_contract_expected_command_after_seconds_consistent", true),
        "tooling_response_gate_retry_contract_expected_lane_command_after_seconds_consistent": response_gate_bool("/retry_contract_expected_lane_command_after_seconds_consistent", true),
        "tooling_response_gate_retry_contract_expected_lane_command_pressure_consistent": response_gate_bool("/retry_contract_expected_lane_command_pressure_consistent", true),
        "tooling_response_gate_retry_contract_expected_lane_mode_pressure_consistent": response_gate_bool("/retry_contract_expected_lane_mode_pressure_consistent", true),
        "tooling_response_gate_retry_contract_expected_lane_command_mode_consistent": response_gate_bool("/retry_contract_expected_lane_command_mode_consistent", true),
        "tooling_response_gate_retry_contract_expected_lane_command_mode_after_seconds_consistent": response_gate_bool("/retry_contract_expected_lane_command_mode_after_seconds_consistent", true),
        "tooling_response_gate_retry_contract_expected_lane_command_consistent": response_gate_bool("/retry_contract_expected_lane_command_consistent", true),
        "tooling_response_gate_retry_contract_expected_lane_command_class_consistent": response_gate_bool("/retry_contract_expected_lane_command_class_consistent", true),
        "tooling_response_gate_retry_contract_expected_lane_mode_class_consistent": response_gate_bool("/retry_contract_expected_lane_mode_class_consistent", true),
        "tooling_response_gate_retry_contract_expected_lane_class_consistent": response_gate_bool("/retry_contract_expected_lane_class_consistent", true),
        "tooling_response_gate_retry_after_seconds_consistent": response_gate_bool("/retry_after_seconds_consistent", true),
        "tooling_response_gate_retry_after_seconds_non_negative": response_gate_bool("/retry_after_seconds_non_negative", true),
        "tooling_response_gate_contract_consistent": response_gate_bool("/contract_consistent", true)
    })
}

fn dashboard_response_gate_allowed_score_bands_for_severity(
    severity: &str,
) -> &'static [&'static str] {
    match severity {
        "ready" => &["ready"],
        "degraded" => &["strong", "watch"],
        "blocked" => &["weak", "critical"],
        _ => &[],
    }
}

fn dashboard_response_gate_score_band_severity_bucket_known(
    severity: &str,
    score_band: &str,
) -> bool {
    dashboard_response_gate_allowed_score_bands_for_severity(severity)
        .iter()
        .any(|allowed| *allowed == score_band)
}

fn dashboard_response_gate_score_band_severity_bucket_consistent(
    severity: &str,
    score_band: &str,
) -> bool {
    dashboard_response_gate_score_band_severity_bucket_known(severity, score_band)
}

#[cfg(test)]
mod dashboard_response_gate_score_band_bucket_matrix_tests {
    use super::{
        dashboard_response_gate_score_band_severity_bucket_consistent,
        dashboard_response_gate_score_band_severity_bucket_known,
    };

    #[test]
    fn response_gate_score_band_bucket_matrix_covers_ready_degraded_blocked() {
        let valid = [
            ("ready", "ready"),
            ("degraded", "strong"),
            ("degraded", "watch"),
            ("blocked", "weak"),
            ("blocked", "critical"),
        ];
        for (severity, band) in valid {
            assert!(dashboard_response_gate_score_band_severity_bucket_known(
                severity, band
            ));
            assert!(dashboard_response_gate_score_band_severity_bucket_consistent(
                severity, band
            ));
        }

        let invalid = [
            ("ready", "strong"),
            ("degraded", "weak"),
            ("blocked", "watch"),
            ("unknown", "critical"),
        ];
        for (severity, band) in invalid {
            assert!(!dashboard_response_gate_score_band_severity_bucket_known(
                severity, band
            ));
            assert!(!dashboard_response_gate_score_band_severity_bucket_consistent(
                severity, band
            ));
        }
    }
}
