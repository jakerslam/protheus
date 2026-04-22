// SPDX-License-Identifier: Apache-2.0
use crate::importance::{band_rank, infer_from_event, to_json as importance_to_json};
use crate::now_iso;
use base64::Engine;
use chrono::{TimeZone, Utc};
use crate::execution_lane_bridge::{
    evaluate_importance_json, prioritize_attention_json, DEFAULT_FRONT_JUMP_THRESHOLD,
    INITIATIVE_POLICY_VERSION,
};
use serde_json::{json, Value};
use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

#[derive(Debug, Clone)]
struct AttentionContract {
    enabled: bool,
    push_attention_queue: bool,
    queue_path: PathBuf,
    receipts_path: PathBuf,
    latest_path: PathBuf,
    cursor_state_path: PathBuf,
    max_queue_depth: usize,
    backpressure_soft_watermark: usize,
    backpressure_hard_watermark: usize,
    max_batch_size: usize,
    ttl_hours: i64,
    dedupe_window_hours: i64,
    backpressure_drop_below: String,
    escalate_levels: Vec<String>,
    priority_map: BTreeMap<String, i64>,
    require_layer2_authority: bool,
}

#[derive(Debug, Clone)]
struct Layer2ImportanceDecision {
    score: f64,
    band: String,
    priority: i64,
    front_jump: bool,
    initiative_action: String,
    initiative_policy_version: String,
    initiative_repeat_after_sec: i64,
    initiative_max_messages: i64,
}

fn usage() {
    eprintln!("Usage:");
    eprintln!(
        "  protheus-ops attention-queue enqueue --event-json-base64=<base64> [--run-context=<value>]"
    );
    eprintln!("  protheus-ops attention-queue enqueue --event-json=<json> [--run-context=<value>]");
    eprintln!("  protheus-ops attention-queue status");
    eprintln!(
        "  protheus-ops attention-queue next [--consumer=<id>] [--limit=<n>] [--wait-ms=<n>] [--run-context=<value>]"
    );
    eprintln!(
        "  protheus-ops attention-queue ack --consumer=<id> --through-index=<n> --cursor-token=<token> [--run-context=<value>]"
    );
    eprintln!(
        "  protheus-ops attention-queue drain [--consumer=<id>] [--limit=<n>] [--wait-ms=<n>] [--run-context=<value>]"
    );
    eprintln!(
        "  protheus-ops attention-queue compact [--retain=<n>] [--min-acked=<n>] [--run-context=<value>]"
    );
}

fn read_json(path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&raw).ok()
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

fn write_json(path: &Path, value: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut raw) = serde_json::to_string_pretty(value) {
        raw.push('\n');
        let _ = fs::write(path, raw);
    }
}

fn write_jsonl(path: &Path, rows: &[Value]) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let mut out = String::new();
    for row in rows {
        if let Ok(line) = serde_json::to_string(row) {
            out.push_str(&line);
            out.push('\n');
        }
    }
    let _ = fs::write(path, out);
}

fn append_jsonl(path: &Path, row: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(line) = serde_json::to_string(row) {
        let _ = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| {
                std::io::Write::write_all(&mut file, format!("{line}\n").as_bytes())
            });
    }
}

fn parse_cli_flags(argv: &[String]) -> BTreeMap<String, String> {
    crate::contract_lane_utils::parse_cli_flags(argv)
}

fn bool_from_env(name: &str) -> Option<bool> {
    let raw = std::env::var(name).ok()?;
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => Some(true),
        "0" | "false" | "no" | "off" => Some(false),
        _ => None,
    }
}

fn normalize_path(root: &Path, value: Option<&Value>, fallback: &str) -> PathBuf {
    let raw = value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback);
    let candidate = PathBuf::from(raw);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn normalize_consumer_id(raw: Option<&str>) -> String {
    let mut out = String::new();
    for ch in raw.unwrap_or_default().trim().chars() {
        let keep = ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.' | ':' | '@');
        if keep {
            out.push(ch.to_ascii_lowercase());
        } else if !out.ends_with('_') {
            out.push('_');
        }
        if out.len() >= 80 {
            break;
        }
    }
    out.trim_matches('_').to_string()
}

