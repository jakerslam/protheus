
pub fn compute_policy_hold_latest_event(
    input: &PolicyHoldLatestEventInput,
) -> PolicyHoldLatestEventOutput {
    for (idx, evt) in input.events.iter().enumerate().rev() {
        let event_type = evt
            .event_type
            .as_ref()
            .map(|v| v.trim().to_ascii_lowercase())
            .unwrap_or_default();
        if event_type != "autonomy_run" {
            continue;
        }

        let result = evt
            .result
            .as_ref()
            .map(|v| v.trim().to_string())
            .unwrap_or_default();
        if !evt.policy_hold.unwrap_or(false) && !is_policy_hold_result(&result) {
            continue;
        }

        return PolicyHoldLatestEventOutput {
            found: true,
            event_index: Some(idx as u32),
            result: evt.result.as_ref().map(|v| v.to_string()),
            ts: evt
                .ts
                .as_ref()
                .map(|v| v.to_string())
                .filter(|v| !v.trim().is_empty()),
            ts_ms: non_negative_number(evt.ts_ms),
            hold_reason: policy_hold_reason_from_latest_entry(evt),
            route_block_reason: evt
                .route_block_reason
                .as_ref()
                .map(|v| v.trim().to_string())
                .filter(|v| !v.is_empty()),
        };
    }

    PolicyHoldLatestEventOutput {
        found: false,
        event_index: None,
        result: None,
        ts: None,
        ts_ms: None,
        hold_reason: None,
        route_block_reason: None,
    }
}

fn minutes_until_next_utc_day(now_ms: f64) -> u32 {
    let now = if now_ms.is_finite() && now_ms > 0.0 {
        now_ms as i64
    } else {
        0
    };
    if now <= 0 {
        return 0;
    }
    let secs = now / 1000;
    let rem_ms = (now % 1000) as u32;
    let Some(dt) = chrono::DateTime::<chrono::Utc>::from_timestamp(secs, rem_ms * 1_000_000) else {
        return 0;
    };
    let date = dt.date_naive();
    let Some(next_day) = date.succ_opt() else {
        return 0;
    };
    let Some(next_midnight) = next_day.and_hms_opt(0, 0, 0) else {
        return 0;
    };
    let delta_ms = (next_midnight - dt.naive_utc()).num_milliseconds().max(0);
    ((delta_ms + 59_999) / 60_000) as u32
}

pub fn compute_policy_hold_cooldown(input: &PolicyHoldCooldownInput) -> PolicyHoldCooldownOutput {
    let mut cooldown = non_negative_number(input.base_minutes).unwrap_or(0.0);
    let pressure_applicable = input.pressure_applicable.unwrap_or(false);
    let pressure_level = input
        .pressure_level
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    let cooldown_warn = non_negative_number(input.cooldown_warn_minutes).unwrap_or(30.0);
    let cooldown_hard = non_negative_number(input.cooldown_hard_minutes).unwrap_or(60.0);

    if pressure_applicable && pressure_level == "hard" {
        cooldown = cooldown.max(cooldown_hard);
    } else if pressure_applicable && pressure_level == "warn" {
        cooldown = cooldown.max(cooldown_warn);
    }

    let result = input
        .last_result
        .as_ref()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    if !result.is_empty() {
        let until_next_day_caps = input.until_next_day_caps.unwrap_or(true);
        let now_ms = non_negative_number(input.now_ms).unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|v| v.as_millis() as f64)
                .unwrap_or(0.0)
        });
        let cap_minutes = non_negative_number(input.cooldown_cap_minutes).unwrap_or(180.0);
        let manual_review_minutes =
            non_negative_number(input.cooldown_manual_review_minutes).unwrap_or(90.0);
        let unchanged_state_minutes =
            non_negative_number(input.cooldown_unchanged_state_minutes).unwrap_or(90.0);
        let readiness_retry_minutes =
            non_negative_number(input.readiness_retry_minutes).unwrap_or(120.0);

        if result == "no_candidates_policy_daily_cap" || result == "no_candidates_policy_canary_cap"
        {
            let cap_cooldown = if until_next_day_caps {
                minutes_until_next_utc_day(now_ms) as f64
            } else {
                cap_minutes
            };
            cooldown = cooldown.max(cap_cooldown);
        } else if result == "no_candidates_policy_manual_review_pending"
            || result == "stop_repeat_gate_human_escalation_pending"
        {
            cooldown = cooldown.max(manual_review_minutes);
        } else if result == "no_candidates_policy_unchanged_state" {
            cooldown = cooldown.max(unchanged_state_minutes);
        } else if result == "stop_init_gate_readiness"
            || result == "stop_init_gate_readiness_blocked"
            || result == "stop_init_gate_criteria_quality_insufficient"
        {
            cooldown = cooldown.max(readiness_retry_minutes);
        }
    }

    PolicyHoldCooldownOutput {
        cooldown_minutes: cooldown.round().max(0.0) as u32,
    }
}

