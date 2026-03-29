fn evaluate_productivity(policy: &Value, paths: &TritShadowPaths) -> Value {
    let activation = policy
        .pointer("/influence/activation")
        .cloned()
        .unwrap_or_else(|| json!({}));
    if activation.get("enabled").and_then(Value::as_bool) != Some(true) {
        return json!({
            "enabled": false,
            "active": true,
            "reason": "activation_gate_disabled",
            "report_rows_evaluated": 0,
            "calibration_rows_evaluated": 0,
            "source_reliability": Value::Null,
        });
    }
    let reports = sorted_shadow_reports(&paths.report_history);
    let report_window = clamp_int(activation.get("report_window"), 1, 365, 1) as usize;
    let recent_reports = reports
        .iter()
        .rev()
        .take(report_window)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    let reports_pass = recent_reports.len() >= report_window
        && recent_reports
            .iter()
            .all(|row| report_passes_auto_stage(row, &activation));
    if !reports_pass {
        return json!({
            "enabled": true,
            "active": false,
            "reason": "activation_report_threshold_not_met",
            "report_rows_evaluated": recent_reports.len(),
            "calibration_rows_evaluated": 0,
            "source_reliability": Value::Null,
        });
    }
    let calibrations = sorted_calibration_rows(&paths.calibration_history);
    let calibration_window = clamp_int(activation.get("calibration_window"), 1, 365, 1);
    let calibration_check =
        calibration_window_passes_auto_stage(&calibrations, &activation, calibration_window);
    if calibration_check.get("pass").and_then(Value::as_bool) != Some(true) {
        return json!({
            "enabled": true,
            "active": false,
            "reason": "activation_calibration_threshold_not_met",
            "report_rows_evaluated": recent_reports.len(),
            "calibration_rows_evaluated": calibration_check.get("rows_evaluated").and_then(Value::as_u64).unwrap_or(0),
            "source_reliability": Value::Null,
        });
    }
    let source_reliability = source_reliability_gate(
        calibration_check
            .get("recent")
            .and_then(Value::as_array)
            .unwrap_or(&Vec::new()),
        &activation,
    );
    if source_reliability.get("pass").and_then(Value::as_bool) != Some(true) {
        return json!({
            "enabled": true,
            "active": false,
            "reason": "activation_source_reliability_not_met",
            "report_rows_evaluated": recent_reports.len(),
            "calibration_rows_evaluated": calibration_check.get("rows_evaluated").and_then(Value::as_u64).unwrap_or(0),
            "source_reliability": source_reliability,
        });
    }
    json!({
        "enabled": true,
        "active": true,
        "reason": "activation_threshold_met",
        "report_rows_evaluated": recent_reports.len(),
        "calibration_rows_evaluated": calibration_check.get("rows_evaluated").and_then(Value::as_u64).unwrap_or(0),
        "source_reliability": source_reliability,
    })
}