fn parse_limit(raw: Option<&String>, fallback: usize, max: usize) -> usize {
    let parsed = raw
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(fallback);
    parsed.clamp(1, max)
}

fn parse_non_negative_limit(raw: Option<&String>, fallback: usize, max: usize) -> usize {
    let parsed = raw
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(fallback);
    parsed.clamp(0, max)
}

fn parse_wait_ms(raw: Option<&String>, fallback: u64, max: u64) -> u64 {
    let parsed = raw
        .and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(fallback);
    parsed.clamp(0, max)
}

fn parse_through_index(raw: Option<&String>) -> Option<usize> {
    raw.and_then(|v| v.trim().parse::<usize>().ok())
}

fn severity_rank(raw: &str) -> i64 {
    match raw.trim().to_ascii_lowercase().as_str() {
        "critical" => 3,
        "warn" | "warning" => 2,
        "info" => 1,
        _ => 1,
    }
}

fn attention_lane_rank(raw: &str) -> i64 {
    match raw.trim().to_ascii_lowercase().as_str() {
        "critical" => 3,
        "standard" => 2,
        "background" => 1,
        _ => 2,
    }
}

fn classify_attention_lane(
    source: &str,
    source_type: &str,
    severity: &str,
    summary: &str,
    band: &str,
) -> String {
    let normalized_severity = normalize_severity(Some(severity));
    let normalized_band = band.trim().to_ascii_lowercase();
    let normalized_source = source.trim().to_ascii_lowercase();
    let normalized_source_type = source_type.trim().to_ascii_lowercase();
    let normalized_summary = summary.trim().to_ascii_lowercase();
    let is_critical_summary = normalized_summary.contains("fail")
        || normalized_summary.contains("error")
        || normalized_summary.contains("critical")
        || normalized_summary.contains("degraded")
        || normalized_summary.contains("alert")
        || normalized_summary.contains("benchmark_sanity")
        || normalized_summary.contains("backpressure")
        || normalized_summary.contains("throttle")
        || normalized_summary.contains("stale");
    if normalized_severity == "critical"
        || normalized_severity == "warn"
        || normalized_band == "p0"
        || normalized_band == "p1"
        || is_critical_summary
    {
        return "critical".to_string();
    }
    let background_source = normalized_source_type.contains("receipt")
        || normalized_source_type.contains("audit")
        || normalized_source_type.contains("timeline")
        || normalized_source_type.contains("history")
        || normalized_source_type.contains("log")
        || normalized_source_type.contains("trace")
        || normalized_source.contains("receipt")
        || normalized_source.contains("audit")
        || normalized_source.contains("timeline")
        || normalized_source.contains("history")
        || normalized_source.contains("log")
        || normalized_source.contains("trace");
    let background_band =
        normalized_severity == "info" && (normalized_band == "p3" || normalized_band == "p4");
    if background_source || background_band {
        "background".to_string()
    } else {
        "standard".to_string()
    }
}

fn parse_ts_ms(raw: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.timestamp_millis())
}

