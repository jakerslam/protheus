    let response_gate_next_action_command_consistent =
        response_gate_next_action_command == response_gate_expected_next_action_command;
    let response_gate_next_action_command_known = matches!(
        response_gate_next_action_command,
        "dashboard.troubleshooting.recent.state --json"
            | "dashboard.troubleshooting.snapshot.capture --include-contract --json"
            | "dashboard.troubleshooting.summary --limit=20 --json"
            | "none"
    );
    let response_gate_retry_class = if response_gate_ready {
        "none"
    } else if response_gate_severity == "degraded" {
        "single_retry"
    } else {
        "bounded_retry"
    };
    let response_gate_expected_retry_class = if response_gate_expected_severity == "ready" {
        "none"
    } else if response_gate_expected_severity == "degraded" {
        "single_retry"
    } else {
        "bounded_retry"
    };
    let response_gate_retry_class_consistent =
        response_gate_retry_class == response_gate_expected_retry_class;
    let response_gate_retry_class_known = matches!(
        response_gate_retry_class,
        "none" | "single_retry" | "bounded_retry"
    );
    let response_gate_retry_after_seconds = match response_gate_score_band {
        "ready" => 0_i64,
        "strong" => 15_i64,
        "watch" => 30_i64,
        "weak" => 60_i64,
        _ => 120_i64,
    };
    let response_gate_expected_retry_after_seconds = match response_gate_expected_score_band {
        "ready" => 0_i64,
        "strong" => 15_i64,
        "watch" => 30_i64,
        "weak" => 60_i64,
        _ => 120_i64,
    };
    let response_gate_retry_after_seconds_consistent =
        response_gate_retry_after_seconds == response_gate_expected_retry_after_seconds;
    let response_gate_retry_after_seconds_non_negative =
        response_gate_retry_after_seconds >= 0 && response_gate_expected_retry_after_seconds >= 0;
    let response_gate_next_action_lane_consistent = match response_gate_escalation_lane {
        "none" => response_gate_next_action_command == "none",
        "dashboard.troubleshooting.recent.state" => {
            response_gate_next_action_command
                .starts_with("dashboard.troubleshooting.recent.state")
        }
        "dashboard.troubleshooting.snapshot.capture" => {
            response_gate_next_action_command
                .starts_with("dashboard.troubleshooting.snapshot.capture")
        }
        "dashboard.troubleshooting.summary" => {
            response_gate_next_action_command
                .starts_with("dashboard.troubleshooting.summary")
        }
        _ => false,
    };
    let response_gate_retry_command_consistent = if response_gate_retry_class == "none" {
        response_gate_next_action_command == "none"
    } else {
        response_gate_next_action_command != "none"
    };
    let response_gate_retry_window_consistent = if response_gate_retry_class == "none" {
        response_gate_retry_after_seconds == 0
    } else {
        response_gate_retry_after_seconds > 0
    };
    let response_gate_retry_signature_key = format!(
        "class={};after={};lane={}",
        response_gate_retry_class, response_gate_retry_after_seconds, response_gate_escalation_lane
    );
    let response_gate_expected_retry_signature_key = format!(
        "class={};after={};lane={}",
        response_gate_expected_retry_class,
        response_gate_expected_retry_after_seconds,
        response_gate_expected_escalation_lane
    );
    let response_gate_retry_signature_consistent =
        response_gate_retry_signature_key == response_gate_expected_retry_signature_key;
    let response_gate_retry_signature_known = matches!(
        response_gate_retry_signature_key.as_str(),
        "class=none;after=0;lane=none"
            | "class=single_retry;after=15;lane=dashboard.troubleshooting.snapshot.capture"
            | "class=single_retry;after=15;lane=dashboard.troubleshooting.summary"
            | "class=single_retry;after=30;lane=dashboard.troubleshooting.recent.state"
            | "class=bounded_retry;after=60;lane=dashboard.troubleshooting.recent.state"
            | "class=bounded_retry;after=120;lane=dashboard.troubleshooting.recent.state"
            | "class=bounded_retry;after=120;lane=dashboard.troubleshooting.snapshot.capture"
            | "class=bounded_retry;after=120;lane=dashboard.troubleshooting.summary"
    );
    let response_gate_lane_retry_window_consistent = match response_gate_escalation_lane {
        "none" => response_gate_retry_after_seconds == 0,
        "dashboard.troubleshooting.recent.state" => response_gate_retry_after_seconds >= 30,
        "dashboard.troubleshooting.snapshot.capture" => {
            response_gate_retry_after_seconds > 0 && response_gate_retry_after_seconds <= 30
        }
        "dashboard.troubleshooting.summary" => {
            response_gate_retry_after_seconds > 0 && response_gate_retry_after_seconds <= 30
        }
        _ => false,
    };
    let response_gate_retry_band_consistent = match response_gate_retry_class {
        "none" => response_gate_score_band == "ready",
        "single_retry" => matches!(response_gate_score_band, "strong" | "watch"),
        "bounded_retry" => matches!(response_gate_score_band, "weak" | "critical"),
        _ => false,
    };
    let response_gate_retry_contract_after_seconds_class_consistent =
        match response_gate_retry_class {
            "none" => response_gate_retry_after_seconds == 0,
            "single_retry" => {
                response_gate_retry_after_seconds >= 1 && response_gate_retry_after_seconds < 60
            }
            "bounded_retry" => response_gate_retry_after_seconds >= 60,
            _ => false,
        };
    let response_gate_retry_contract_after_seconds_score_band_consistent =
        match response_gate_score_band {
            "ready" => response_gate_retry_after_seconds == 0,
            "strong" | "watch" => {
                response_gate_retry_after_seconds >= 1 && response_gate_retry_after_seconds < 60
            }
            "weak" | "critical" => response_gate_retry_after_seconds >= 60,
            _ => false,
        };
    let response_gate_retry_contract_after_seconds_lane_consistent =
        match response_gate_next_action_lane {
            "none" => response_gate_retry_after_seconds == 0,
            "dashboard.troubleshooting.recent.state"
            | "dashboard.troubleshooting.snapshot.capture"
            | "dashboard.troubleshooting.summary" => response_gate_retry_after_seconds > 0,
            _ => false,
        };
    let response_gate_retry_contract_after_seconds_next_action_window_consistent =
        match response_gate_next_action_lane {
            "none" => response_gate_retry_after_seconds == 0,
            "dashboard.troubleshooting.recent.state" => response_gate_retry_after_seconds >= 30,
            "dashboard.troubleshooting.snapshot.capture"
            | "dashboard.troubleshooting.summary" => {
                response_gate_retry_after_seconds > 0 && response_gate_retry_after_seconds <= 30
            }
            _ => false,
        };
    let response_gate_retry_contract_lane_command_consistent =
        (response_gate_next_action_lane == "none" && response_gate_next_action_command == "none")
            || (response_gate_next_action_lane == "dashboard.troubleshooting.recent.state"
                && response_gate_next_action_command
                    == "dashboard.troubleshooting.recent.state --json")
            || (response_gate_next_action_lane == "dashboard.troubleshooting.snapshot.capture"
                && response_gate_next_action_command
                    == "dashboard.troubleshooting.snapshot.capture --json")
            || (response_gate_next_action_lane == "dashboard.troubleshooting.summary"
                && response_gate_next_action_command
                    == "dashboard.troubleshooting.summary --json");
    let response_gate_retry_contract_after_seconds_lane_command_consistent =
        if response_gate_retry_after_seconds == 0 {
            response_gate_next_action_lane == "none"
                && response_gate_next_action_command == "none"
        } else {
            response_gate_next_action_lane != "none"
                && response_gate_next_action_command != "none"
        };
    let response_gate_retry_contract_after_seconds_command_consistent =
        if response_gate_next_action_command == "none" {
            response_gate_retry_after_seconds == 0
        } else {
            response_gate_retry_after_seconds > 0
        };
    let response_gate_retry_mode = if response_gate_next_action_command == "none" {
        "passive"
    } else {
        "active"
    };
    let response_gate_expected_retry_mode = if response_gate_expected_next_action_command == "none" {
        "passive"
    } else {
        "active"
    };
    let response_gate_retry_mode_consistent =
        response_gate_retry_mode == response_gate_expected_retry_mode;
    let response_gate_retry_mode_known =
        matches!(response_gate_retry_mode, "passive" | "active");
    let response_gate_retry_contract_lane_mode_consistent = if response_gate_next_action_lane == "none"
    {
        response_gate_retry_mode == "passive"
    } else {
        response_gate_retry_mode == "active"
    };
    let response_gate_retry_contract_after_seconds_mode_consistent = match response_gate_retry_mode {
        "passive" => response_gate_retry_after_seconds == 0,
        "active" => response_gate_retry_after_seconds >= 1,
        _ => false,
    };
    let response_gate_retry_contract_after_seconds_lane_mode_consistent =
        if response_gate_retry_after_seconds == 0 {
            response_gate_next_action_lane == "none" && response_gate_retry_mode == "passive"
        } else {
            response_gate_next_action_lane != "none" && response_gate_retry_mode == "active"
        };
    let response_gate_retry_contract_lane_command_mode_consistent =
        if response_gate_retry_mode == "passive" {
            response_gate_next_action_lane == "none" && response_gate_next_action_command == "none"
        } else {
            response_gate_next_action_lane != "none" && response_gate_next_action_command != "none"
        };
    let response_gate_retry_contract_after_seconds_lane_command_mode_consistent =
        if response_gate_retry_after_seconds == 0 {
            response_gate_next_action_lane == "none"
                && response_gate_next_action_command == "none"
                && response_gate_retry_mode == "passive"
        } else {
            response_gate_next_action_lane != "none"
                && response_gate_next_action_command != "none"
                && response_gate_retry_mode == "active"
        };
    let response_gate_retry_contract_after_seconds_command_mode_consistent =
        if response_gate_retry_after_seconds == 0 {
            response_gate_next_action_command == "none" && response_gate_retry_mode == "passive"
        } else {
            response_gate_next_action_command != "none" && response_gate_retry_mode == "active"
        };
    let response_gate_retry_action_vector_key = format!(
        "{}|{}|{}",
        response_gate_retry_class, response_gate_retry_mode, response_gate_next_action_command
    );
    let response_gate_expected_retry_action_vector_key = format!(
        "{}|{}|{}",
        response_gate_expected_retry_class,
        response_gate_expected_retry_mode,
        response_gate_expected_next_action_command
    );
    let response_gate_retry_action_vector_consistent =
        response_gate_retry_action_vector_key == response_gate_expected_retry_action_vector_key;
    let response_gate_retry_budget_points = (120_i64 - response_gate_retry_after_seconds).max(0);
    let response_gate_expected_retry_budget_points =
        (120_i64 - response_gate_expected_retry_after_seconds).max(0);
    let response_gate_retry_budget_consistent =
        response_gate_retry_budget_points == response_gate_expected_retry_budget_points;
    let response_gate_retry_budget_non_negative =
        response_gate_retry_budget_points >= 0 && response_gate_expected_retry_budget_points >= 0;
    let response_gate_expected_retry_budget_from_band = match response_gate_score_band {
        "ready" => 120_i64,
        "strong" => 105_i64,