fn evaluate_auto_stage(policy: &Value, paths: &TritShadowPaths) -> Value {
    let auto_cfg = policy
        .pointer("/influence/auto_stage")
        .cloned()
        .unwrap_or_else(|| json!({}));
    if auto_cfg.get("enabled").and_then(Value::as_bool) != Some(true) {
        return json!({
            "enabled": false,
            "stage": 0,
            "reason": "auto_stage_disabled",
            "report_rows_evaluated": 0,
        });
    }
    let productivity = evaluate_productivity(policy, paths);
    if productivity.get("enabled").and_then(Value::as_bool) == Some(true)
        && productivity.get("active").and_then(Value::as_bool) != Some(true)
    {
        return json!({
            "enabled": true,
            "stage": 0,
            "reason": "productivity_threshold_not_met",
            "report_rows_evaluated": productivity.get("report_rows_evaluated").cloned().unwrap_or(Value::from(0)),
            "calibration_rows_evaluated": productivity.get("calibration_rows_evaluated").cloned().unwrap_or(Value::from(0)),
            "productivity": productivity,
        });
    }
    let reports = sorted_shadow_reports(&paths.report_history);
    let calibrations = sorted_calibration_rows(&paths.calibration_history);
    let calibration = latest_calibration(&paths.calibration_history);
    let stage3_cfg = auto_cfg.get("stage3").cloned().unwrap_or_else(|| json!({}));
    let stage2_cfg = auto_cfg.get("stage2").cloned().unwrap_or_else(|| json!({}));
    let stage3_window = clamp_int(stage3_cfg.get("consecutive_reports"), 1, 365, 6) as usize;
    let stage2_window = clamp_int(stage2_cfg.get("consecutive_reports"), 1, 365, 3) as usize;
    let stage3_cal_window = clamp_int(stage3_cfg.get("min_calibration_reports"), 1, 365, 1);
    let stage2_cal_window = clamp_int(stage2_cfg.get("min_calibration_reports"), 1, 365, 1);
    let recent3 = reports
        .iter()
        .rev()
        .take(stage3_window)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    let recent2 = reports
        .iter()
        .rev()
        .take(stage2_window)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    let stage3_reports_pass = recent3.len() >= stage3_window
        && recent3
            .iter()
            .all(|row| report_passes_auto_stage(row, &stage3_cfg));
    let stage2_reports_pass = recent2.len() >= stage2_window
        && recent2
            .iter()
            .all(|row| report_passes_auto_stage(row, &stage2_cfg));
    let stage3_cal_check =
        calibration_window_passes_auto_stage(&calibrations, &stage3_cfg, stage3_cal_window);
    let stage2_cal_check =
        calibration_window_passes_auto_stage(&calibrations, &stage2_cfg, stage2_cal_window);
    let activation_cfg = policy
        .pointer("/influence/activation")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let stage3_source = if as_bool(stage3_cfg.get("require_source_reliability"), false) {
        source_reliability_gate(
            stage3_cal_check
                .get("recent")
                .and_then(Value::as_array)
                .unwrap_or(&Vec::new()),
            &activation_cfg,
        )
    } else {
        json!({"pass": true})
    };
    let stage2_source = if as_bool(stage2_cfg.get("require_source_reliability"), false) {
        source_reliability_gate(
            stage2_cal_check
                .get("recent")
                .and_then(Value::as_array)
                .unwrap_or(&Vec::new()),
            &activation_cfg,
        )
    } else {
        json!({"pass": true})
    };
    let stage3_cal_pass = stage3_cal_check.get("pass").and_then(Value::as_bool) == Some(true)
        && stage3_source.get("pass").and_then(Value::as_bool) == Some(true);
    let stage2_cal_pass = stage2_cal_check.get("pass").and_then(Value::as_bool) == Some(true)
        && stage2_source.get("pass").and_then(Value::as_bool) == Some(true);
    if stage3_reports_pass && stage3_cal_pass {
        return json!({
            "enabled": true,
            "stage": 3,
            "reason": "auto_stage3_threshold_met",
            "report_rows_evaluated": recent3.len(),
            "calibration_rows_evaluated": stage3_cal_check.get("rows_evaluated").cloned().unwrap_or(Value::from(0)),
            "calibration_date": calibration.as_ref().and_then(|v| v.get("date")).cloned().unwrap_or(Value::Null),
            "productivity": productivity,
        });
    }
    if stage2_reports_pass && stage2_cal_pass {
        return json!({
            "enabled": true,
            "stage": 2,
            "reason": "auto_stage2_threshold_met",
            "report_rows_evaluated": recent2.len(),
            "calibration_rows_evaluated": stage2_cal_check.get("rows_evaluated").cloned().unwrap_or(Value::from(0)),
            "calibration_date": calibration.as_ref().and_then(|v| v.get("date")).cloned().unwrap_or(Value::Null),
            "productivity": productivity,
        });
    }
    json!({
        "enabled": true,
        "stage": 0,
        "reason": "auto_thresholds_not_met",
        "report_rows_evaluated": reports.len(),
        "calibration_rows_evaluated": calibrations.len(),
        "calibration_date": calibration.as_ref().and_then(|v| v.get("date")).cloned().unwrap_or(Value::Null),
        "productivity": productivity,
    })
}

