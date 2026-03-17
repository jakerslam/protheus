// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use chrono::{Duration, Utc};
use serde_json::{json, Map, Value};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};

const POLICY_REL: &str = "client/config/trit_shadow_policy.json";
const SUCCESS_CRITERIA_REL: &str = "client/config/trit_shadow_success_criteria.json";
const TRUST_STATE_REL: &str = "client/local/state/autonomy/trit_shadow_trust_state.json";
const INFLUENCE_BUDGET_REL: &str = "client/local/state/autonomy/trit_shadow_influence_budget.json";
const INFLUENCE_GUARD_REL: &str = "client/local/state/autonomy/trit_shadow_influence_guard.json";
const REPORT_HISTORY_REL: &str = "client/local/state/autonomy/trit_shadow_reports/history.jsonl";
const CALIBRATION_HISTORY_REL: &str = "client/local/state/autonomy/trit_shadow_calibration/history.jsonl";

fn usage() {
    println!("trit-shadow-kernel commands:");
    println!("  protheus-ops trit-shadow-kernel paths [--payload-base64=<json>]");
    println!("  protheus-ops trit-shadow-kernel default-policy");
    println!("  protheus-ops trit-shadow-kernel normalize-policy --payload-base64=<json>");
    println!("  protheus-ops trit-shadow-kernel load-policy [--payload-base64=<json>]");
    println!("  protheus-ops trit-shadow-kernel load-success-criteria [--payload-base64=<json>]");
    println!("  protheus-ops trit-shadow-kernel load-trust-state [--payload-base64=<json>]");
    println!("  protheus-ops trit-shadow-kernel save-trust-state --payload-base64=<json>");
    println!("  protheus-ops trit-shadow-kernel build-trust-map --payload-base64=<json>");
    println!("  protheus-ops trit-shadow-kernel evaluate-productivity [--payload-base64=<json>]");
    println!("  protheus-ops trit-shadow-kernel evaluate-auto-stage [--payload-base64=<json>]");
    println!("  protheus-ops trit-shadow-kernel resolve-stage-decision [--payload-base64=<json>]");
    println!("  protheus-ops trit-shadow-kernel resolve-stage [--payload-base64=<json>]");
    println!("  protheus-ops trit-shadow-kernel can-consume-override [--payload-base64=<json>]");
    println!("  protheus-ops trit-shadow-kernel consume-override --payload-base64=<json>");
    println!("  protheus-ops trit-shadow-kernel load-influence-guard [--payload-base64=<json>]");
    println!("  protheus-ops trit-shadow-kernel save-influence-guard --payload-base64=<json>");
    println!("  protheus-ops trit-shadow-kernel influence-blocked [--payload-base64=<json>]");
    println!("  protheus-ops trit-shadow-kernel apply-influence-guard --payload-base64=<json>");
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    let ts = now_iso();
    let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
    let mut out = json!({
        "ok": ok,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "payload": payload,
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn cli_error(kind: &str, error: &str) -> Value {
    let ts = now_iso();
    let mut out = json!({
        "ok": false,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string(),
        "error": error,
        "fail_closed": true,
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("trit_shadow_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("trit_shadow_kernel_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("trit_shadow_kernel_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("trit_shadow_kernel_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn as_object<'a>(value: Option<&'a Value>) -> Option<&'a Map<String, Value>> {
    value.and_then(Value::as_object)
}

fn as_array<'a>(value: Option<&'a Value>) -> &'a Vec<Value> {
    value.and_then(Value::as_array).unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Vec<Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Vec::new)
    })
}

fn as_str(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(v)) => v.trim().to_string(),
        Some(Value::Null) | None => String::new(),
        Some(v) => v.to_string().trim_matches('"').trim().to_string(),
    }
}

fn as_bool(value: Option<&Value>, fallback: bool) -> bool {
    match value {
        Some(Value::Bool(v)) => *v,
        Some(Value::Number(n)) => n.as_i64().map(|v| v != 0).unwrap_or(fallback),
        Some(Value::String(v)) => lane_utils::parse_bool(Some(v.as_str()), fallback),
        _ => fallback,
    }
}

