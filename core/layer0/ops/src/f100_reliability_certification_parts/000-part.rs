// SPDX-License-Identifier: Apache-2.0
use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso, parse_args};
use chrono::{DateTime, Duration, NaiveDate, Utc};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

const LANE_ID: &str = "f100_reliability_certification";
const DEFAULT_POLICY_REL: &str = "client/runtime/config/f100_reliability_certification_policy.json";

#[derive(Debug, Clone)]
struct Tier {
    min_uptime: f64,
    max_receipt_p95_ms: f64,
    max_receipt_p99_ms: f64,
    max_incident_rate: f64,
    max_change_fail_rate: f64,
    max_error_budget_burn_ratio: f64,
}

#[derive(Debug, Clone)]
struct Policy {
    strict_default: bool,
    active_tier: String,
    tiers: BTreeMap<String, Tier>,
    window_days: i64,
    missing_metric_fail_closed: bool,
    sources_execution_reliability_path: PathBuf,
    sources_error_budget_latest_path: PathBuf,
    sources_error_budget_history_path: PathBuf,
    sources_spine_runs_dir: PathBuf,
    sources_incident_log_path: PathBuf,
    drill_evidence_paths: Vec<PathBuf>,
    rollback_evidence_paths: Vec<PathBuf>,
    min_drill_evidence_count: usize,
    min_rollback_evidence_count: usize,
    latest_path: PathBuf,
    history_path: PathBuf,
    policy_path: PathBuf,
}

fn usage() {
    println!("Usage:");
    println!("  infring-ops f100-reliability-certification run [--strict=1|0] [--policy=<path>]");
    println!("  infring-ops f100-reliability-certification status [--policy=<path>]");
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn bool_flag(raw: Option<&String>, fallback: bool) -> bool {
    lane_utils::parse_bool(raw.map(String::as_str), fallback)
}

fn read_json(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("read_json_failed:{}:{err}", path.display()))?;
    serde_json::from_str::<Value>(&raw)
        .map_err(|err| format!("parse_json_failed:{}:{err}", path.display()))
}

fn read_jsonl(path: &Path) -> Vec<Value> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                return None;
            }
            serde_json::from_str::<Value>(trimmed).ok()
        })
        .collect()
}

fn resolve_path(root: &Path, raw: Option<&str>, fallback: &str) -> PathBuf {
    let token = raw.unwrap_or(fallback).trim();
    if token.is_empty() {
        return root.join(fallback);
    }
    let candidate = PathBuf::from(token);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn value_as_f64(value: Option<&Value>) -> Option<f64> {
    match value {
        Some(Value::Number(n)) => n.as_f64(),
        Some(Value::String(raw)) => raw.trim().parse::<f64>().ok(),
        _ => None,
    }
}

fn parse_iso_day(value: Option<&Value>) -> Option<NaiveDate> {
    let value = value?;
    let token = value.as_str()?.trim();
    if token.is_empty() {
        return None;
    }
    if token.len() >= 10 {
        let day = &token[..10];
        if let Ok(parsed) = NaiveDate::parse_from_str(day, "%Y-%m-%d") {
            return Some(parsed);
        }
    }
    DateTime::parse_from_rfc3339(token)
        .ok()
        .map(|dt| dt.date_naive())
}

fn percentile(values: &[f64], q: f64) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    let quantile = if q.is_finite() {
        q.clamp(0.0, 1.0)
    } else {
        0.5
    };
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    let idx = ((sorted.len() as f64) * quantile).ceil() as usize;
    let idx = idx.saturating_sub(1).min(sorted.len().saturating_sub(1));
    sorted.get(idx).copied()
}

fn collect_spine_latency_metrics(
    spine_runs_dir: &Path,
) -> (Option<f64>, Option<f64>, usize, usize) {
    let mut latency_ms = Vec::<f64>::new();
    let mut files_scanned = 0usize;

    if let Ok(entries) = fs::read_dir(spine_runs_dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|v| v.to_str()) != Some("jsonl") {
                continue;
            }
            files_scanned += 1;
            for row in read_jsonl(&path) {
                match row.get("type").and_then(Value::as_str).unwrap_or("") {
                    "spine_run_complete" => {
                        if let Some(ms) = row.get("elapsed_ms").and_then(Value::as_f64) {
                            latency_ms.push(ms);
                        }
                    }
                    "spine_observability_trace" => {
                        if let Some(ms) = row.get("trace_duration_ms").and_then(Value::as_f64) {
                            latency_ms.push(ms);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    (
        percentile(&latency_ms, 0.95),
        percentile(&latency_ms, 0.99),
        latency_ms.len(),
        files_scanned,
    )
}

fn collect_incident_rate(
    incident_log_path: &Path,
    window_start: NaiveDate,
    now: NaiveDate,
) -> (f64, usize) {
    let rows = read_jsonl(incident_log_path);
    let mut incidents = 0usize;
    for row in rows {
        if row.get("type").and_then(Value::as_str) != Some("autonomy_human_escalation") {
            continue;
        }
        let Some(day) = parse_iso_day(row.get("ts").or_else(|| row.get("date"))) else {
            continue;
        };
        if day < window_start || day > now {
            continue;
        }
        incidents += 1;
    }
    let window_days = (now - window_start).num_days().max(1) as f64;
    (incidents as f64 / window_days, incidents)
}

fn collect_change_fail_rate(
    history_path: &Path,
    window_start: NaiveDate,
    now: NaiveDate,
) -> (f64, usize, usize) {
    let rows = read_jsonl(history_path);
    let mut total = 0usize;
    let mut failed = 0usize;

    for row in rows {
        let Some(day) = parse_iso_day(row.get("ts").or_else(|| row.get("date"))) else {
            continue;
        };
        if day < window_start || day > now {
            continue;
        }
        total += 1;
        let ok = row.get("ok").and_then(Value::as_bool).unwrap_or(false);
        let blocked = row
            .get("gate")
            .and_then(|v| v.get("promotion_blocked"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        if !ok || blocked {
            failed += 1;
        }
    }

    let rate = if total > 0 {
        failed as f64 / total as f64
    } else {
        0.0
    };
    (rate, total, failed)
}

fn evidence_status(paths: &[PathBuf], min_count: usize) -> Value {
    let mut found = Vec::<String>::new();
    let mut missing = Vec::<String>::new();
    for path in paths {
        if path.exists() {
            found.push(path.to_string_lossy().to_string());
        } else {
            missing.push(path.to_string_lossy().to_string());
        }
    }
    let ok = found.len() >= min_count;
    json!({
        "ok": ok,
        "required_min": min_count,
        "found_count": found.len(),
        "found_paths": found,
        "missing_paths": missing
    })
}
