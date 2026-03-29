fn normalize_policy(input: &Map<String, Value>) -> Value {
    let base = default_policy();
    let base_semantics = as_object(base.get("semantics")).unwrap();
    let base_trust = as_object(base.get("trust")).unwrap();
    let base_influence = as_object(base.get("influence")).unwrap();
    let base_activation = as_object(base_influence.get("activation")).unwrap();
    let base_auto_stage = as_object(base_influence.get("auto_stage")).unwrap();
    let base_stage2 = as_object(base_auto_stage.get("stage2")).unwrap();
    let base_stage3 = as_object(base_auto_stage.get("stage3")).unwrap();
    let base_adaptation = as_object(base.get("adaptation")).unwrap();

    let semantics = as_object(input.get("semantics"));
    let trust = as_object(input.get("trust"));
    let influence = as_object(input.get("influence"));
    let activation = influence.and_then(|v| as_object(v.get("activation")));
    let auto_stage = influence.and_then(|v| as_object(v.get("auto_stage")));
    let stage2 = auto_stage.and_then(|v| as_object(v.get("stage2")));
    let stage3 = auto_stage.and_then(|v| as_object(v.get("stage3")));
    let adaptation = as_object(input.get("adaptation"));

    let trust_floor = clamp_number(
        trust.and_then(|v| v.get("source_trust_floor")),
        0.01,
        5.0,
        base_trust
            .get("source_trust_floor")
            .and_then(Value::as_f64)
            .unwrap_or(0.6),
    );
    let version = as_str(input.get("version"))
        .chars()
        .take(32)
        .collect::<String>()
        .if_empty_then("1.0");
    let auto_stage_mode = {
        let raw = as_str(auto_stage.and_then(|v| v.get("mode")));
        if raw.eq_ignore_ascii_case("override") {
            "override"
        } else {
            "floor"
        }
    };

    json!({
        "version": version,
        "enabled": input.get("enabled").map(Value::as_bool).flatten().unwrap_or(true),
        "semantics": {
            "locked": semantics.and_then(|v| v.get("locked")).map(Value::as_bool).flatten().unwrap_or(true),
            "neutral_on_missing": semantics.and_then(|v| v.get("neutral_on_missing")).map(Value::as_bool).flatten().unwrap_or(true),
            "min_non_neutral_signals": clamp_int(semantics.and_then(|v| v.get("min_non_neutral_signals")), 0, 1000, base_semantics.get("min_non_neutral_signals").and_then(Value::as_i64).unwrap_or(1)),
            "min_non_neutral_weight": clamp_number(semantics.and_then(|v| v.get("min_non_neutral_weight")), 0.0, 1000.0, base_semantics.get("min_non_neutral_weight").and_then(Value::as_f64).unwrap_or(0.9)),
            "min_confidence_for_non_neutral": clamp_number(semantics.and_then(|v| v.get("min_confidence_for_non_neutral")), 0.0, 1.0, base_semantics.get("min_confidence_for_non_neutral").and_then(Value::as_f64).unwrap_or(0.3))
        },
        "trust": {
            "enabled": trust.and_then(|v| v.get("enabled")).map(Value::as_bool).flatten().unwrap_or(true),
            "default_source_trust": clamp_number(trust.and_then(|v| v.get("default_source_trust")), 0.01, 5.0, base_trust.get("default_source_trust").and_then(Value::as_f64).unwrap_or(1.0)),
            "source_trust_floor": trust_floor,
            "source_trust_ceiling": clamp_number(trust.and_then(|v| v.get("source_trust_ceiling")), trust_floor, 5.0, base_trust.get("source_trust_ceiling").and_then(Value::as_f64).unwrap_or(1.5)),
            "freshness_half_life_hours": clamp_number(trust.and_then(|v| v.get("freshness_half_life_hours")), 1.0, (24 * 365) as f64, base_trust.get("freshness_half_life_hours").and_then(Value::as_f64).unwrap_or(72.0))
        },
        "influence": {
            "stage": clamp_int(influence.and_then(|v| v.get("stage")), 0, 3, base_influence.get("stage").and_then(Value::as_i64).unwrap_or(0)),
            "min_confidence_stage2": clamp_number(influence.and_then(|v| v.get("min_confidence_stage2")), 0.0, 1.0, base_influence.get("min_confidence_stage2").and_then(Value::as_f64).unwrap_or(0.78)),
            "min_confidence_stage3": clamp_number(influence.and_then(|v| v.get("min_confidence_stage3")), 0.0, 1.0, base_influence.get("min_confidence_stage3").and_then(Value::as_f64).unwrap_or(0.85)),
            "max_overrides_per_day": clamp_int(influence.and_then(|v| v.get("max_overrides_per_day")), 0, 10000, base_influence.get("max_overrides_per_day").and_then(Value::as_i64).unwrap_or(3)),
            "auto_disable_hours_on_regression": clamp_number(influence.and_then(|v| v.get("auto_disable_hours_on_regression")), 1.0, (24 * 30) as f64, base_influence.get("auto_disable_hours_on_regression").and_then(Value::as_f64).unwrap_or(24.0)),
            "activation": {
                "enabled": activation.and_then(|v| v.get("enabled")).map(Value::as_bool).flatten().unwrap_or(false),
                "report_window": clamp_int(activation.and_then(|v| v.get("report_window")), 1, 365, base_activation.get("report_window").and_then(Value::as_i64).unwrap_or(4)),
                "min_decisions": clamp_int(activation.and_then(|v| v.get("min_decisions")), 1, 1_000_000, base_activation.get("min_decisions").and_then(Value::as_i64).unwrap_or(20)),
                "max_divergence_rate": clamp_number(activation.and_then(|v| v.get("max_divergence_rate")), 0.0, 1.0, base_activation.get("max_divergence_rate").and_then(Value::as_f64).unwrap_or(0.08)),
                "require_success_criteria_pass": activation.and_then(|v| v.get("require_success_criteria_pass")).map(Value::as_bool).flatten().unwrap_or(true),
                "require_safety_pass": activation.and_then(|v| v.get("require_safety_pass")).map(Value::as_bool).flatten().unwrap_or(true),
                "require_drift_non_increasing": activation.and_then(|v| v.get("require_drift_non_increasing")).map(Value::as_bool).flatten().unwrap_or(true),
                "calibration_window": clamp_int(activation.and_then(|v| v.get("calibration_window")), 1, 365, base_activation.get("calibration_window").and_then(Value::as_i64).unwrap_or(3)),
                "min_calibration_events": clamp_int(activation.and_then(|v| v.get("min_calibration_events")), 0, 1_000_000, base_activation.get("min_calibration_events").and_then(Value::as_i64).unwrap_or(20)),
                "min_calibration_accuracy": clamp_number(activation.and_then(|v| v.get("min_calibration_accuracy")), 0.0, 1.0, base_activation.get("min_calibration_accuracy").and_then(Value::as_f64).unwrap_or(0.58)),
                "max_calibration_ece": clamp_number(activation.and_then(|v| v.get("max_calibration_ece")), 0.0, 1.0, base_activation.get("max_calibration_ece").and_then(Value::as_f64).unwrap_or(0.23)),
                "min_source_samples": clamp_int(activation.and_then(|v| v.get("min_source_samples")), 1, 1_000_000, base_activation.get("min_source_samples").and_then(Value::as_i64).unwrap_or(8)),
                "min_source_hit_rate": clamp_number(activation.and_then(|v| v.get("min_source_hit_rate")), 0.0, 1.0, base_activation.get("min_source_hit_rate").and_then(Value::as_f64).unwrap_or(0.55)),
                "max_sources_below_threshold": clamp_int(activation.and_then(|v| v.get("max_sources_below_threshold")), 0, 1_000_000, base_activation.get("max_sources_below_threshold").and_then(Value::as_i64).unwrap_or(1)),
                "allow_if_no_source_data": activation.and_then(|v| v.get("allow_if_no_source_data")).map(Value::as_bool).flatten().unwrap_or(false)
            },
            "auto_stage": {
                "enabled": auto_stage.and_then(|v| v.get("enabled")).map(Value::as_bool).flatten().unwrap_or(false),
                "mode": auto_stage_mode,
                "stage2": {
                    "consecutive_reports": clamp_int(stage2.and_then(|v| v.get("consecutive_reports")), 1, 365, base_stage2.get("consecutive_reports").and_then(Value::as_i64).unwrap_or(3)),
                    "min_calibration_reports": clamp_int(stage2.and_then(|v| v.get("min_calibration_reports")), 1, 365, base_stage2.get("min_calibration_reports").and_then(Value::as_i64).unwrap_or(1)),
                    "min_decisions": clamp_int(stage2.and_then(|v| v.get("min_decisions")), 1, 1_000_000, base_stage2.get("min_decisions").and_then(Value::as_i64).unwrap_or(20)),
                    "max_divergence_rate": clamp_number(stage2.and_then(|v| v.get("max_divergence_rate")), 0.0, 1.0, base_stage2.get("max_divergence_rate").and_then(Value::as_f64).unwrap_or(0.08)),
                    "min_calibration_events": clamp_int(stage2.and_then(|v| v.get("min_calibration_events")), 0, 1_000_000, base_stage2.get("min_calibration_events").and_then(Value::as_i64).unwrap_or(20)),
                    "min_calibration_accuracy": clamp_number(stage2.and_then(|v| v.get("min_calibration_accuracy")), 0.0, 1.0, base_stage2.get("min_calibration_accuracy").and_then(Value::as_f64).unwrap_or(0.55)),
                    "max_calibration_ece": clamp_number(stage2.and_then(|v| v.get("max_calibration_ece")), 0.0, 1.0, base_stage2.get("max_calibration_ece").and_then(Value::as_f64).unwrap_or(0.25)),
                    "require_success_criteria_pass": stage2.and_then(|v| v.get("require_success_criteria_pass")).map(Value::as_bool).flatten().unwrap_or(false),
                    "require_safety_pass": stage2.and_then(|v| v.get("require_safety_pass")).map(Value::as_bool).flatten().unwrap_or(true),
                    "require_drift_non_increasing": stage2.and_then(|v| v.get("require_drift_non_increasing")).map(Value::as_bool).flatten().unwrap_or(true),
                    "require_source_reliability": stage2.and_then(|v| v.get("require_source_reliability")).map(Value::as_bool).flatten().unwrap_or(false)
                },
                "stage3": {
                    "consecutive_reports": clamp_int(stage3.and_then(|v| v.get("consecutive_reports")), 1, 365, base_stage3.get("consecutive_reports").and_then(Value::as_i64).unwrap_or(6)),
                    "min_calibration_reports": clamp_int(stage3.and_then(|v| v.get("min_calibration_reports")), 1, 365, base_stage3.get("min_calibration_reports").and_then(Value::as_i64).unwrap_or(1)),
                    "min_decisions": clamp_int(stage3.and_then(|v| v.get("min_decisions")), 1, 1_000_000, base_stage3.get("min_decisions").and_then(Value::as_i64).unwrap_or(40)),
                    "max_divergence_rate": clamp_number(stage3.and_then(|v| v.get("max_divergence_rate")), 0.0, 1.0, base_stage3.get("max_divergence_rate").and_then(Value::as_f64).unwrap_or(0.05)),
                    "min_calibration_events": clamp_int(stage3.and_then(|v| v.get("min_calibration_events")), 0, 1_000_000, base_stage3.get("min_calibration_events").and_then(Value::as_i64).unwrap_or(40)),
                    "min_calibration_accuracy": clamp_number(stage3.and_then(|v| v.get("min_calibration_accuracy")), 0.0, 1.0, base_stage3.get("min_calibration_accuracy").and_then(Value::as_f64).unwrap_or(0.65)),
                    "max_calibration_ece": clamp_number(stage3.and_then(|v| v.get("max_calibration_ece")), 0.0, 1.0, base_stage3.get("max_calibration_ece").and_then(Value::as_f64).unwrap_or(0.2)),
                    "require_success_criteria_pass": stage3.and_then(|v| v.get("require_success_criteria_pass")).map(Value::as_bool).flatten().unwrap_or(true),
                    "require_safety_pass": stage3.and_then(|v| v.get("require_safety_pass")).map(Value::as_bool).flatten().unwrap_or(true),
                    "require_drift_non_increasing": stage3.and_then(|v| v.get("require_drift_non_increasing")).map(Value::as_bool).flatten().unwrap_or(true),
                    "require_source_reliability": stage3.and_then(|v| v.get("require_source_reliability")).map(Value::as_bool).flatten().unwrap_or(false)
                }
            }
        },
        "adaptation": {
            "enabled": adaptation.and_then(|v| v.get("enabled")).map(Value::as_bool).flatten().unwrap_or(true),
            "cadence_days": clamp_int(adaptation.and_then(|v| v.get("cadence_days")), 1, 60, base_adaptation.get("cadence_days").and_then(Value::as_i64).unwrap_or(7)),
            "min_samples_per_source": clamp_int(adaptation.and_then(|v| v.get("min_samples_per_source")), 1, 10_000, base_adaptation.get("min_samples_per_source").and_then(Value::as_i64).unwrap_or(6)),
            "reward_step": clamp_number(adaptation.and_then(|v| v.get("reward_step")), 0.0, 1.0, base_adaptation.get("reward_step").and_then(Value::as_f64).unwrap_or(0.04)),
            "penalty_step": clamp_number(adaptation.and_then(|v| v.get("penalty_step")), 0.0, 1.0, base_adaptation.get("penalty_step").and_then(Value::as_f64).unwrap_or(0.06)),
            "max_delta_per_cycle": clamp_number(adaptation.and_then(|v| v.get("max_delta_per_cycle")), 0.0, 1.0, base_adaptation.get("max_delta_per_cycle").and_then(Value::as_f64).unwrap_or(0.08))
        }
    })
}