fn ts_ms_to_iso(ts_ms: i64) -> String {
    Utc.timestamp_millis_opt(ts_ms)
        .single()
        .map(|dt| dt.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
        .unwrap_or_else(now_iso)
}

fn clean_text(value: Option<&str>, max_len: usize) -> String {
    let mut out = String::new();
    if let Some(raw) = value {
        for ch in raw.split_whitespace().collect::<Vec<_>>().join(" ").chars() {
            if out.len() >= max_len {
                break;
            }
            out.push(ch);
        }
    }
    out.trim().to_string()
}

fn normalize_severity(raw: Option<&str>) -> String {
    match raw.unwrap_or_default().trim().to_ascii_lowercase().as_str() {
        "critical" => "critical".to_string(),
        "warn" | "warning" => "warn".to_string(),
        "info" => "info".to_string(),
        _ => "info".to_string(),
    }
}

fn parse_f64(value: Option<&Value>) -> Option<f64> {
    value
        .and_then(Value::as_f64)
        .and_then(|n| if n.is_finite() { Some(n) } else { None })
}

fn parse_layer2_importance(raw: &str) -> Option<Layer2ImportanceDecision> {
    let parsed = serde_json::from_str::<Value>(raw).ok()?;
    if parsed.get("ok").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    let score = parsed.get("score").and_then(Value::as_f64)?;
    let band = parsed
        .get("band")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| "p4".to_string());
    let priority = parsed
        .get("priority")
        .and_then(Value::as_i64)
        .unwrap_or_else(|| ((score * 1000.0).round() as i64).clamp(1, 1000))
        .clamp(1, 1000);
    let front_jump = parsed
        .get("front_jump")
        .and_then(Value::as_bool)
        .unwrap_or(score >= DEFAULT_FRONT_JUMP_THRESHOLD);
    let initiative_action = parsed
        .get("initiative_action")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| "silent".to_string());
    let initiative_policy_version = parsed
        .get("initiative_policy_version")
        .and_then(Value::as_str)
        .map(str::to_string)
        .unwrap_or_else(|| INITIATIVE_POLICY_VERSION.to_string());
    let initiative_repeat_after_sec = parsed
        .get("initiative_repeat_after_sec")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    let initiative_max_messages = parsed
        .get("initiative_max_messages")
        .and_then(Value::as_i64)
        .unwrap_or(0)
        .max(0);
    Some(Layer2ImportanceDecision {
        score: score.clamp(0.0, 1.0),
        band,
        priority,
        front_jump,
        initiative_action,
        initiative_policy_version,
        initiative_repeat_after_sec,
        initiative_max_messages,
    })
}

fn evaluate_importance_via_layer2(
    event: &Value,
    fallback: &crate::importance::ImportanceDecision,
) -> Option<Layer2ImportanceDecision> {
    let payload = json!({
        "criticality": fallback.criticality,
        "urgency": fallback.urgency,
        "impact": fallback.impact,
        "user_relevance": fallback.user_relevance,
        "confidence": fallback.confidence,
        "core_floor": fallback.core_floor,
        "inherited_score": event.pointer("/importance/score").and_then(Value::as_f64).unwrap_or(fallback.score),
        "front_jump_threshold": DEFAULT_FRONT_JUMP_THRESHOLD
    });
    let encoded = serde_json::to_string(&payload).ok()?;
    let raw = evaluate_importance_json(&encoded).ok()?;
    parse_layer2_importance(&raw)
}

fn prioritize_rows_via_layer2(rows: &[Value]) -> Option<Vec<Value>> {
    let payload = json!({
        "events": rows,
        "front_jump_threshold": DEFAULT_FRONT_JUMP_THRESHOLD
    });
    let encoded = serde_json::to_string(&payload).ok()?;
    let raw = prioritize_attention_json(&encoded).ok()?;
    let parsed = serde_json::from_str::<Value>(&raw).ok()?;
    if parsed.get("ok").and_then(Value::as_bool) != Some(true) {
        return None;
    }
    parsed
        .get("events")
        .and_then(Value::as_array)
        .map(|arr| arr.to_vec())
}

fn event_score(row: &Value) -> f64 {
    parse_f64(
        row.pointer("/importance/score")
            .or_else(|| row.get("score")),
    )
    .unwrap_or_else(|| {
        let sev = row
            .get("severity")
            .and_then(Value::as_str)
            .unwrap_or("info")
            .trim()
            .to_ascii_lowercase();
        if sev == "critical" {
            0.85
        } else if sev == "warn" {
            0.60
        } else {
            0.35
        }
    })
    .clamp(0.0, 1.0)
}

fn event_band_rank(row: &Value) -> i64 {
    let band = row
        .pointer("/importance/band")
        .and_then(Value::as_str)
        .or_else(|| row.get("band").and_then(Value::as_str))
        .unwrap_or("p4");
    let rank = band_rank(band);
    if rank > 0 {
        rank
    } else {
        let sev = row
            .get("severity")
            .and_then(Value::as_str)
            .unwrap_or("info")
            .trim()
            .to_ascii_lowercase();
        if sev == "critical" {
            band_rank("p1")
        } else if sev == "warn" {
            band_rank("p2")
        } else {
            band_rank("p4")
        }
    }
}