pub fn compute_receipt_verdict(input: &ReceiptVerdictInput) -> ReceiptVerdictOutput {
    let decision = input.decision.trim().to_ascii_uppercase();
    let exec_check_name = if decision == "ACTUATE" {
        "actuation_execute_ok".to_string()
    } else if decision == "DIRECTIVE_VALIDATE" {
        "directive_validate_ok".to_string()
    } else if decision == "DIRECTIVE_DECOMPOSE" {
        "directive_decompose_ok".to_string()
    } else {
        "route_execute_ok".to_string()
    };

    let route_attestation_status = input.route_attestation_status.trim().to_ascii_lowercase();
    let route_expected_model = input.route_attestation_expected_model.trim();
    let route_attestation_mismatch =
        !route_expected_model.is_empty() && route_attestation_status == "mismatch";

    let criteria_pass = if input.success_criteria_required {
        input.success_criteria_passed
    } else {
        true
    };
    let checks = vec![
        ReceiptCheck {
            name: exec_check_name.clone(),
            pass: input.exec_ok,
        },
        ReceiptCheck {
            name: "postconditions_ok".to_string(),
            pass: input.postconditions_ok,
        },
        ReceiptCheck {
            name: "dod_passed".to_string(),
            pass: input.dod_passed,
        },
        ReceiptCheck {
            name: "success_criteria_met".to_string(),
            pass: criteria_pass,
        },
        ReceiptCheck {
            name: "queue_outcome_logged".to_string(),
            pass: input.queue_outcome_logged,
        },
        ReceiptCheck {
            name: "route_model_attested".to_string(),
            pass: !route_attestation_mismatch,
        },
    ];

    let failed: Vec<String> = checks
        .iter()
        .filter(|row| !row.pass)
        .map(|row| row.name.clone())
        .collect();
    let passed = failed.is_empty();
    let mut outcome = "shipped".to_string();
    let exec_check_pass = checks
        .iter()
        .find(|row| row.name == exec_check_name)
        .map(|row| row.pass)
        .unwrap_or(false);
    let postconditions_ok = checks
        .iter()
        .find(|row| row.name == "postconditions_ok")
        .map(|row| row.pass)
        .unwrap_or(false);
    let queue_outcome_logged = checks
        .iter()
        .find(|row| row.name == "queue_outcome_logged")
        .map(|row| row.pass)
        .unwrap_or(false);
    let route_model_attested = checks
        .iter()
        .find(|row| row.name == "route_model_attested")
        .map(|row| row.pass)
        .unwrap_or(false);
    let dod_passed = checks
        .iter()
        .find(|row| row.name == "dod_passed")
        .map(|row| row.pass)
        .unwrap_or(false);
    let success_criteria_met = checks
        .iter()
        .find(|row| row.name == "success_criteria_met")
        .map(|row| row.pass)
        .unwrap_or(false);

    if !exec_check_pass || !postconditions_ok || !queue_outcome_logged || !route_model_attested {
        outcome = "reverted".to_string();
    } else if !dod_passed || !success_criteria_met {
        outcome = "no_change".to_string();
    }

    let primary_failure = if let Some(first_failed) = failed.first() {
        if first_failed == "success_criteria_met"
            && input
                .success_criteria_primary_failure
                .as_ref()
                .map(|s| !s.trim().is_empty())
                .unwrap_or(false)
        {
            input.success_criteria_primary_failure.clone()
        } else {
            Some(first_failed.clone())
        }
    } else {
        None
    };

    ReceiptVerdictOutput {
        exec_check_name,
        checks,
        failed,
        passed,
        outcome,
        primary_failure,
        route_attestation_mismatch,
    }
}

pub fn compute_default_backlog_autoscale_state(
    input: &DefaultBacklogAutoscaleStateInput,
) -> DefaultBacklogAutoscaleStateOutput {
    let module = {
        let normalized = input.module.trim();
        if normalized.is_empty() {
            "autonomy_backlog_autoscale".to_string()
        } else {
            normalized.to_string()
        }
    };
    DefaultBacklogAutoscaleStateOutput {
        schema_id: "autonomy_backlog_autoscale".to_string(),
        schema_version: "1.0.0".to_string(),
        module,
        current_cells: 0.0,
        target_cells: 0.0,
        last_run_ts: None,
        last_high_pressure_ts: None,
        last_action: None,
        updated_at: None,
    }
}