trait StringExt {
    fn if_empty_then(self, fallback: &str) -> String;
}

impl StringExt for String {
    fn if_empty_then(self, fallback: &str) -> String {
        if self.trim().is_empty() {
            fallback.to_string()
        } else {
            self
        }
    }
}

fn load_policy_from_path(path: &Path) -> Value {
    let raw = read_json(path);
    let obj = payload_obj(&raw).clone();
    normalize_policy(&obj)
}

fn default_success_criteria() -> Value {
    json!({
        "version": "1.0",
        "targets": {
            "max_divergence_rate": 0.05,
            "min_decisions_for_divergence": 30,
            "max_safety_regressions": 0,
            "drift_non_increasing": true,
            "min_yield_lift": 0.03
        },
        "baseline": {
            "drift_rate": 0.03,
            "yield_rate": 0.714
        }
    })
}

fn load_success_criteria_from_path(path: &Path) -> Value {
    let raw = read_json(path);
    if raw.is_null() {
        default_success_criteria()
    } else {
        raw
    }
}

fn default_trust_state(policy: &Value) -> Value {
    json!({
        "schema_id": "trit_shadow_trust_state",
        "schema_version": "1.0.0",
        "updated_at": Value::Null,
        "default_source_trust": clamp_number(policy.pointer("/trust/default_source_trust"), 0.01, 5.0, 1.0),
        "by_source": {}
    })
}