fn as_f64(value: Option<&Value>) -> Option<f64> {
    match value {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(v)) => v.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn clamp_number(value: Option<&Value>, lo: f64, hi: f64, fallback: f64) -> f64 {
    let raw = as_f64(value).unwrap_or(fallback);
    if !raw.is_finite() {
        return fallback;
    }
    raw.clamp(lo, hi)
}

fn clamp_int(value: Option<&Value>, lo: i64, hi: i64, fallback: i64) -> i64 {
    let raw = as_f64(value).unwrap_or(fallback as f64);
    if !raw.is_finite() {
        return fallback;
    }
    raw.floor().clamp(lo as f64, hi as f64) as i64
}

fn round_to(value: f64, digits: u32) -> f64 {
    let factor = 10_f64.powi(i32::try_from(digits).unwrap_or(4));
    (value * factor).round() / factor
}

fn now_date() -> String {
    now_iso()[..10].to_string()
}

fn absolutize(root: &Path, raw: &str) -> PathBuf {
    let candidate = PathBuf::from(raw.trim());
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

#[derive(Clone, Debug)]
struct TritShadowPaths {
    policy: PathBuf,
    success_criteria: PathBuf,
    trust_state: PathBuf,
    influence_budget: PathBuf,
    influence_guard: PathBuf,
    report_history: PathBuf,
    calibration_history: PathBuf,
}

impl TritShadowPaths {
    fn as_json(&self) -> Value {
        json!({
            "policy": self.policy.to_string_lossy(),
            "success_criteria": self.success_criteria.to_string_lossy(),
            "trust_state": self.trust_state.to_string_lossy(),
            "influence_budget": self.influence_budget.to_string_lossy(),
            "influence_guard": self.influence_guard.to_string_lossy(),
            "report_history": self.report_history.to_string_lossy(),
            "calibration_history": self.calibration_history.to_string_lossy(),
        })
    }
}

fn resolve_path(root: &Path, payload: &Map<String, Value>, key: &str, env_name: &str, fallback_rel: &str) -> PathBuf {
    if let Some(paths) = as_object(payload.get("paths")) {
        if let Some(raw) = paths.get(key) {
            let s = as_str(Some(raw));
            if !s.is_empty() {
                return absolutize(root, &s);
            }
        }
    }
    if let Some(raw) = payload.get("file_path") {
        let s = as_str(Some(raw));
        if !s.is_empty() {
            return absolutize(root, &s);
        }
    }
    if let Ok(raw) = std::env::var(env_name) {
        if !raw.trim().is_empty() {
            return absolutize(root, &raw);
        }
    }
    root.join(fallback_rel)
}

fn resolve_paths(root: &Path, payload: &Map<String, Value>) -> TritShadowPaths {
    TritShadowPaths {
        policy: resolve_path(root, payload, "policy", "AUTONOMY_TRIT_SHADOW_POLICY_PATH", POLICY_REL),
        success_criteria: resolve_path(
            root,
            payload,
            "success_criteria",
            "AUTONOMY_TRIT_SHADOW_SUCCESS_CRITERIA_PATH",
            SUCCESS_CRITERIA_REL,
        ),
        trust_state: resolve_path(root, payload, "trust_state", "AUTONOMY_TRIT_SHADOW_TRUST_STATE_PATH", TRUST_STATE_REL),
        influence_budget: resolve_path(
            root,
            payload,
            "influence_budget",
            "AUTONOMY_TRIT_SHADOW_INFLUENCE_BUDGET_PATH",
            INFLUENCE_BUDGET_REL,
        ),
        influence_guard: resolve_path(
            root,
            payload,
            "influence_guard",
            "AUTONOMY_TRIT_SHADOW_INFLUENCE_GUARD_PATH",
            INFLUENCE_GUARD_REL,
        ),
        report_history: resolve_path(
            root,
            payload,
            "report_history",
            "AUTONOMY_TRIT_SHADOW_REPORT_HISTORY_PATH",
            REPORT_HISTORY_REL,
        ),
        calibration_history: resolve_path(
            root,
            payload,
            "calibration_history",
            "AUTONOMY_TRIT_SHADOW_CALIBRATION_HISTORY_PATH",
            CALIBRATION_HISTORY_REL,
        ),
    }
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("trit_shadow_kernel_create_dir_failed:{}:{err}", parent.display()))?;
    }
    Ok(())
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let temp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        Utc::now().timestamp_millis()
    ));
    let payload = serde_json::to_string_pretty(value)
        .map_err(|err| format!("trit_shadow_kernel_encode_json_failed:{err}"))?;
    let mut file = fs::File::create(&temp)
        .map_err(|err| format!("trit_shadow_kernel_create_tmp_failed:{}:{err}", temp.display()))?;
    file.write_all(payload.as_bytes())
        .and_then(|_| file.write_all(b"\n"))
        .map_err(|err| format!("trit_shadow_kernel_write_tmp_failed:{}:{err}", temp.display()))?;
    fs::rename(&temp, path)
        .map_err(|err| format!("trit_shadow_kernel_rename_tmp_failed:{}:{err}", path.display()))
}