fn parse_non_negative_number(value: Option<&serde_json::Value>) -> Option<f64> {
    let parsed = match value {
        Some(v) => {
            if let Some(n) = v.as_f64() {
                Some(n)
            } else if let Some(s) = v.as_str() {
                s.trim().parse::<f64>().ok()
            } else {
                None
            }
        }
        None => None,
    }?;
    if !parsed.is_finite() {
        return None;
    }
    Some(parsed.max(0.0))
}

fn parse_clean_optional_string(value: Option<&serde_json::Value>) -> Option<String> {
    let raw = value.and_then(|v| v.as_str())?;
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

pub fn compute_normalize_backlog_autoscale_state(
    input: &NormalizeBacklogAutoscaleStateInput,
) -> NormalizeBacklogAutoscaleStateOutput {
    let module_fallback = {
        let normalized = input.module.trim();
        if normalized.is_empty() {
            "autonomy_backlog_autoscale".to_string()
        } else {
            normalized.to_string()
        }
    };
    let src_obj = input.src.as_ref().and_then(|value| value.as_object());
    let module = src_obj
        .and_then(|obj| obj.get("module"))
        .and_then(|value| value.as_str())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| module_fallback.clone());
    let current_cells =
        parse_non_negative_number(src_obj.and_then(|obj| obj.get("current_cells"))).unwrap_or(0.0);
    let target_cells =
        parse_non_negative_number(src_obj.and_then(|obj| obj.get("target_cells"))).unwrap_or(0.0);
    let last_run_ts = parse_clean_optional_string(src_obj.and_then(|obj| obj.get("last_run_ts")));
    let last_high_pressure_ts =
        parse_clean_optional_string(src_obj.and_then(|obj| obj.get("last_high_pressure_ts")));
    let last_action = parse_clean_optional_string(src_obj.and_then(|obj| obj.get("last_action")));
    let updated_at = parse_clean_optional_string(src_obj.and_then(|obj| obj.get("updated_at")));
    NormalizeBacklogAutoscaleStateOutput {
        schema_id: "autonomy_backlog_autoscale".to_string(),
        schema_version: "1.0.0".to_string(),
        module,
        current_cells,
        target_cells,
        last_run_ts,
        last_high_pressure_ts,
        last_action,
        updated_at,
    }
}

pub fn compute_spawn_allocated_cells(
    input: &SpawnAllocatedCellsInput,
) -> SpawnAllocatedCellsOutput {
    let resolved = input
        .active_cells
        .or(input.current_cells)
        .or(input.allocated_cells)
        .filter(|value| value.is_finite())
        .map(|value| value.max(0.0).floor() as i64);
    SpawnAllocatedCellsOutput {
        active_cells: resolved,
    }
}

pub fn compute_spawn_capacity_boost_snapshot(
    input: &SpawnCapacityBoostSnapshotInput,
) -> SpawnCapacityBoostSnapshotOutput {
    let base = SpawnCapacityBoostSnapshotOutput {
        enabled: input.enabled,
        active: false,
        lookback_minutes: input.lookback_minutes.max(0.0),
        min_granted_cells: input.min_granted_cells.max(0.0),
        grant_count: 0,
        granted_cells: 0.0,
        latest_ts: None,
    };
    if !input.enabled {
        return base;
    }
    if input.rows.is_empty() {
        return base;
    }
    let now_ms = if input.now_ms.is_finite() {
        input.now_ms
    } else {
        Utc::now().timestamp_millis() as f64
    };
    let cutoff_ms = now_ms - (base.lookback_minutes * 60000.0);
    let mut grant_count: i64 = 0;
    let mut granted_cells: f64 = 0.0;
    let mut latest_ts: Option<String> = None;

    for row in input.rows.iter().rev() {
        let row_type = row
            .r#type
            .as_deref()
            .unwrap_or("")
            .trim()
            .to_ascii_lowercase();
        if row_type != "spawn_request" {
            continue;
        }
        let Some(ts_raw) = row.ts.as_deref() else {
            continue;
        };
        let Some(ts_ms) = parse_rfc3339_ts_ms(ts_raw.trim()) else {
            continue;
        };
        if (ts_ms as f64) < cutoff_ms {
            break;
        }
        let granted = row.granted_cells.unwrap_or(0.0);
        if !granted.is_finite() || granted < base.min_granted_cells {
            continue;
        }
        grant_count += 1;
        granted_cells += granted;
        if latest_ts.is_none() {
            latest_ts = Some(ts_raw.trim().to_string());
        }
    }
    SpawnCapacityBoostSnapshotOutput {
        enabled: base.enabled,
        active: grant_count > 0,
        lookback_minutes: base.lookback_minutes,
        min_granted_cells: base.min_granted_cells,
        grant_count,
        granted_cells: (granted_cells * 1000.0).round() / 1000.0,
        latest_ts,
    }
}