fn normalize_trust_state(input: &Map<String, Value>, policy: &Value) -> Value {
    let base = default_trust_state(policy);
    let base_default = base
        .get("default_source_trust")
        .and_then(Value::as_f64)
        .unwrap_or(1.0);
    let floor = clamp_number(policy.pointer("/trust/source_trust_floor"), 0.01, 5.0, 0.6);
    let ceiling = clamp_number(
        policy.pointer("/trust/source_trust_ceiling"),
        floor,
        5.0,
        1.5,
    );
    let mut by_source = serde_json::Map::new();
    if let Some(source_map) = input.get("by_source").and_then(Value::as_object) {
        for (source, row) in source_map {
            let rec = row.as_object();
            by_source.insert(source.clone(), json!({
                "trust": clamp_number(rec.and_then(|v| v.get("trust")), floor, ceiling, base_default),
                "samples": clamp_int(rec.and_then(|v| v.get("samples")), 0, 1_000_000, 0),
                "hit_rate": clamp_number(rec.and_then(|v| v.get("hit_rate")), 0.0, 1.0, 0.0),
                "updated_at": rec.and_then(|v| v.get("updated_at")).map(|v| Value::String(as_str(Some(v)))).unwrap_or(Value::Null)
            }));
        }
    }
    json!({
        "schema_id": input.get("schema_id").cloned().unwrap_or_else(|| Value::String("trit_shadow_trust_state".to_string())),
        "schema_version": input.get("schema_version").cloned().unwrap_or_else(|| Value::String("1.0.0".to_string())),
        "updated_at": input.get("updated_at").cloned().unwrap_or(Value::Null),
        "default_source_trust": clamp_number(input.get("default_source_trust"), floor, ceiling, base_default),
        "by_source": by_source,
    })
}