fn read_json(path: &Path) -> Value {
    let Ok(raw) = fs::read_to_string(path) else {
        return Value::Null;
    };
    serde_json::from_str::<Value>(&raw).unwrap_or(Value::Null)
}

fn read_jsonl(path: &Path) -> Vec<Value> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|row| row.is_object())
        .collect()
}

fn default_policy() -> Value {
    json!({
        "version": "1.0",
        "enabled": true,
        "semantics": {
            "locked": true,
            "neutral_on_missing": true,
            "min_non_neutral_signals": 1,
            "min_non_neutral_weight": 0.9,
            "min_confidence_for_non_neutral": 0.3
        },
        "trust": {
            "enabled": true,
            "default_source_trust": 1,
            "source_trust_floor": 0.6,
            "source_trust_ceiling": 1.5,
            "freshness_half_life_hours": 72
        },
        "influence": {
            "stage": 0,
            "min_confidence_stage2": 0.78,
            "min_confidence_stage3": 0.85,
            "max_overrides_per_day": 3,
            "auto_disable_hours_on_regression": 24,
            "activation": {
                "enabled": false,
                "report_window": 4,
                "min_decisions": 20,
                "max_divergence_rate": 0.08,
                "require_success_criteria_pass": true,
                "require_safety_pass": true,
                "require_drift_non_increasing": true,
                "calibration_window": 3,
                "min_calibration_events": 20,
                "min_calibration_accuracy": 0.58,
                "max_calibration_ece": 0.23,
                "min_source_samples": 8,
                "min_source_hit_rate": 0.55,
                "max_sources_below_threshold": 1,
                "allow_if_no_source_data": false
            },
            "auto_stage": {
                "enabled": false,
                "mode": "floor",
                "stage2": {
                    "consecutive_reports": 3,
                    "min_calibration_reports": 1,
                    "min_decisions": 20,
                    "max_divergence_rate": 0.08,
                    "min_calibration_events": 20,
                    "min_calibration_accuracy": 0.55,
                    "max_calibration_ece": 0.25,
                    "require_success_criteria_pass": false,
                    "require_safety_pass": true,
                    "require_drift_non_increasing": true,
                    "require_source_reliability": false
                },
                "stage3": {
                    "consecutive_reports": 6,
                    "min_calibration_reports": 1,
                    "min_decisions": 40,
                    "max_divergence_rate": 0.05,
                    "min_calibration_events": 40,
                    "min_calibration_accuracy": 0.65,
                    "max_calibration_ece": 0.2,
                    "require_success_criteria_pass": true,
                    "require_safety_pass": true,
                    "require_drift_non_increasing": true,
                    "require_source_reliability": false
                }
            }
        },
        "adaptation": {
            "enabled": true,
            "cadence_days": 7,
            "min_samples_per_source": 6,
            "reward_step": 0.04,
            "penalty_step": 0.06,
            "max_delta_per_cycle": 0.08
        }
    })
}

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
        base_trust.get("source_trust_floor").and_then(Value::as_f64).unwrap_or(0.6),
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
    let base_default = base.get("default_source_trust").and_then(Value::as_f64).unwrap_or(1.0);
    let floor = clamp_number(policy.pointer("/trust/source_trust_floor"), 0.01, 5.0, 0.6);
    let ceiling = clamp_number(policy.pointer("/trust/source_trust_ceiling"), floor, 5.0, 1.5);
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
        .filter(|row| row.get("type").and_then(Value::as_str) == Some("trit_shadow_replay_calibration"))
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
    let checks = success.and_then(|v| v.get("checks")).and_then(Value::as_object);
    if clamp_number(summary.and_then(|v| v.get("total_decisions")), 0.0, 1_000_000.0, 0.0)
        < clamp_number(cfg.get("min_decisions"), 0.0, 1_000_000.0, 0.0)
    {
        return false;
    }
    if clamp_number(summary.and_then(|v| v.get("divergence_rate")), 0.0, 1.0, 0.0)
        > clamp_number(cfg.get("max_divergence_rate"), 0.0, 1.0, 1.0)
    {
        return false;
    }
    if as_bool(cfg.get("require_success_criteria_pass"), false)
        && success.and_then(|v| v.get("pass")).and_then(Value::as_bool) != Some(true)
    {
        return false;
    }
    if as_bool(cfg.get("require_safety_pass"), true) {
        let safety = checks.and_then(|v| v.get("safety_regressions")).and_then(Value::as_object);
        if safety.and_then(|v| v.get("pass")).and_then(Value::as_bool) != Some(true) {
            return false;
        }
    }
    if as_bool(cfg.get("require_drift_non_increasing"), true) {
        let drift = checks.and_then(|v| v.get("drift_non_increasing")).and_then(Value::as_object);
        if drift.and_then(|v| v.get("pass")).and_then(Value::as_bool) != Some(true) {
            return false;
        }
    }
    true
}

