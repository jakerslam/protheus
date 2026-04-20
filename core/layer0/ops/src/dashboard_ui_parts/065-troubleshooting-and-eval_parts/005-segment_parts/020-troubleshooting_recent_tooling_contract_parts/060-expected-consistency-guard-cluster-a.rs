    let response_gate_retry_contract_expected_mode_pressure_consistent =
        match response_gate_expected_retry_pressure_tier {
            "low" => {
                response_gate_expected_retry_mode == "none"
                    || response_gate_expected_retry_mode == "passive"
            }
            "medium" | "high" => response_gate_expected_retry_mode == "active",
            _ => false,
        };
    let response_gate_retry_contract_expected_lane_pressure_consistent =
        match response_gate_expected_retry_pressure_tier {
            "low" => response_gate_expected_next_action_lane == "none",
            "medium" | "high" => {
                response_gate_expected_next_action_lane
                    == "dashboard.troubleshooting.recent.state"
            }
            _ => false,
        };
    let response_gate_retry_contract_expected_command_pressure_consistent =
        match response_gate_expected_retry_pressure_tier {
            "low" => response_gate_expected_next_action_command == "none",
            "medium" | "high" => {
                response_gate_expected_next_action_command
                    == "dashboard.troubleshooting.recent.state --json"
            }
            _ => false,
        };
    let response_gate_retry_contract_expected_pressure_class_inverse_consistent =
        match response_gate_expected_retry_pressure_tier {
            "low" => response_gate_expected_retry_class == "none",
            "medium" => response_gate_expected_retry_class == "single_retry",
            "high" => response_gate_expected_retry_class == "bounded_retry",
            _ => false,
        };
    let response_gate_retry_contract_expected_command_mode_consistent =
        match response_gate_expected_retry_mode {
            "none" | "passive" => response_gate_expected_next_action_command == "none",
            "active" => {
                response_gate_expected_next_action_command
                    == "dashboard.troubleshooting.recent.state --json"
            }
            _ => false,
        };
    let response_gate_retry_contract_expected_after_seconds_mode_consistent =
        match response_gate_expected_retry_mode {
            "none" | "passive" => response_gate_expected_retry_after_seconds == 0,
            "active" => response_gate_expected_retry_after_seconds >= 1,
            _ => false,
        };
    let response_gate_retry_contract_expected_lane_after_seconds_consistent =
        match response_gate_expected_next_action_lane {
            "none" => response_gate_expected_retry_after_seconds == 0,
            "dashboard.troubleshooting.recent.state" => {
                response_gate_expected_retry_after_seconds >= 1
            }
            _ => false,
        };
    let response_gate_retry_contract_expected_command_after_seconds_consistent =
        match response_gate_expected_next_action_command {
            "none" => response_gate_expected_retry_after_seconds == 0,
            "dashboard.troubleshooting.recent.state --json" => {
                response_gate_expected_retry_after_seconds >= 1
            }
            _ => false,
        };
    let response_gate_retry_contract_expected_lane_command_after_seconds_consistent =
        if response_gate_expected_retry_after_seconds == 0 {
            response_gate_expected_next_action_lane == "none"
                && response_gate_expected_next_action_command == "none"
        } else {
            response_gate_expected_next_action_lane
                == "dashboard.troubleshooting.recent.state"
                && response_gate_expected_next_action_command
                    == "dashboard.troubleshooting.recent.state --json"
        };
    let response_gate_retry_contract_expected_lane_command_pressure_consistent =
        match response_gate_expected_retry_pressure_tier {
            "low" => {
                response_gate_expected_next_action_lane == "none"
                    && response_gate_expected_next_action_command == "none"
            }
            "medium" | "high" => {
                response_gate_expected_next_action_lane
                    == "dashboard.troubleshooting.recent.state"
                    && response_gate_expected_next_action_command
                        == "dashboard.troubleshooting.recent.state --json"
            }
            _ => false,
        };
    let response_gate_retry_contract_expected_lane_mode_pressure_consistent =
        match response_gate_expected_retry_pressure_tier {
            "low" => {
                response_gate_expected_next_action_lane == "none"
                    && (response_gate_expected_retry_mode == "none"
                        || response_gate_expected_retry_mode == "passive")
            }
            "medium" | "high" => {
                response_gate_expected_next_action_lane
                    == "dashboard.troubleshooting.recent.state"
                    && response_gate_expected_retry_mode == "active"
            }
            _ => false,
        };
    let response_gate_retry_contract_expected_lane_command_mode_consistent =
        match response_gate_expected_retry_mode {
            "none" | "passive" => {
                response_gate_expected_next_action_lane == "none"
                    && response_gate_expected_next_action_command == "none"
            }
            "active" => {
                response_gate_expected_next_action_lane
                    == "dashboard.troubleshooting.recent.state"
                    && response_gate_expected_next_action_command
                        == "dashboard.troubleshooting.recent.state --json"
            }
            _ => false,
        };
    let response_gate_retry_contract_expected_lane_command_mode_after_seconds_consistent =
        if response_gate_expected_retry_after_seconds == 0 {
            (response_gate_expected_retry_mode == "none"
                || response_gate_expected_retry_mode == "passive")
                && response_gate_expected_next_action_lane == "none"
                && response_gate_expected_next_action_command == "none"
        } else {
            response_gate_expected_retry_mode == "active"
                && response_gate_expected_next_action_lane
                    == "dashboard.troubleshooting.recent.state"
                && response_gate_expected_next_action_command
                    == "dashboard.troubleshooting.recent.state --json"
        };
    let response_gate_retry_contract_expected_lane_command_consistent =
        (response_gate_expected_next_action_lane == "none"
            && response_gate_expected_next_action_command == "none")
            || (response_gate_expected_next_action_lane
                == "dashboard.troubleshooting.recent.state"
                && response_gate_expected_next_action_command
                    == "dashboard.troubleshooting.recent.state --json");
    let response_gate_retry_contract_expected_lane_command_class_consistent =
        match response_gate_expected_retry_class {
            "none" => {
                response_gate_expected_next_action_lane == "none"
                    && response_gate_expected_next_action_command == "none"
            }
            "single_retry" | "bounded_retry" => {
                response_gate_expected_next_action_lane
                    == "dashboard.troubleshooting.recent.state"
                    && response_gate_expected_next_action_command
                        == "dashboard.troubleshooting.recent.state --json"
            }
            _ => false,
        };
    let response_gate_retry_contract_expected_lane_mode_class_consistent =
        match response_gate_expected_retry_class {
            "none" => {
                response_gate_expected_next_action_lane == "none"
                    && (response_gate_expected_retry_mode == "none"
                        || response_gate_expected_retry_mode == "passive")
            }
            "single_retry" | "bounded_retry" => {
                response_gate_expected_next_action_lane
                    == "dashboard.troubleshooting.recent.state"
                    && response_gate_expected_retry_mode == "active"
            }
            _ => false,
        };
    let response_gate_retry_contract_expected_lane_class_consistent =
        match response_gate_expected_retry_class {
            "none" => response_gate_expected_next_action_lane == "none",
            "single_retry" | "bounded_retry" => {
                response_gate_expected_next_action_lane
                    == "dashboard.troubleshooting.recent.state"
            }
            _ => false,
        };
    let response_gate_signature_key = format!(
        "ready={};severity={};primary={};lane={};reason={};count={};set={}",
        response_gate_ready,
        response_gate_severity,
        response_gate_primary_blocker,
        response_gate_escalation_lane,
        response_gate_escalation_reason_code,
        response_gate_blocker_count,
        response_gate_blocker_set_key
    );
    let response_gate_expected_signature_key = format!(
        "ready={};severity={};primary={};lane={};reason={};count={};set={}",
        response_gate_ready,
        response_gate_expected_severity,
        response_gate_primary_blocker_expected,
        response_gate_expected_escalation_lane,
        response_gate_expected_escalation_reason_code,
        response_gate_expected_blocker_count,
        response_gate_expected_blocker_set_key
    );
    let response_gate_signature_consistent =
        response_gate_signature_key == response_gate_expected_signature_key;
    let response_gate_escalation_reason_known = response_gate_escalation_reason_code != "unknown";
    let response_gate_escalation_vector_key = format!(
        "{}|{}|{}",
        response_gate_primary_blocker, response_gate_escalation_lane, response_gate_escalation_reason_code
    );
    let response_gate_expected_escalation_vector_key = format!(
        "{}|{}|{}",
        response_gate_primary_blocker_expected,
        response_gate_expected_escalation_lane,
        response_gate_expected_escalation_reason_code
    );
    let response_gate_escalation_signature_consistent =
        response_gate_escalation_vector_key == response_gate_expected_escalation_vector_key;
    let response_gate_escalation_vector_known = matches!(
        response_gate_escalation_vector_key.as_str(),
        "final_response_contract|dashboard.troubleshooting.recent.state|finalization_integrity_failure"
            | "answer_contract|dashboard.troubleshooting.recent.state|answer_integrity_failure"
            | "llm_reliability|dashboard.troubleshooting.snapshot.capture|llm_reliability_degraded"
            | "watchdog|dashboard.troubleshooting.summary|watchdog_pressure"
            | "none|none|none"
    );
    let response_gate_decision_vector_key = format!(
        "{}|{}|{}",
        response_gate_severity, response_gate_ready, !response_gate_ready
    );
    let response_gate_expected_decision_vector_key = format!(
        "{}|{}|{}",
        response_gate_expected_severity, response_gate_ready, !response_gate_ready
    );
    let response_gate_decision_signature_consistent =
        response_gate_decision_vector_key == response_gate_expected_decision_vector_key;
    let response_gate_decision_vector_known = matches!(
        response_gate_decision_vector_key.as_str(),
        "ready|true|false" | "degraded|false|true" | "blocked|false|true"
    );