fn load_trust_state_from_path(policy: &Value, path: &Path) -> Value {
    let raw = read_json(path);
    let obj = payload_obj(&raw).clone();
    normalize_trust_state(&obj, policy)
}

fn save_trust_state_to_path(state: &Value, policy: &Value, path: &Path) -> Result<Value, String> {
    let obj = payload_obj(state).clone();
    let mut normalized = normalize_trust_state(&obj, policy);
    normalized["updated_at"] = Value::String(now_iso());
    write_json_atomic(path, &normalized)?;
    Ok(normalized)
}

fn build_trust_map(trust_state: &Value) -> Value {
    let mut out = serde_json::Map::new();
    if let Some(by_source) = trust_state.get("by_source").and_then(Value::as_object) {
        for (source, row) in by_source {
            out.insert(
                source.clone(),
                Value::from(row.get("trust").and_then(Value::as_f64).unwrap_or(1.0)),
            );
        }
    }
    Value::Object(out)
}

fn sorted_shadow_reports(path: &Path) -> Vec<Value> {
    let mut rows = read_jsonl(path)
        .into_iter()
        .filter(|row| row.get("type").and_then(Value::as_str) == Some("trit_shadow_report"))
        .filter(|row| row.get("ok").and_then(Value::as_bool) == Some(true))
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| as_str(a.get("ts")).cmp(&as_str(b.get("ts"))));
    rows
}