fn calibration_passes_auto_stage(calibration: &Value, cfg: &Value) -> bool {
    let summary = calibration.get("summary").and_then(Value::as_object);
    if clamp_number(summary.and_then(|v| v.get("total_events")), 0.0, 1_000_000.0, 0.0)
        < clamp_number(cfg.get("min_calibration_events"), 0.0, 1_000_000.0, 0.0)
    {
        return false;
    }
    if clamp_number(summary.and_then(|v| v.get("accuracy")), 0.0, 1.0, 0.0)
        < clamp_number(cfg.get("min_calibration_accuracy"), 0.0, 1.0, 0.0)
    {
        return false;
    }
    if clamp_number(summary.and_then(|v| v.get("expected_calibration_error")), 0.0, 1.0, 1.0)
        > clamp_number(cfg.get("max_calibration_ece"), 0.0, 1.0, 1.0)
    {
        return false;
    }
    true
}

fn calibration_window_passes_auto_stage(rows: &[Value], cfg: &Value, required_window: i64) -> Value {
    let window = required_window.max(1) as usize;
    let recent = rows.iter().rev().take(window).cloned().collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>();
    let pass = recent.len() >= window && recent.iter().all(|row| calibration_passes_auto_stage(row, cfg));
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
            let hit_rate = if samples > 0.0 { weighted_hits / samples } else { 0.0 };
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

fn evaluate_productivity(policy: &Value, paths: &TritShadowPaths) -> Value {
    let activation = policy.pointer("/influence/activation").cloned().unwrap_or_else(|| json!({}));
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
    let recent_reports = reports.iter().rev().take(report_window).cloned().collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>();
    let reports_pass = recent_reports.len() >= report_window && recent_reports.iter().all(|row| report_passes_auto_stage(row, &activation));
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
    let calibration_check = calibration_window_passes_auto_stage(&calibrations, &activation, calibration_window);
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
        calibration_check.get("recent").and_then(Value::as_array).unwrap_or(&Vec::new()),
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
    let auto_cfg = policy.pointer("/influence/auto_stage").cloned().unwrap_or_else(|| json!({}));
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
    let recent3 = reports.iter().rev().take(stage3_window).cloned().collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>();
    let recent2 = reports.iter().rev().take(stage2_window).cloned().collect::<Vec<_>>().into_iter().rev().collect::<Vec<_>>();
    let stage3_reports_pass = recent3.len() >= stage3_window && recent3.iter().all(|row| report_passes_auto_stage(row, &stage3_cfg));
    let stage2_reports_pass = recent2.len() >= stage2_window && recent2.iter().all(|row| report_passes_auto_stage(row, &stage2_cfg));
    let stage3_cal_check = calibration_window_passes_auto_stage(&calibrations, &stage3_cfg, stage3_cal_window);
    let stage2_cal_check = calibration_window_passes_auto_stage(&calibrations, &stage2_cfg, stage2_cal_window);
    let activation_cfg = policy.pointer("/influence/activation").cloned().unwrap_or_else(|| json!({}));
    let stage3_source = if as_bool(stage3_cfg.get("require_source_reliability"), false) {
        source_reliability_gate(stage3_cal_check.get("recent").and_then(Value::as_array).unwrap_or(&Vec::new()), &activation_cfg)
    } else {
        json!({"pass": true})
    };
    let stage2_source = if as_bool(stage2_cfg.get("require_source_reliability"), false) {
        source_reliability_gate(stage2_cal_check.get("recent").and_then(Value::as_array).unwrap_or(&Vec::new()), &activation_cfg)
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
        let mode = if policy.pointer("/influence/auto_stage/mode").and_then(Value::as_str) == Some("override") {
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
    let mut next = if budget.is_object() { budget.clone() } else { default_influence_budget() };
    next["updated_at"] = Value::String(now_iso());
    write_json_atomic(path, &next)?;
    Ok(next)
}

fn can_consume_override(policy: &Value, date_str: &str, path: &Path) -> Value {
    let max_per_day = clamp_int(policy.pointer("/influence/max_overrides_per_day"), 0, 10_000, 0);
    if max_per_day <= 0 {
        return json!({"allowed": false, "reason": "budget_disabled", "remaining": 0});
    }
    let budget = load_influence_budget(path);
    let row = budget.pointer(&format!("/by_date/{date_str}")).cloned().unwrap_or_else(|| json!({"overrides": 0}));
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

fn consume_override(source: &str, policy: &Value, date_str: &str, path: &Path) -> Result<Value, String> {
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
    let used = clamp_int(budget.pointer(&format!("/by_date/{date_str}/overrides")), 0, 1_000_000, 0) + 1;
    budget["by_date"][date_str]["overrides"] = Value::from(used);
    if !budget.pointer(&format!("/by_date/{date_str}/by_source")).map(Value::is_object).unwrap_or(false) {
        budget["by_date"][date_str]["by_source"] = json!({});
    }
    let source_key = if source.trim().is_empty() { "unknown" } else { source.trim() };
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

fn save_influence_guard(guard: &Value, path: &Path) -> Result<Value, String> {
    let mut next = if guard.is_object() { guard.clone() } else { default_influence_guard() };
    next["updated_at"] = Value::String(now_iso());
    write_json_atomic(path, &next)?;
    Ok(next)
}

fn is_influence_blocked(guard: &Value, now_ts: Option<&str>) -> Value {
    if guard.get("disabled").and_then(Value::as_bool) != Some(true) {
        return json!({"blocked": false, "reason": "enabled"});
    }
    let now_ms = now_ts
        .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.timestamp_millis()))
        .unwrap_or_else(|| Utc::now().timestamp_millis());
    let until_ms = guard
        .get("disabled_until")
        .and_then(Value::as_str)
        .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok().map(|dt| dt.timestamp_millis()));
    if let Some(until) = until_ms {
        if now_ms > until {
            return json!({"blocked": false, "reason": "expired"});
        }
    }
    json!({
        "blocked": true,
        "reason": as_str(guard.get("reason")).if_empty_then("disabled"),
        "disabled_until": guard.get("disabled_until").cloned().unwrap_or(Value::Null),
    })
}

fn apply_influence_guard(report_payload: &Value, policy: &Value, path: &Path) -> Result<Value, String> {
    let summary = report_payload.get("summary").and_then(Value::as_object);
    let gate = summary.and_then(|v| v.get("gate")).and_then(Value::as_object);
    let status = as_str(summary.and_then(|v| v.get("status"))).to_ascii_lowercase();
    let should_disable = if gate.and_then(|v| v.get("enabled")).and_then(Value::as_bool) == Some(true) {
        gate.and_then(|v| v.get("pass")).and_then(Value::as_bool) == Some(false)
    } else {
        status == "critical"
    };
    let mut next = load_influence_guard(path);
    let disable_hours = clamp_number(policy.pointer("/influence/auto_disable_hours_on_regression"), 1.0, (24 * 30) as f64, 24.0);
    if should_disable {
        next["disabled"] = Value::Bool(true);
        let reason = if gate.and_then(|v| v.get("enabled")).and_then(Value::as_bool) == Some(true)
            && gate.and_then(|v| v.get("pass")).and_then(Value::as_bool) == Some(false)
        {
            format!(
                "shadow_gate_failed:{}",
                as_str(gate.and_then(|v| v.get("reason"))).if_empty_then("divergence_rate_exceeds_limit")
            )
        } else {
            "shadow_status_critical".to_string()
        };
        next["reason"] = Value::String(reason);
        next["disabled_until"] = Value::String(
            (Utc::now() + Duration::hours(disable_hours.round() as i64)).to_rfc3339(),
        );
    } else {
        next["disabled"] = Value::Bool(false);
        next["reason"] = Value::Null;
        next["disabled_until"] = Value::Null;
    }
    next["last_report_ts"] = report_payload
        .get("ts")
        .cloned()
        .unwrap_or_else(|| Value::String(now_iso()));
    save_influence_guard(&next, path)
}

fn command_payload_map<'a>(payload: &'a Map<String, Value>, key: &str) -> Map<String, Value> {
    payload
        .get(key)
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_else(|| payload.clone())
}

fn run_command(root: &Path, command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    let paths = resolve_paths(root, payload);
    match command {
        "paths" => Ok(paths.as_json()),
        "default-policy" => Ok(default_policy()),
        "normalize-policy" => {
            let policy = command_payload_map(payload, "policy");
            Ok(normalize_policy(&policy))
        }
        "load-policy" => Ok(load_policy_from_path(&paths.policy)),
        "load-success-criteria" => Ok(load_success_criteria_from_path(&paths.success_criteria)),
        "load-trust-state" => {
            let policy = payload
                .get("policy")
                .and_then(Value::as_object)
                .map(normalize_policy)
                .unwrap_or_else(|| load_policy_from_path(&paths.policy));
            Ok(load_trust_state_from_path(&policy, &paths.trust_state))
        }
        "save-trust-state" => {
            let policy = payload
                .get("policy")
                .and_then(Value::as_object)
                .map(normalize_policy)
                .unwrap_or_else(|| load_policy_from_path(&paths.policy));
            let state = payload.get("state").cloned().unwrap_or_else(|| Value::Object(payload.clone()));
            save_trust_state_to_path(&state, &policy, &paths.trust_state)
        }
        "build-trust-map" => {
            let trust_state = payload.get("trust_state").cloned().unwrap_or_else(|| Value::Object(payload.clone()));
            Ok(build_trust_map(&trust_state))
        }
        "evaluate-productivity" => {
            let policy = payload
                .get("policy")
                .and_then(Value::as_object)
                .map(normalize_policy)
                .unwrap_or_else(|| load_policy_from_path(&paths.policy));
            Ok(evaluate_productivity(&policy, &paths))
        }
        "evaluate-auto-stage" => {
            let policy = payload
                .get("policy")
                .and_then(Value::as_object)
                .map(normalize_policy)
                .unwrap_or_else(|| load_policy_from_path(&paths.policy));
            Ok(evaluate_auto_stage(&policy, &paths))
        }
        "resolve-stage-decision" => {
            let policy = payload
                .get("policy")
                .and_then(Value::as_object)
                .map(normalize_policy)
                .unwrap_or_else(|| load_policy_from_path(&paths.policy));
            Ok(resolve_stage_decision(&policy, &paths))
        }
        "resolve-stage" => {
            let policy = payload
                .get("policy")
                .and_then(Value::as_object)
                .map(normalize_policy)
                .unwrap_or_else(|| load_policy_from_path(&paths.policy));
            let stage = resolve_stage_decision(&policy, &paths)
                .get("stage")
                .cloned()
                .unwrap_or(Value::from(0));
            Ok(json!({"stage": stage}))
        }
        "can-consume-override" => {
            let policy = payload
                .get("policy")
                .and_then(Value::as_object)
                .map(normalize_policy)
                .unwrap_or_else(|| load_policy_from_path(&paths.policy));
            let date_str = as_str(payload.get("date_str")).if_empty_then(&now_date());
            Ok(can_consume_override(&policy, &date_str, &paths.influence_budget))
        }
        "consume-override" => {
            let policy = payload
                .get("policy")
                .and_then(Value::as_object)
                .map(normalize_policy)
                .unwrap_or_else(|| load_policy_from_path(&paths.policy));
            let date_str = as_str(payload.get("date_str")).if_empty_then(&now_date());
            let source = as_str(payload.get("source")).if_empty_then("unknown");
            consume_override(&source, &policy, &date_str, &paths.influence_budget)
        }
        "load-influence-guard" => Ok(load_influence_guard(&paths.influence_guard)),
        "save-influence-guard" => {
            let guard = payload.get("guard").cloned().unwrap_or_else(|| Value::Object(payload.clone()));
            save_influence_guard(&guard, &paths.influence_guard)
        }
        "influence-blocked" => {
            let guard = payload
                .get("guard")
                .cloned()
                .unwrap_or_else(|| load_influence_guard(&paths.influence_guard));
            let now_ts = as_str(payload.get("now_ts"));
            Ok(is_influence_blocked(
                &guard,
                if now_ts.is_empty() { None } else { Some(now_ts.as_str()) },
            ))
        }
        "apply-influence-guard" => {
            let policy = payload
                .get("policy")
                .and_then(Value::as_object)
                .map(normalize_policy)
                .unwrap_or_else(|| load_policy_from_path(&paths.policy));
            let report = payload
                .get("report_payload")
                .cloned()
                .or_else(|| payload.get("report").cloned())
                .unwrap_or_else(|| Value::Object(payload.clone()));
            apply_influence_guard(&report, &policy, &paths.influence_guard)
        }
        _ => Err("trit_shadow_kernel_unknown_command".to_string()),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let Some(command) = argv.first().map(|v| v.as_str()) else {
        usage();
        return 1;
    };
    if matches!(command, "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("trit_shadow_kernel", &err));
            return 1;
        }
    };
    let payload = payload_obj(&payload).clone();
    match run_command(root, command, &payload) {
        Ok(out) => {
            print_json_line(&cli_receipt("trit_shadow_kernel", out));
            0
        }
        Err(err) => {
            print_json_line(&cli_error("trit_shadow_kernel", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_file(name: &str) -> PathBuf {
        let base = std::env::temp_dir().join(format!("trit-shadow-kernel-{}-{}", std::process::id(), Utc::now().timestamp_nanos_opt().unwrap_or_default()));
        fs::create_dir_all(&base).unwrap();
        base.join(name)
    }

    #[test]
    fn normalize_policy_clamps_values() {
        let normalized = normalize_policy(payload_obj(&json!({
            "influence": {
                "stage": 7,
                "max_overrides_per_day": -5,
                "auto_stage": {"mode": "override"}
            },
            "trust": {"source_trust_floor": 0.2, "source_trust_ceiling": 9}
        })));
        assert_eq!(normalized.pointer("/influence/stage").and_then(Value::as_i64), Some(3));
        assert_eq!(normalized.pointer("/influence/max_overrides_per_day").and_then(Value::as_i64), Some(0));
        assert_eq!(normalized.pointer("/trust/source_trust_floor").and_then(Value::as_f64), Some(0.2_f64.max(0.01)));
        assert_eq!(normalized.pointer("/influence/auto_stage/mode").and_then(Value::as_str), Some("override"));
    }

    #[test]
    fn trust_state_round_trip_and_map() {
        let path = temp_file("trust_state.json");
        let policy = default_policy();
        let saved = save_trust_state_to_path(&json!({
            "default_source_trust": 1.2,
            "by_source": {
                "policy": {"trust": 1.4, "samples": 10, "hit_rate": 0.7}
            }
        }), &policy, &path).unwrap();
        let loaded = load_trust_state_from_path(&policy, &path);
        assert_eq!(saved.pointer("/by_source/policy/trust"), loaded.pointer("/by_source/policy/trust"));
        let trust_map = build_trust_map(&loaded);
        assert_eq!(trust_map.get("policy").and_then(Value::as_f64), Some(1.4));
    }

    #[test]
    fn productivity_and_auto_stage_activate_from_histories() {
        let root = Path::new(".");
        let report_history = temp_file("reports.jsonl");
        let calibration_history = temp_file("calibration.jsonl");
        fs::write(
            &report_history,
            concat!(
                "{\"type\":\"trit_shadow_report\",\"ok\":true,\"ts\":\"2026-03-17T00:00:00Z\",\"summary\":{\"total_decisions\":30,\"divergence_rate\":0.01},\"success_criteria\":{\"pass\":true,\"checks\":{\"safety_regressions\":{\"pass\":true},\"drift_non_increasing\":{\"pass\":true}}}}\n"
            ),
        ).unwrap();
        fs::write(
            &calibration_history,
            concat!(
                "{\"type\":\"trit_shadow_replay_calibration\",\"ok\":true,\"ts\":\"2026-03-17T00:10:00Z\",\"date\":\"2026-03-17\",\"summary\":{\"total_events\":30,\"accuracy\":0.7,\"expected_calibration_error\":0.1},\"source_reliability\":[{\"source\":\"policy\",\"samples\":12,\"hit_rate\":0.7}]}\n"
            ),
        ).unwrap();
        let payload = json!({
            "paths": {
                "report_history": report_history,
                "calibration_history": calibration_history
            },
            "policy": {
                "influence": {
                    "stage": 1,
                    "activation": {
                        "enabled": true,
                        "report_window": 1,
                        "calibration_window": 1,
                        "min_decisions": 20,
                        "min_calibration_events": 20,
                        "min_source_samples": 8,
                        "min_source_hit_rate": 0.55,
                        "max_sources_below_threshold": 1,
                        "allow_if_no_source_data": false
                    },
                    "auto_stage": {
                        "enabled": true,
                        "mode": "floor",
                        "stage2": {
                            "consecutive_reports": 1,
                            "min_calibration_reports": 1,
                            "min_decisions": 20,
                            "max_divergence_rate": 0.08,
                            "min_calibration_events": 20,
                            "min_calibration_accuracy": 0.55,
                            "max_calibration_ece": 0.25,
                            "require_source_reliability": false
                        },
                        "stage3": {
                            "consecutive_reports": 2,
                            "min_calibration_reports": 2,
                            "min_decisions": 40,
                            "max_divergence_rate": 0.05,
                            "min_calibration_events": 40,
                            "min_calibration_accuracy": 0.65,
                            "max_calibration_ece": 0.2,
                            "require_source_reliability": false
                        }
                    }
                }
            }
        });
        let result = run_command(root, "evaluate-auto-stage", payload_obj(&payload)).unwrap();
        assert_eq!(result.get("stage").and_then(Value::as_i64), Some(2));
        let decision = run_command(root, "resolve-stage-decision", payload_obj(&payload)).unwrap();
        assert_eq!(decision.get("stage").and_then(Value::as_i64), Some(2));
    }

    #[test]
    fn override_budget_and_guard_flow() {
        let root = Path::new(".");
        let budget_path = temp_file("influence_budget.json");
        let guard_path = temp_file("influence_guard.json");
        let policy = json!({"influence": {"max_overrides_per_day": 2, "auto_disable_hours_on_regression": 24}});
        let consume_payload = json!({
            "policy": policy,
            "date_str": "2026-03-17",
            "source": "planner",
            "paths": {"influence_budget": budget_path}
        });
        let first = run_command(root, "consume-override", payload_obj(&consume_payload)).unwrap();
        assert_eq!(first.get("consumed").and_then(Value::as_bool), Some(true));
        let second = run_command(root, "consume-override", payload_obj(&consume_payload)).unwrap();
        assert_eq!(second.get("consumed").and_then(Value::as_bool), Some(true));
        let third = run_command(root, "consume-override", payload_obj(&consume_payload)).unwrap();
        assert_eq!(third.get("consumed").and_then(Value::as_bool), Some(false));

        let guard = run_command(
            root,
            "apply-influence-guard",
            payload_obj(&json!({
                "policy": {"influence": {"auto_disable_hours_on_regression": 24}},
                "paths": {"influence_guard": guard_path},
                "report_payload": {
                    "ts": "2026-03-17T12:00:00Z",
                    "summary": {
                        "status": "critical",
                        "gate": {"enabled": true, "pass": false, "reason": "divergence_rate_exceeds_limit"}
                    }
                }
            })),
        ).unwrap();
        assert_eq!(guard.get("disabled").and_then(Value::as_bool), Some(true));
        let blocked = run_command(
            root,
            "influence-blocked",
            payload_obj(&json!({
                "guard": guard,
                "now_ts": "2026-03-17T12:30:00Z"
            })),
        ).unwrap();
        assert_eq!(blocked.get("blocked").and_then(Value::as_bool), Some(true));
    }
}
