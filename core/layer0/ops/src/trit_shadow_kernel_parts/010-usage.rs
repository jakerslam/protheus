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
use crate::now_iso;

const POLICY_REL: &str = "client/config/trit_shadow_policy.json";
const SUCCESS_CRITERIA_REL: &str = "client/config/trit_shadow_success_criteria.json";
const TRUST_STATE_REL: &str = "client/local/state/autonomy/trit_shadow_trust_state.json";
const INFLUENCE_BUDGET_REL: &str = "client/local/state/autonomy/trit_shadow_influence_budget.json";
const INFLUENCE_GUARD_REL: &str = "client/local/state/autonomy/trit_shadow_influence_guard.json";
const REPORT_HISTORY_REL: &str = "client/local/state/autonomy/trit_shadow_reports/history.jsonl";
const CALIBRATION_HISTORY_REL: &str =
    "client/local/state/autonomy/trit_shadow_calibration/history.jsonl";

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
    crate::contract_lane_utils::cli_receipt(kind, payload)
}

fn cli_error(kind: &str, error: &str) -> Value {
    crate::contract_lane_utils::cli_error(kind, error)
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
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
    let normalized = raw.trim().replace('\\', "/");
    let candidate = PathBuf::from(&normalized);
    if candidate
        .components()
        .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return root.to_path_buf();
    }
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

fn resolve_path(
    root: &Path,
    payload: &Map<String, Value>,
    key: &str,
    env_name: &str,
    fallback_rel: &str,
) -> PathBuf {
    let absolutize_guarded = |raw: &str| {
        let normalized = raw.trim().replace('\\', "/");
        let candidate = PathBuf::from(&normalized);
        if candidate
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
        {
            return root.join(fallback_rel);
        }
        if candidate.is_absolute() {
            candidate
        } else {
            root.join(candidate)
        }
    };
    if let Some(paths) = as_object(payload.get("paths")) {
        if let Some(raw) = paths.get(key) {
            let s = as_str(Some(raw));
            if !s.is_empty() {
                return absolutize_guarded(&s);
            }
        }
    }
    if let Some(raw) = payload.get("file_path") {
        let s = as_str(Some(raw));
        if !s.is_empty() {
            return absolutize_guarded(&s);
        }
    }
    if let Ok(raw) = std::env::var(env_name) {
        if !raw.trim().is_empty() {
            return absolutize_guarded(&raw);
        }
    }
    root.join(fallback_rel)
}

fn resolve_paths(root: &Path, payload: &Map<String, Value>) -> TritShadowPaths {
    TritShadowPaths {
        policy: resolve_path(
            root,
            payload,
            "policy",
            "AUTONOMY_TRIT_SHADOW_POLICY_PATH",
            POLICY_REL,
        ),
        success_criteria: resolve_path(
            root,
            payload,
            "success_criteria",
            "AUTONOMY_TRIT_SHADOW_SUCCESS_CRITERIA_PATH",
            SUCCESS_CRITERIA_REL,
        ),
        trust_state: resolve_path(
            root,
            payload,
            "trust_state",
            "AUTONOMY_TRIT_SHADOW_TRUST_STATE_PATH",
            TRUST_STATE_REL,
        ),
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
        fs::create_dir_all(parent).map_err(|err| {
            format!(
                "trit_shadow_kernel_create_dir_failed:{}:{err}",
                parent.display()
            )
        })?;
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
    let mut file = fs::File::create(&temp).map_err(|err| {
        format!(
            "trit_shadow_kernel_create_tmp_failed:{}:{err}",
            temp.display()
        )
    })?;
    file.write_all(payload.as_bytes())
        .and_then(|_| file.write_all(b"\n"))
        .map_err(|err| {
            format!(
                "trit_shadow_kernel_write_tmp_failed:{}:{err}",
                temp.display()
            )
        })?;
    fs::rename(&temp, path).map_err(|err| {
        format!(
            "trit_shadow_kernel_rename_tmp_failed:{}:{err}",
            path.display()
        )
    })
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
