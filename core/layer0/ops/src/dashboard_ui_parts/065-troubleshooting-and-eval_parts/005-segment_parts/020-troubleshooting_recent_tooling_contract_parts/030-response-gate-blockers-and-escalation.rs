    let response_gate_blockers = dashboard_response_gate_blockers_from_flags(
        final_response_contract_ok,
        answer_contract_ok,
        llm_reliability_not_low,
        watchdog_triggered,
    );
    let response_gate_expected_blockers = dashboard_response_gate_blockers_from_flags(
        final_response_contract_ok,
        answer_contract_ok,
        llm_reliability_not_low,
        watchdog_triggered,
    );
    let response_gate_blocker_set_consistent = response_gate_blockers == response_gate_expected_blockers;
    let response_gate_blocker_set_key = if response_gate_blockers.is_empty() {
        "none".to_string()
    } else {
        response_gate_blockers.join("|")
    };
    let response_gate_expected_blocker_set_key = if response_gate_expected_blockers.is_empty() {
        "none".to_string()
    } else {
        response_gate_expected_blockers.join("|")
    };
    let response_gate_blocker_set_key_consistent =
        response_gate_blocker_set_key == response_gate_expected_blocker_set_key;
    let response_gate_blocker_count = response_gate_blockers.len() as i64;
    let response_gate_blocker_count_key_consistent =
        if response_gate_blocker_count == 0 {
            response_gate_blocker_set_key == "none"
        } else {
            response_gate_blocker_set_key != "none"
        };
    let response_gate_expected_blocker_count = response_gate_expected_blockers.len() as i64;
    let response_gate_expected_blocker_count_matches =
        response_gate_expected_blocker_count == response_gate_blocker_count;
    let response_gate_blocker_budget_max = 4_i64;
    let response_gate_blocker_budget_consistent = response_gate_blocker_count >= 0
        && response_gate_expected_blocker_count >= 0
        && response_gate_blocker_count <= response_gate_blocker_budget_max
        && response_gate_expected_blocker_count <= response_gate_blocker_budget_max;
    let response_gate_blocker_has_final_response_contract = response_gate_blockers
        .iter()
        .any(|row| row == "final_response_contract");
    let response_gate_blocker_has_answer_contract =
        response_gate_blockers.iter().any(|row| row == "answer_contract");
    let response_gate_blocker_has_llm_reliability =
        response_gate_blockers.iter().any(|row| row == "llm_reliability");
    let response_gate_blocker_has_watchdog = response_gate_blockers.iter().any(|row| row == "watchdog");
    let response_gate_blocker_flags_key = format!(
        "final_response_contract={};answer_contract={};llm_reliability={};watchdog={}",
        response_gate_blocker_has_final_response_contract,
        response_gate_blocker_has_answer_contract,
        response_gate_blocker_has_llm_reliability,
        response_gate_blocker_has_watchdog
    );
    let response_gate_expected_blocker_flags_key = format!(
        "final_response_contract={};answer_contract={};llm_reliability={};watchdog={}",
        !final_response_contract_ok,
        !answer_contract_ok,
        !llm_reliability_not_low,
        watchdog_triggered
    );
    let response_gate_blocker_flags_consistent = response_gate_blocker_flags_key
        == response_gate_expected_blocker_flags_key
        && response_gate_blocker_count
            == i64::from(response_gate_blocker_has_final_response_contract)
                + i64::from(response_gate_blocker_has_answer_contract)
                + i64::from(response_gate_blocker_has_llm_reliability)
                + i64::from(response_gate_blocker_has_watchdog);
    let response_gate_primary_blocker = response_gate_blockers
        .first()
        .map(|row| row.as_str())
        .unwrap_or("none");
    let response_gate_primary_blocker_expected = if !final_response_contract_ok {
        "final_response_contract"
    } else if !answer_contract_ok {
        "answer_contract"
    } else if !llm_reliability_not_low {
        "llm_reliability"
    } else if watchdog_triggered {
        "watchdog"
    } else {
        "none"
    };
    let response_gate_blocker_priority_consistent =
        response_gate_primary_blocker == response_gate_primary_blocker_expected;
    let response_gate_blocker_vector_key = format!(
        "count={};set={};primary={}",
        response_gate_blocker_count, response_gate_blocker_set_key, response_gate_primary_blocker
    );
    let response_gate_expected_blocker_vector_key = format!(
        "count={};set={};primary={}",
        response_gate_expected_blocker_count,
        response_gate_expected_blocker_set_key,
        response_gate_primary_blocker_expected
    );
    let response_gate_blocker_vector_consistent =
        response_gate_blocker_vector_key == response_gate_expected_blocker_vector_key;
    let response_gate_primary_blocker_known = matches!(
        response_gate_primary_blocker,
        "final_response_contract" | "answer_contract" | "llm_reliability" | "watchdog" | "none"
    );
    let response_gate_escalation_lane = match response_gate_primary_blocker {
        "final_response_contract" | "answer_contract" => "dashboard.troubleshooting.recent.state",
        "llm_reliability" => "dashboard.troubleshooting.snapshot.capture",
        "watchdog" => "dashboard.troubleshooting.summary",
        _ => "none",
    };
    let response_gate_escalation_lane_known = matches!(
        response_gate_escalation_lane,
        "dashboard.troubleshooting.recent.state"
            | "dashboard.troubleshooting.snapshot.capture"
            | "dashboard.troubleshooting.summary"
            | "none"
    );
    let response_gate_blockers_consistent = if response_gate_ready {
        response_gate_blocker_count == 0
            && response_gate_primary_blocker == "none"
            && response_gate_escalation_lane == "none"
    } else {
        response_gate_blocker_count > 0
            && response_gate_primary_blocker != "none"
            && response_gate_escalation_lane != "none"
    };
    let response_gate_severity_consistent = if response_gate_ready {
        response_gate_severity == "ready"
    } else if response_gate_score >= 0.6 {
        response_gate_severity == "degraded"
    } else {
        response_gate_severity == "blocked"
    };
    let response_gate_manual_review_consistent = !response_gate_ready;
    let response_gate_requires_manual_review = !response_gate_ready;
    let response_gate_expected_requires_manual_review = response_gate_expected_severity != "ready";
    let response_gate_manual_review_signature_consistent =
        response_gate_requires_manual_review == response_gate_expected_requires_manual_review;
    let response_gate_manual_review_reason = if response_gate_requires_manual_review {
        "gated_response_not_ready"
    } else {
        "none"
    };
    let response_gate_expected_manual_review_reason = if response_gate_expected_requires_manual_review {
        "gated_response_not_ready"
    } else {
        "none"
    };
    let response_gate_manual_review_reason_consistent =
        response_gate_manual_review_reason == response_gate_expected_manual_review_reason;
    let response_gate_manual_review_reason_known =
        matches!(response_gate_manual_review_reason, "gated_response_not_ready" | "none");
    let response_gate_manual_review_vector_key = format!(
        "required={};reason={}",
        response_gate_requires_manual_review, response_gate_manual_review_reason
    );
    let response_gate_expected_manual_review_vector_key = format!(
        "required={};reason={}",
        response_gate_expected_requires_manual_review, response_gate_expected_manual_review_reason
    );
    let response_gate_manual_review_vector_consistent =
        response_gate_manual_review_vector_key == response_gate_expected_manual_review_vector_key;
    let response_gate_manual_review_vector_known = matches!(
        response_gate_manual_review_vector_key.as_str(),
        "required=true;reason=gated_response_not_ready" | "required=false;reason=none"
    );
    let response_gate_blocker_count_matches =
        response_gate_blocker_count == response_gate_blockers.len() as i64;
    let response_gate_primary_blocker_matches = if response_gate_blocker_count == 0 {
        response_gate_primary_blocker == "none"
    } else {
        response_gate_blockers
            .first()
            .is_some_and(|row| row == response_gate_primary_blocker)
    };
    let response_gate_escalation_contract_ok = if response_gate_primary_blocker == "none" {
        response_gate_escalation_lane == "none"
    } else {
        response_gate_escalation_lane != "none"
    };
    let response_gate_escalation_reason_code = match response_gate_primary_blocker {
        "final_response_contract" => "finalization_integrity_failure",
        "answer_contract" => "answer_integrity_failure",
        "llm_reliability" => "llm_reliability_degraded",
        "watchdog" => "watchdog_pressure",
        "none" => "none",
        _ => "unknown",
    };
    let response_gate_next_action_command = match response_gate_primary_blocker {
        "final_response_contract" | "answer_contract" => {
            "dashboard.troubleshooting.recent.state --json"
        }
        "llm_reliability" => "dashboard.troubleshooting.snapshot.capture --include-contract --json",
        "watchdog" => "dashboard.troubleshooting.summary --limit=20 --json",
        _ => "none",
    };
    let response_gate_expected_escalation_lane = match response_gate_primary_blocker_expected {
        "final_response_contract" | "answer_contract" => "dashboard.troubleshooting.recent.state",
        "llm_reliability" => "dashboard.troubleshooting.snapshot.capture",
        "watchdog" => "dashboard.troubleshooting.summary",
        _ => "none",
    };
    let response_gate_expected_escalation_reason_code = match response_gate_primary_blocker_expected {
        "final_response_contract" => "finalization_integrity_failure",
        "answer_contract" => "answer_integrity_failure",
        "llm_reliability" => "llm_reliability_degraded",
        "watchdog" => "watchdog_pressure",
        _ => "none",
    };
    let response_gate_expected_next_action_command = match response_gate_primary_blocker_expected {
        "final_response_contract" | "answer_contract" => {
            "dashboard.troubleshooting.recent.state --json"
        }
        "llm_reliability" => "dashboard.troubleshooting.snapshot.capture --include-contract --json",
        "watchdog" => "dashboard.troubleshooting.summary --limit=20 --json",
        _ => "none",
    };