fn sorted_calibration_rows(path: &Path) -> Vec<Value> {
    let mut rows = read_jsonl(path)
        .into_iter()
        .filter(|row| {
            row.get("type").and_then(Value::as_str) == Some("trit_shadow_replay_calibration")
        })
        .filter(|row| row.get("ok").and_then(Value::as_bool) == Some(true))
        .collect::<Vec<_>>();
    rows.sort_by(|a, b| as_str(a.get("ts")).cmp(&as_str(b.get("ts"))));
    rows
}

fn latest_calibration(path: &Path) -> Option<Value> {
    let rows = sorted_calibration_rows(path);
    rows.last().cloned()
}

fn report_passes_auto_stage(row: &Value, cfg: &Value) -> bool {
    let summary = row.get("summary").and_then(Value::as_object);
    let success = row.get("success_criteria").and_then(Value::as_object);
    let checks = success
        .and_then(|v| v.get("checks"))
        .and_then(Value::as_object);
    if clamp_number(
        summary.and_then(|v| v.get("total_decisions")),
        0.0,
        1_000_000.0,
        0.0,
    ) < clamp_number(cfg.get("min_decisions"), 0.0, 1_000_000.0, 0.0)
    {
        return false;
    }
    if clamp_number(
        summary.and_then(|v| v.get("divergence_rate")),
        0.0,
        1.0,
        0.0,
    ) > clamp_number(cfg.get("max_divergence_rate"), 0.0, 1.0, 1.0)
    {
        return false;
    }
    if as_bool(cfg.get("require_success_criteria_pass"), false)
        && success.and_then(|v| v.get("pass")).and_then(Value::as_bool) != Some(true)
    {
        return false;
    }
    if as_bool(cfg.get("require_safety_pass"), true) {
        let safety = checks
            .and_then(|v| v.get("safety_regressions"))
            .and_then(Value::as_object);
        if safety.and_then(|v| v.get("pass")).and_then(Value::as_bool) != Some(true) {
            return false;
        }
    }
    if as_bool(cfg.get("require_drift_non_increasing"), true) {
        let drift = checks
            .and_then(|v| v.get("drift_non_increasing"))
            .and_then(Value::as_object);
        if drift.and_then(|v| v.get("pass")).and_then(Value::as_bool) != Some(true) {
            return false;
        }
    }
    true
}

