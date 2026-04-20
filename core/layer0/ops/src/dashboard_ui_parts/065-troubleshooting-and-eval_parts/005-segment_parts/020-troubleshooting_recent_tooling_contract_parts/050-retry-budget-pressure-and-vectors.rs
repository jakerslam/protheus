        "watch" => 90_i64,
        "weak" => 60_i64,
        _ => 0_i64,
    };
    let response_gate_retry_budget_band_consistent =
        response_gate_retry_budget_points == response_gate_expected_retry_budget_from_band;
    let response_gate_expected_retry_budget_from_expected_band = match response_gate_expected_score_band {
        "ready" => 120_i64,
        "strong" => 105_i64,
        "watch" => 90_i64,
        "weak" => 60_i64,
        _ => 0_i64,
    };
    let response_gate_retry_budget_expected_band_consistent = response_gate_expected_retry_budget_points
        == response_gate_expected_retry_budget_from_expected_band;
    let response_gate_retry_budget_range_consistent = (0_i64..=120_i64)
        .contains(&response_gate_retry_budget_points)
        && (0_i64..=120_i64).contains(&response_gate_expected_retry_budget_points);
    let response_gate_retry_budget_mode_consistent = if response_gate_retry_mode == "passive" {
        response_gate_retry_budget_points == 120 && response_gate_retry_after_seconds == 0
    } else {
        response_gate_retry_budget_points < 120 && response_gate_retry_after_seconds > 0
    };
    let response_gate_retry_contract_after_seconds_budget_consistent =
        if response_gate_retry_after_seconds == 0 {
            response_gate_retry_budget_points == 120
        } else {
            response_gate_retry_budget_points < 120
        };
    let response_gate_retry_pressure_tier = if response_gate_retry_budget_points >= 100 {
        "low"
    } else if response_gate_retry_budget_points >= 70 {
        "medium"
    } else {
        "high"
    };
    let response_gate_expected_retry_pressure_tier = if response_gate_score_band == "ready" {
        "low"
    } else if matches!(response_gate_score_band, "strong" | "watch") {
        "medium"
    } else {
        "high"
    };
    let response_gate_retry_pressure_tier_consistent =
        response_gate_retry_pressure_tier == response_gate_expected_retry_pressure_tier;
    let response_gate_retry_pressure_tier_known =
        matches!(response_gate_retry_pressure_tier, "low" | "medium" | "high");
    let response_gate_retry_contract_after_seconds_pressure_consistent =
        match response_gate_retry_pressure_tier {
            "low" => response_gate_retry_after_seconds == 0,
            "medium" => {
                response_gate_retry_after_seconds >= 1 && response_gate_retry_after_seconds < 60
            }
            "high" => response_gate_retry_after_seconds >= 60,
            _ => false,
        };
    let response_gate_retry_budget_vector_key = format!(
        "points={};tier={};mode={}",
        response_gate_retry_budget_points, response_gate_retry_pressure_tier, response_gate_retry_mode
    );
    let response_gate_expected_retry_budget_vector_key = format!(
        "points={};tier={};mode={}",
        response_gate_expected_retry_budget_from_expected_band,
        response_gate_expected_retry_pressure_tier,
        response_gate_expected_retry_mode
    );
    let response_gate_retry_budget_vector_consistent =
        response_gate_retry_budget_vector_key == response_gate_expected_retry_budget_vector_key;
    let response_gate_retry_budget_vector_known = matches!(
        response_gate_retry_budget_vector_key.as_str(),
        "points=120;tier=low;mode=passive"
            | "points=105;tier=medium;mode=active"
            | "points=90;tier=medium;mode=active"
            | "points=60;tier=high;mode=active"
            | "points=0;tier=high;mode=active"
    );
    let response_gate_retry_tier_window_consistent = match response_gate_retry_pressure_tier {
        "low" => response_gate_retry_after_seconds == 0,
        "medium" => (15_i64..=30_i64).contains(&response_gate_retry_after_seconds),
        "high" => response_gate_retry_after_seconds >= 60,
        _ => false,
    };
    let response_gate_retry_tier_mode_consistent = match response_gate_retry_pressure_tier {
        "low" => response_gate_retry_mode == "passive",
        "medium" | "high" => response_gate_retry_mode == "active",
        _ => false,
    };
    let response_gate_retry_tier_vector_key = format!(
        "tier={};after={};mode={}",
        response_gate_retry_pressure_tier, response_gate_retry_after_seconds, response_gate_retry_mode
    );
    let response_gate_expected_retry_tier_vector_key = format!(
        "tier={};after={};mode={}",
        response_gate_expected_retry_pressure_tier,
        response_gate_expected_retry_after_seconds,
        response_gate_expected_retry_mode
    );
    let response_gate_retry_tier_vector_consistent =
        response_gate_retry_tier_vector_key == response_gate_expected_retry_tier_vector_key;
    let response_gate_retry_tier_vector_known = matches!(
        response_gate_retry_tier_vector_key.as_str(),
        "tier=low;after=0;mode=passive"
            | "tier=medium;after=15;mode=active"
            | "tier=medium;after=30;mode=active"
            | "tier=high;after=60;mode=active"
            | "tier=high;after=90;mode=active"
            | "tier=high;after=120;mode=active"
    );
    let response_gate_retry_contract_vector_key = format!(
        "class={};budget={};tier={};after={};mode={};lane={}",
        response_gate_retry_class,
        response_gate_retry_budget_points,
        response_gate_retry_pressure_tier,
        response_gate_retry_after_seconds,
        response_gate_retry_mode,
        response_gate_next_action_lane
    );
    let response_gate_expected_retry_contract_vector_key = format!(
        "class={};budget={};tier={};after={};mode={};lane={}",
        response_gate_expected_retry_class,
        response_gate_expected_retry_budget_points,
        response_gate_expected_retry_pressure_tier,
        response_gate_expected_retry_after_seconds,
        response_gate_expected_retry_mode,
        response_gate_expected_next_action_lane
    );
    let response_gate_retry_contract_vector_consistent = response_gate_retry_contract_vector_key
        == response_gate_expected_retry_contract_vector_key;
    let response_gate_retry_contract_vector_known = matches!(
        response_gate_retry_contract_vector_key.as_str(),
        "class=none;budget=120;tier=low;after=0;mode=passive;lane=none"
            | "class=single_retry;budget=90;tier=medium;after=30;mode=active;lane=dashboard.troubleshooting.recent.state"
            | "class=bounded_retry;budget=60;tier=high;after=60;mode=active;lane=dashboard.troubleshooting.recent.state"
            | "class=bounded_retry;budget=0;tier=high;after=120;mode=active;lane=dashboard.troubleshooting.recent.state"
    );
    let response_gate_retry_contract_family_consistent = match response_gate_retry_class {
        "none" => {
            response_gate_retry_budget_points == 120
                && response_gate_retry_pressure_tier == "low"
                && response_gate_retry_after_seconds == 0
                && response_gate_retry_mode == "passive"
                && response_gate_next_action_lane == "none"
        }
        "single_retry" => {
            response_gate_retry_budget_points == 90
                && response_gate_retry_pressure_tier == "medium"
                && response_gate_retry_after_seconds == 30
                && response_gate_retry_mode == "active"
                && response_gate_next_action_lane == "dashboard.troubleshooting.recent.state"
        }
        "bounded_retry" => {
            response_gate_retry_budget_points <= 60
                && response_gate_retry_pressure_tier == "high"
                && response_gate_retry_after_seconds >= 60
                && response_gate_retry_mode == "active"
                && response_gate_next_action_lane == "dashboard.troubleshooting.recent.state"
        }
        _ => false,
    };
    let response_gate_retry_contract_severity_consistent = match response_gate_severity {
        "ready" => {
            response_gate_retry_class == "none"
                && response_gate_retry_pressure_tier == "low"
                && response_gate_retry_mode == "passive"
                && response_gate_next_action_lane == "none"
        }
        "degraded" => {
            response_gate_retry_class == "single_retry"
                && response_gate_retry_pressure_tier == "medium"
                && response_gate_retry_mode == "active"
                && response_gate_next_action_lane == "dashboard.troubleshooting.recent.state"
        }
        "blocked" => {
            response_gate_retry_class == "bounded_retry"
                && response_gate_retry_pressure_tier == "high"
                && response_gate_retry_mode == "active"
                && response_gate_next_action_lane == "dashboard.troubleshooting.recent.state"
        }
        _ => false,
    };
    let response_gate_retry_contract_coherence_consistent =
        response_gate_retry_contract_vector_consistent
            && response_gate_retry_contract_family_consistent
            && response_gate_retry_contract_severity_consistent
            && response_gate_retry_mode_consistent
            && response_gate_next_action_lane_consistent
            && response_gate_retry_window_consistent;
    let response_gate_retry_contract_lane_class_consistent = match response_gate_retry_class {
        "none" => response_gate_next_action_lane == "none",
        "single_retry" | "bounded_retry" => {
            response_gate_next_action_lane == "dashboard.troubleshooting.recent.state"
        }
        _ => false,
    };
    let response_gate_retry_contract_command_class_consistent = match response_gate_retry_class {
        "none" => response_gate_next_action_command == "none",
        "single_retry" | "bounded_retry" => {
            response_gate_next_action_command == "dashboard.troubleshooting.recent.state --json"
        }
        _ => false,
    };
    let response_gate_retry_contract_expected_class_consistent =
        match response_gate_expected_retry_class {
            "none" => {
                response_gate_expected_retry_pressure_tier == "low"
                    && response_gate_expected_retry_after_seconds == 0
                    && response_gate_expected_retry_mode == "passive"
                    && response_gate_expected_next_action_lane == "none"
                    && response_gate_expected_next_action_command == "none"
            }
            "single_retry" => {
                response_gate_expected_retry_pressure_tier == "medium"
                    && response_gate_expected_retry_after_seconds == 30
                    && response_gate_expected_retry_mode == "active"
                    && response_gate_expected_next_action_lane
                        == "dashboard.troubleshooting.recent.state"
                    && response_gate_expected_next_action_command
                        == "dashboard.troubleshooting.recent.state --json"
            }
            "bounded_retry" => {
                response_gate_expected_retry_pressure_tier == "high"
                    && response_gate_expected_retry_after_seconds >= 60
                    && response_gate_expected_retry_mode == "active"
                    && response_gate_expected_next_action_lane
                        == "dashboard.troubleshooting.recent.state"
                    && response_gate_expected_next_action_command
                        == "dashboard.troubleshooting.recent.state --json"
            }
            _ => false,
        };
    let response_gate_retry_contract_pressure_class_consistent = match response_gate_retry_class {
        "none" => response_gate_retry_pressure_tier == "low",
        "single_retry" => response_gate_retry_pressure_tier == "medium",
        "bounded_retry" => response_gate_retry_pressure_tier == "high",
        _ => false,
    };
    let response_gate_retry_contract_expected_pressure_class_consistent =
        match response_gate_expected_retry_class {
            "none" => response_gate_expected_retry_pressure_tier == "low",
            "single_retry" => response_gate_expected_retry_pressure_tier == "medium",
            "bounded_retry" => response_gate_expected_retry_pressure_tier == "high",
            _ => false,
        };
    let response_gate_retry_contract_expected_command_class_consistent =
        match response_gate_expected_retry_class {
            "none" => response_gate_expected_next_action_command == "none",
            "single_retry" | "bounded_retry" => {
                response_gate_expected_next_action_command
                    == "dashboard.troubleshooting.recent.state --json"
            }
            _ => false,
        };
    let response_gate_retry_contract_expected_mode_class_consistent =
        match response_gate_expected_retry_class {
            "none" => response_gate_expected_retry_mode == "none",
            "single_retry" | "bounded_retry" => {
                response_gate_expected_retry_mode == "active"
            }
            _ => false,
        };
    let response_gate_retry_contract_expected_after_seconds_class_consistent =
        match response_gate_expected_retry_class {
            "none" => response_gate_expected_retry_after_seconds == 0,
            "single_retry" => response_gate_expected_retry_after_seconds == 30,
            "bounded_retry" => response_gate_expected_retry_after_seconds == 60,
            _ => false,
        };
    let response_gate_retry_contract_expected_after_seconds_band_class_consistent =
        match response_gate_expected_retry_class {
            "none" => response_gate_expected_retry_after_seconds == 0,
            "single_retry" => {
                response_gate_expected_retry_after_seconds >= 1
                    && response_gate_expected_retry_after_seconds < 60
            }
            "bounded_retry" => response_gate_expected_retry_after_seconds >= 60,
            _ => false,
        };
    let response_gate_retry_contract_expected_after_seconds_pressure_consistent =
        match response_gate_expected_retry_pressure_tier {
            "low" => response_gate_expected_retry_after_seconds == 0,
            "medium" => {
                response_gate_expected_retry_after_seconds >= 1
                    && response_gate_expected_retry_after_seconds < 60
            }
            "high" => response_gate_expected_retry_after_seconds >= 60,
            _ => false,
        };