fn resolve_stage_decision(policy: &Value, paths: &TritShadowPaths) -> Value {
    let base_stage = clamp_int(policy.pointer("/influence/stage"), 0, 3, 0);
    if let Ok(raw_env) = std::env::var("AUTONOMY_TRIT_SHADOW_STAGE") {
        let env = raw_env.trim();
        if !env.is_empty() {
            if let Ok(n) = env.parse::<f64>() {
                return json!({
                    "stage": clamp_int(Some(&Value::from(n)), 0, 3, 0),
                    "source": "env_numeric",
                    "base_stage": base_stage,
                    "auto_stage": Value::Null,
                });
            }
            let label_stage = match env.to_ascii_lowercase().as_str() {
                "shadow_only" => Some(0),
                "advisory" => Some(1),
                "influence_limited" => Some(2),
                "influence_budgeted" => Some(3),
                _ => None,
            };
            if let Some(stage) = label_stage {
                return json!({
                    "stage": stage,
                    "source": "env_label",
                    "base_stage": base_stage,
                    "auto_stage": Value::Null,
                });
            }
        }
    }
    let auto = evaluate_auto_stage(policy, paths);
    if auto.get("enabled").and_then(Value::as_bool) == Some(true) {
        let mode = if policy
            .pointer("/influence/auto_stage/mode")
            .and_then(Value::as_str)
            == Some("override")
        {
            "override"
        } else {
            "floor"
        };
        let auto_stage = clamp_int(auto.get("stage"), 0, 3, 0);
        let stage = if mode == "override" {
            auto_stage
        } else {
            base_stage.max(auto_stage)
        };
        return json!({
            "stage": stage,
            "source": format!("auto_{mode}"),
            "base_stage": base_stage,
            "auto_stage": auto,
        });
    }
    json!({
        "stage": base_stage,
        "source": "policy",
        "base_stage": base_stage,
        "auto_stage": auto,
    })
}

fn default_influence_budget() -> Value {
    json!({
        "schema_id": "trit_shadow_influence_budget",
        "schema_version": "1.0.0",
        "by_date": {},
        "updated_at": Value::Null,
    })
}

fn load_influence_budget(path: &Path) -> Value {
    let raw = read_json(path);
    let mut by_date = serde_json::Map::new();
    if let Some(rows) = raw.get("by_date").and_then(Value::as_object) {
        for (date, row) in rows {
            let rec = row.as_object();
            by_date.insert(date.clone(), json!({
                "overrides": clamp_int(rec.and_then(|v| v.get("overrides")), 0, 1_000_000, 0),
                "by_source": rec.and_then(|v| v.get("by_source")).cloned().unwrap_or_else(|| json!({}))
            }));
        }
    }
    json!({
        "schema_id": raw.get("schema_id").cloned().unwrap_or_else(|| Value::String("trit_shadow_influence_budget".to_string())),
        "schema_version": raw.get("schema_version").cloned().unwrap_or_else(|| Value::String("1.0.0".to_string())),
        "by_date": by_date,
        "updated_at": raw.get("updated_at").cloned().unwrap_or(Value::Null),
    })
}

fn save_influence_budget(budget: &Value, path: &Path) -> Result<Value, String> {
    let mut next = if budget.is_object() {
        budget.clone()
    } else {
        default_influence_budget()
    };
    next["updated_at"] = Value::String(now_iso());
    write_json_atomic(path, &next)?;
    Ok(next)
}

fn can_consume_override(policy: &Value, date_str: &str, path: &Path) -> Value {
    let max_per_day = clamp_int(
        policy.pointer("/influence/max_overrides_per_day"),
        0,
        10_000,
        0,
    );
    if max_per_day <= 0 {
        return json!({"allowed": false, "reason": "budget_disabled", "remaining": 0});
    }
    let budget = load_influence_budget(path);
    let row = budget
        .pointer(&format!("/by_date/{date_str}"))
        .cloned()
        .unwrap_or_else(|| json!({"overrides": 0}));
    let used = clamp_int(row.get("overrides"), 0, 1_000_000, 0);
    let remaining = (max_per_day - used).max(0);
    if remaining <= 0 {
        return json!({
            "allowed": false,
            "reason": "daily_override_budget_exhausted",
            "remaining": 0,
            "used": used,
            "max_per_day": max_per_day,
        });
    }
    json!({
        "allowed": true,
        "reason": "ok",
        "remaining": remaining,
        "used": used,
        "max_per_day": max_per_day,
    })
}

