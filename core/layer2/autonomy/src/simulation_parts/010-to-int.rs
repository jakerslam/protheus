// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/autonomy (authoritative).

use crate::{
    append_jsonl, now_iso, parse_bool_str, parse_date_or_today, read_json, read_jsonl,
    resolve_runtime_path, round_to, write_json_atomic,
};
use chrono::{Duration, NaiveDate};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};

fn to_int(raw: Option<&str>, fallback: i64, lo: i64, hi: i64) -> i64 {
    raw.and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(fallback)
        .clamp(lo, hi)
}

fn normalize_signal_token(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    let mut prev_sep = false;
    for ch in raw.trim().chars() {
        let next = if ch.is_ascii_alphanumeric() {
            ch.to_ascii_lowercase()
        } else {
            '_'
        };
        if next == '_' {
            if prev_sep {
                continue;
            }
            prev_sep = true;
        } else {
            prev_sep = false;
        }
        out.push(next);
    }
    out.trim_matches('_').to_string()
}

fn canonical_run_result(raw: &str) -> String {
    match normalize_signal_token(raw).as_str() {
        "lockbusy" | "lock_busy" => "lock_busy".to_string(),
        "stop_repeat_gate" | "stop_repeat_gate_interval" | "repeat_gate_stop"
        | "repeat_gate_interval_stop" => "stop_repeat_gate_interval".to_string(),
        "stop_init_gate" | "stop_init_gate_interval" => "stop_init_gate_interval".to_string(),
        "executed_ok" | "executed_success" | "run_executed" | "execute" => "executed".to_string(),
        other => other.to_string(),
    }
}

fn canonical_outcome(raw: &str) -> String {
    match normalize_signal_token(raw).as_str() {
        "ship" | "shipped_ok" | "shipped_success" => "shipped".to_string(),
        other => other.to_string(),
    }
}

fn date_window(end_date: &str, days: i64) -> Vec<String> {
    let fallback = chrono::Utc::now().date_naive();
    let end = NaiveDate::parse_from_str(end_date, "%Y-%m-%d").unwrap_or(fallback);
    (0..days)
        .map(|idx| (end - Duration::days(idx)).format("%Y-%m-%d").to_string())
        .collect()
}

fn safe_rate(num: i64, den: i64) -> f64 {
    if den <= 0 {
        0.0
    } else {
        num as f64 / den as f64
    }
}