fn calibration_passes_auto_stage(calibration: &Value, cfg: &Value) -> bool {
    let summary = calibration.get("summary").and_then(Value::as_object);
    if clamp_number(
        summary.and_then(|v| v.get("total_events")),
        0.0,
        1_000_000.0,
        0.0,
    ) < clamp_number(cfg.get("min_calibration_events"), 0.0, 1_000_000.0, 0.0)
    {
        return false;
    }
    if clamp_number(summary.and_then(|v| v.get("accuracy")), 0.0, 1.0, 0.0)
        < clamp_number(cfg.get("min_calibration_accuracy"), 0.0, 1.0, 0.0)
    {
        return false;
    }
    if clamp_number(
        summary.and_then(|v| v.get("expected_calibration_error")),
        0.0,
        1.0,
        1.0,
    ) > clamp_number(cfg.get("max_calibration_ece"), 0.0, 1.0, 1.0)
    {
        return false;
    }
    true
}

fn calibration_window_passes_auto_stage(
    rows: &[Value],
    cfg: &Value,
    required_window: i64,
) -> Value {
    let window = required_window.max(1) as usize;
    let recent = rows
        .iter()
        .rev()
        .take(window)
        .cloned()
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .collect::<Vec<_>>();
    let pass = recent.len() >= window
        && recent
            .iter()
            .all(|row| calibration_passes_auto_stage(row, cfg));
    json!({
        "required_window": window,
        "rows_evaluated": recent.len(),
        "pass": pass,
        "recent": recent,
    })
}

fn source_reliability_gate(rows: &[Value], cfg: &Value) -> Value {
    let min_samples = clamp_int(cfg.get("min_source_samples"), 1, 1_000_000, 8);
    let min_hit_rate = clamp_number(cfg.get("min_source_hit_rate"), 0.0, 1.0, 0.55);
    let max_below = clamp_int(cfg.get("max_sources_below_threshold"), 0, 1_000_000, 0);
    let allow_if_no_source_data = as_bool(cfg.get("allow_if_no_source_data"), false);

    let mut totals: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    for row in rows {
        for source_row in as_array(row.get("source_reliability")) {
            let source = as_str(source_row.get("source"));
            if source.is_empty() {
                continue;
            }
            let samples = clamp_number(source_row.get("samples"), 0.0, 1_000_000.0, 0.0);
            let hit_rate_raw = as_f64(source_row.get("hit_rate"));
            let reliability_raw = as_f64(source_row.get("reliability"));
            let hit_rate = hit_rate_raw.or(reliability_raw).unwrap_or(f64::NAN);
            if !hit_rate.is_finite() {
                continue;
            }
            let entry = totals.entry(source).or_insert((0.0, 0.0));
            entry.0 += samples;
            entry.1 += samples * hit_rate;
        }
    }

    let mut aggregated = totals
        .into_iter()
        .map(|(source, (samples, weighted_hits))| {
            let hit_rate = if samples > 0.0 {
                weighted_hits / samples
            } else {
                0.0
            };
            json!({
                "source": source,
                "samples": samples as i64,
                "hit_rate": round_to(hit_rate, 4),
                "pass": samples >= min_samples as f64 && hit_rate >= min_hit_rate,
            })
        })
        .collect::<Vec<_>>();
    aggregated.sort_by(|a, b| {
        let b_samples = b.get("samples").and_then(Value::as_i64).unwrap_or(0);
        let a_samples = a.get("samples").and_then(Value::as_i64).unwrap_or(0);
        b_samples
            .cmp(&a_samples)
            .then_with(|| as_str(a.get("source")).cmp(&as_str(b.get("source"))))
    });
    let observed = aggregated
        .iter()
        .filter(|row| row.get("samples").and_then(Value::as_i64).unwrap_or(0) >= min_samples)
        .cloned()
        .collect::<Vec<_>>();
    let failing = observed
        .iter()
        .filter(|row| row.get("pass").and_then(Value::as_bool) != Some(true))
        .cloned()
        .collect::<Vec<_>>();
    let pass = if observed.is_empty() {
        allow_if_no_source_data
    } else {
        failing.len() as i64 <= max_below
    };
    json!({
        "pass": pass,
        "observed_count": observed.len(),
        "failing_count": failing.len(),
        "min_source_samples": min_samples,
        "min_source_hit_rate": min_hit_rate,
        "max_sources_below_threshold": max_below,
        "allow_if_no_source_data": allow_if_no_source_data,
        "top_observed": observed.into_iter().take(8).collect::<Vec<_>>(),
        "top_failing": failing.into_iter().take(8).collect::<Vec<_>>(),
    })
}