fn consume_override(
    source: &str,
    policy: &Value,
    date_str: &str,
    path: &Path,
) -> Result<Value, String> {
    let check = can_consume_override(policy, date_str, path);
    if check.get("allowed").and_then(Value::as_bool) != Some(true) {
        return Ok(json!({
            "consumed": false,
            "allowed": false,
            "reason": check.get("reason").cloned().unwrap_or_else(|| Value::String("blocked".to_string())),
            "remaining": check.get("remaining").cloned().unwrap_or(Value::from(0)),
            "used": check.get("used").cloned().unwrap_or(Value::from(0)),
            "max_per_day": check.get("max_per_day").cloned().unwrap_or(Value::from(0)),
        }));
    }
    let mut budget = load_influence_budget(path);
    if !budget.get("by_date").map(Value::is_object).unwrap_or(false) {
        budget["by_date"] = json!({});
    }
    if budget.pointer(&format!("/by_date/{date_str}")).is_none() {
        budget["by_date"][date_str] = json!({"overrides": 0, "by_source": {}});
    }
    let used = clamp_int(
        budget.pointer(&format!("/by_date/{date_str}/overrides")),
        0,
        1_000_000,
        0,
    ) + 1;
    budget["by_date"][date_str]["overrides"] = Value::from(used);
    if !budget
        .pointer(&format!("/by_date/{date_str}/by_source"))
        .map(Value::is_object)
        .unwrap_or(false)
    {
        budget["by_date"][date_str]["by_source"] = json!({});
    }
    let source_key = if source.trim().is_empty() {
        "unknown"
    } else {
        source.trim()
    };
    let by_source_used = clamp_int(
        budget.pointer(&format!("/by_date/{date_str}/by_source/{source_key}")),
        0,
        1_000_000,
        0,
    ) + 1;
    budget["by_date"][date_str]["by_source"][source_key] = Value::from(by_source_used);
    let saved = save_influence_budget(&budget, path)?;
    Ok(json!({
        "consumed": true,
        "allowed": true,
        "reason": "ok",
        "remaining": clamp_int(check.get("remaining"), 0, 10_000, 0).saturating_sub(1),
        "used": used,
        "max_per_day": check.get("max_per_day").cloned().unwrap_or(Value::from(0)),
        "budget": saved,
    }))
}

fn default_influence_guard() -> Value {
    json!({
        "schema_id": "trit_shadow_influence_guard",
        "schema_version": "1.0.0",
        "disabled": false,
        "reason": Value::Null,
        "disabled_until": Value::Null,
        "last_report_ts": Value::Null,
        "updated_at": Value::Null,
    })
}

fn load_influence_guard(path: &Path) -> Value {
    let raw = read_json(path);
    let reason = {
        let s = as_str(raw.get("reason"));
        if s.is_empty() {
            Value::Null
        } else {
            Value::String(s)
        }
    };
    let disabled_until = {
        let s = as_str(raw.get("disabled_until"));
        if s.is_empty() {
            Value::Null
        } else {
            Value::String(s)
        }
    };
    let last_report_ts = {
        let s = as_str(raw.get("last_report_ts"));
        if s.is_empty() {
            Value::Null
        } else {
            Value::String(s)
        }
    };
    json!({
        "schema_id": raw.get("schema_id").cloned().unwrap_or_else(|| Value::String("trit_shadow_influence_guard".to_string())),
        "schema_version": raw.get("schema_version").cloned().unwrap_or_else(|| Value::String("1.0.0".to_string())),
        "disabled": raw.get("disabled").and_then(Value::as_bool).unwrap_or(false),
        "reason": reason,
        "disabled_until": disabled_until,
        "last_report_ts": last_report_ts,
        "updated_at": raw.get("updated_at").cloned().unwrap_or(Value::Null),
    })
}