fn parse_ts_ms(v: Option<&Value>) -> Option<i64> {
    let text = v.and_then(Value::as_str)?.trim();
    chrono::DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

fn is_policy_hold_event(row: &Value) -> bool {
    let event_type = normalize_signal_token(row.get("type").and_then(Value::as_str).unwrap_or(""));
    if event_type != "autonomy_run" {
        return false;
    }
    if row.get("policy_hold").and_then(Value::as_bool) == Some(true) {
        return true;
    }
    let result = canonical_run_result(row.get("result").and_then(Value::as_str).unwrap_or(""));
    if result.starts_with("no_candidates_policy_") {
        return true;
    }
    if result.starts_with("stop_repeat_gate_") || result.starts_with("stop_init_gate_") {
        return true;
    }

    let block_reason =
        normalize_signal_token(row.get("route_block_reason").and_then(Value::as_str).unwrap_or(""));
    block_reason.contains("gate_manual") || block_reason.contains("budget")
}

fn is_budget_hold_event(row: &Value) -> bool {
    let event_type = normalize_signal_token(row.get("type").and_then(Value::as_str).unwrap_or(""));
    if event_type != "autonomy_run" {
        return false;
    }
    let result = canonical_run_result(row.get("result").and_then(Value::as_str).unwrap_or(""));
    let hold_reason =
        normalize_signal_token(row.get("policy_hold_reason").and_then(Value::as_str).unwrap_or(""));
    let block_reason =
        normalize_signal_token(row.get("route_block_reason").and_then(Value::as_str).unwrap_or(""));

    result.contains("budget")
        || result.contains("burn_rate")
        || hold_reason.contains("budget")
        || hold_reason.contains("burn_rate")
        || block_reason.contains("budget")
        || block_reason.contains("burn_rate")
}

fn is_safety_stop(row: &Value) -> bool {
    let result = canonical_run_result(row.get("result").and_then(Value::as_str).unwrap_or(""));
    result.contains("safety")
}

fn is_no_progress(row: &Value) -> bool {
    let result = canonical_run_result(row.get("result").and_then(Value::as_str).unwrap_or(""));
    let outcome = canonical_outcome(row.get("outcome").and_then(Value::as_str).unwrap_or(""));

    result == "executed" && outcome != "shipped"
}

fn build_checks(counters: &Map<String, Value>, autopause_active: bool) -> Value {
    let attempts = counters
        .get("attempts")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let executed = counters
        .get("executed")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let shipped = counters.get("shipped").and_then(Value::as_i64).unwrap_or(0);
    let no_progress = counters
        .get("no_progress")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let safety_stops = counters
        .get("safety_stops")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let policy_holds = counters
        .get("policy_holds")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let budget_holds = counters
        .get("budget_holds")
        .and_then(Value::as_i64)
        .unwrap_or(0);

    let drift_warn = std::env::var("AUTONOMY_SIM_DRIFT_WARN")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.65);
    let drift_fail = std::env::var("AUTONOMY_SIM_DRIFT_FAIL")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.85);
    let yield_warn = std::env::var("AUTONOMY_SIM_YIELD_WARN")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.2);
    let yield_fail = std::env::var("AUTONOMY_SIM_YIELD_FAIL")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.08);
    let safety_warn = std::env::var("AUTONOMY_SIM_SAFETY_WARN")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.25);
    let safety_fail = std::env::var("AUTONOMY_SIM_SAFETY_FAIL")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.45);
    let policy_hold_warn = std::env::var("AUTONOMY_SIM_POLICY_HOLD_WARN")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.2);
    let policy_hold_fail = std::env::var("AUTONOMY_SIM_POLICY_HOLD_FAIL")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.35);
    let budget_hold_warn = std::env::var("AUTONOMY_SIM_BUDGET_HOLD_WARN")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.12);
    let budget_hold_fail = std::env::var("AUTONOMY_SIM_BUDGET_HOLD_FAIL")
        .ok()
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.25);
    let policy_enforce_fail = parse_bool_str(
        std::env::var("AUTONOMY_SIM_ENFORCE_POLICY_HOLD_FAIL")
            .ok()
            .as_deref(),
        false,
    );
    let budget_enforce_fail = parse_bool_str(
        std::env::var("AUTONOMY_SIM_ENFORCE_BUDGET_HOLD_FAIL")
            .ok()
            .as_deref(),
        false,
    );
    let autopause_fail = parse_bool_str(
        std::env::var("AUTONOMY_SIM_AUTOPAUSE_ACTIVE_FAIL")
            .ok()
            .as_deref(),
        true,
    );
    let min_attempts = to_int(
        std::env::var("AUTONOMY_SIM_MIN_ATTEMPTS").ok().as_deref(),
        5,
        1,
        100000,
    );

    let drift_rate = round_to(safe_rate(no_progress, attempts), 3);
    let yield_rate = round_to(safe_rate(shipped, executed), 3);
    let safety_rate = round_to(safe_rate(safety_stops, attempts), 3);
    let policy_hold_rate = round_to(safe_rate(policy_holds, attempts), 3);
    let budget_hold_rate = round_to(safe_rate(budget_holds, attempts), 3);

    let policy_status = if policy_hold_rate >= policy_hold_fail {
        if policy_enforce_fail {
            "fail"
        } else {
            "warn"
        }
    } else if policy_hold_rate >= policy_hold_warn {
        "warn"
    } else {
        "pass"
    };

    let budget_status = if budget_hold_rate >= budget_hold_fail {
        if budget_enforce_fail {
            "fail"
        } else {
            "warn"
        }
    } else if budget_hold_rate >= budget_hold_warn {
        "warn"
    } else {
        "pass"
    };

    json!({
        "drift_rate": {
            "value": drift_rate,
            "warn": drift_warn,
            "fail": drift_fail,
            "status": if drift_rate >= drift_fail { "fail" } else if drift_rate >= drift_warn { "warn" } else { "pass" }
        },
        "yield_rate": {
            "value": yield_rate,
            "warn": yield_warn,
            "fail": yield_fail,
            "status": if yield_rate <= yield_fail { "fail" } else if yield_rate <= yield_warn { "warn" } else { "pass" }
        },
        "safety_stop_rate": {
            "value": safety_rate,
            "warn": safety_warn,
            "fail": safety_fail,
            "status": if safety_rate >= safety_fail { "fail" } else if safety_rate >= safety_warn { "warn" } else { "pass" }
        },
        "attempt_volume": {
            "value": attempts,
            "min": min_attempts,
            "status": if attempts < min_attempts { "warn" } else { "pass" }
        },
        "policy_hold_rate": {
            "value": policy_hold_rate,
            "warn": policy_hold_warn,
            "fail": policy_hold_fail,
            "enforce_fail": policy_enforce_fail,
            "status": policy_status
        },
        "budget_hold_rate": {
            "value": budget_hold_rate,
            "warn": budget_hold_warn,
            "fail": budget_hold_fail,
            "enforce_fail": budget_enforce_fail,
            "status": budget_status
        },
        "budget_autopause_active": {
            "value": autopause_active,
            "fail_when_active": autopause_fail,
            "status": if autopause_active { if autopause_fail { "fail" } else { "warn" } } else { "pass" }
        }
    })
}

fn verdict_from_checks(checks: &Value) -> &'static str {
    let rows = checks.as_object().cloned().unwrap_or_default();
    if rows
        .values()
        .any(|row| row.get("status").and_then(Value::as_str) == Some("fail"))
    {
        return "fail";
    }
    if rows
        .values()
        .any(|row| row.get("status").and_then(Value::as_str) == Some("warn"))
    {
        return "warn";
    }
    "pass"
}

fn reason_counts(rows: &[Value], reason_fn: fn(&Value) -> String) -> Value {
    let mut map = BTreeMap::<String, i64>::new();
    for row in rows {
        let key = reason_fn(row);
        *map.entry(key).or_insert(0) += 1;
    }
    let obj: Map<String, Value> = map.into_iter().map(|(k, v)| (k, json!(v))).collect();
    Value::Object(obj)
}

fn hold_reason(row: &Value) -> String {
    let reason = row
        .get("policy_hold_reason")
        .or_else(|| row.get("route_block_reason"))
        .or_else(|| row.get("result"))
        .and_then(Value::as_str)
        .unwrap_or("unknown")
        .trim();
    let normalized = normalize_signal_token(reason);
    if normalized.is_empty() {
        "unknown".to_string()
    } else {
        normalized
    }
}

fn read_budget_snapshot(path: &Path, end_date: &str, run_rows: &[Value]) -> Value {
    let snapshot = read_json(path);
    let now_ms = chrono::Utc::now().timestamp_millis();
    let active = snapshot
        .get("active")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let until_ms = snapshot
        .get("until_ms")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    let currently_active = active && (until_ms <= 0 || until_ms > now_ms);

    let end_of_day_ms = chrono::DateTime::parse_from_rfc3339(&format!(
        "{}T23:59:59.999Z",
        parse_date_or_today(Some(end_date))
    ))
    .ok()
    .map(|v| v.timestamp_millis())
    .unwrap_or(now_ms);

    let mut explicit_last: Option<(i64, bool, Option<i64>)> = None;
    let mut implicit_last: Option<i64> = None;

    for row in run_rows {
        if row.get("type").and_then(Value::as_str) != Some("autonomy_run") {
            continue;
        }
        let ts_ms = parse_ts_ms(row.get("ts"));
        let Some(ts_ms) = ts_ms else {
            continue;
        };
        if ts_ms > end_of_day_ms {
            continue;
        }

        let autopause = row
            .get("route_summary")
            .and_then(Value::as_object)
            .and_then(|m| m.get("budget_global_guard"))
            .and_then(Value::as_object)
            .and_then(|m| m.get("autopause"))
            .and_then(Value::as_object)
            .cloned();

        if let Some(ap) = autopause {
            if let Some(ap_active) = ap.get("active").and_then(Value::as_bool) {
                let ap_until = ap
                    .get("until")
                    .and_then(Value::as_str)
                    .and_then(|s| chrono::DateTime::parse_from_rfc3339(s).ok())
                    .map(|dt| dt.timestamp_millis());
                if explicit_last.map(|(ms, _, _)| ts_ms >= ms).unwrap_or(true) {
                    explicit_last = Some((ts_ms, ap_active, ap_until));
                }
            }
        }

        let result = row
            .get("result")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        let block_reason = row
            .get("route_block_reason")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if result.contains("budget_autopause") || block_reason.contains("budget_autopause") {
            if implicit_last.map(|ms| ts_ms >= ms).unwrap_or(true) {
                implicit_last = Some(ts_ms);
            }
        }
    }

    let observed_in_window = explicit_last.is_some() || implicit_last.is_some();
    let active_at_window_end = if let Some((_, active_flag, until_opt)) = explicit_last {
        if !active_flag {
            false
        } else {
            until_opt.map(|until| until > end_of_day_ms).unwrap_or(true)
        }
    } else {
        implicit_last.is_some()
    };

    let external_override = std::env::var("AUTONOMY_SIM_RUNS_DIR").is_ok()
        || std::env::var("AUTONOMY_SIM_PROPOSALS_DIR").is_ok();
    let snapshot_suggests_active =
        currently_active && parse_date_or_today(Some(end_date)) == parse_date_or_today(None);

    let active_relevant = if external_override {
        active_at_window_end
    } else {
        active_at_window_end || (!observed_in_window && snapshot_suggests_active)
    };

    let signal_source = if explicit_last.is_some() {
        Value::String("route_summary.autopause".to_string())
    } else if implicit_last.is_some() {
        Value::String("budget_autopause_signal".to_string())
    } else {
        Value::Null
    };

    json!({
        "path": path,
        "active": active,
        "currently_active": currently_active,
        "active_relevant": active_relevant,
        "source": snapshot.get("source").cloned().unwrap_or(Value::Null),
        "reason": snapshot.get("reason").cloned().unwrap_or(Value::Null),
        "pressure": snapshot.get("pressure").cloned().unwrap_or(Value::Null),
        "until": snapshot.get("until").cloned().unwrap_or(Value::Null),
        "updated_at": snapshot.get("updated_at").cloned().unwrap_or(Value::Null),
        "observed_in_window": observed_in_window,
        "active_at_window_end": active_at_window_end,
        "signal_source": signal_source,
        "explicit_last_ts": explicit_last.map(|(ms, _, _)| chrono::DateTime::from_timestamp_millis(ms).map(|dt| dt.to_rfc3339()).unwrap_or_default()),
        "implicit_last_ts": implicit_last.map(|ms| chrono::DateTime::from_timestamp_millis(ms).map(|dt| dt.to_rfc3339()).unwrap_or_default()),
        "source_mode": if external_override { "derived_from_runs" } else { "live_state_plus_runs" },
        "snapshot_fallback_used": if external_override { false } else { !observed_in_window && snapshot_suggests_active }
    })
}
