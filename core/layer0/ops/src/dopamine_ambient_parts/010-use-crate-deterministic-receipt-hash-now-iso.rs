// SPDX-License-Identifier: Apache-2.0
use crate::now_iso;
use base64::Engine;
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

#[derive(Debug, Clone)]
struct DopamineAmbientPolicy {
    enabled: bool,
    threshold_breach_only: bool,
    surface_levels: Vec<String>,
    push_attention_queue: bool,
    latest_path: PathBuf,
    receipts_path: PathBuf,
    runtime_script: PathBuf,
    status_path: PathBuf,
    history_path: PathBuf,
    policy_path: PathBuf,
}

fn usage() {
    eprintln!("Usage:");
    eprintln!(
        "  infring-ops dopamine-ambient closeout [--date=YYYY-MM-DD] [--run-context=<value>]"
    );
    eprintln!("  infring-ops dopamine-ambient status [--date=YYYY-MM-DD] [--run-context=<value>]");
    eprintln!("  infring-ops dopamine-ambient evaluate --summary-json=<json>|--summary-json-base64=<base64> [--date=YYYY-MM-DD] [--run-context=<value>]");
}

fn read_json(path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&raw).ok()
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

fn normalize_date(raw: Option<&str>) -> String {
    let value = clean_text(raw, 40);
    if value.len() == 10 && value.chars().nth(4) == Some('-') && value.chars().nth(7) == Some('-') {
        return value;
    }
    now_iso()[..10].to_string()
}

fn parse_json_payload(stdout: &str) -> Option<Value> {
    let raw = stdout.trim();
    if raw.is_empty() {
        return None;
    }
    if let Ok(payload) = serde_json::from_str::<Value>(raw) {
        return Some(payload);
    }
    for line in raw.lines().rev() {
        let trimmed = line.trim();
        if !trimmed.starts_with('{') {
            continue;
        }
        if let Ok(payload) = serde_json::from_str::<Value>(trimmed) {
            return Some(payload);
        }
    }
    None
}

fn load_policy(root: &Path) -> DopamineAmbientPolicy {
    let default_policy = root.join("config").join("mech_suit_mode_policy.json");
    let policy_path = std::env::var("MECH_SUIT_MODE_POLICY_PATH")
        .ok()
        .map(PathBuf::from)
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or(default_policy);
    let policy = read_json(&policy_path).unwrap_or_else(|| json!({}));
    let enabled = bool_from_env("MECH_SUIT_MODE_FORCE").unwrap_or_else(|| {
        policy
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true)
    });
    let eyes = policy.get("eyes");
    let dopamine = policy.get("dopamine");
    let state = policy.get("state");

    let surface_levels = dopamine
        .and_then(|v| v.get("surface_levels"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| row.trim().to_ascii_lowercase())
                .filter(|row| matches!(row.as_str(), "critical" | "warn" | "info"))
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
        .unwrap_or_else(|| vec!["warn".to_string(), "critical".to_string()]);

    DopamineAmbientPolicy {
        enabled,
        threshold_breach_only: dopamine
            .and_then(|v| v.get("threshold_breach_only"))
            .and_then(Value::as_bool)
            .unwrap_or(true),
        surface_levels,
        push_attention_queue: eyes
            .and_then(|v| v.get("push_attention_queue"))
            .and_then(Value::as_bool)
            .unwrap_or(true),
        latest_path: normalize_path(
            root,
            dopamine.and_then(|v| v.get("latest_path")),
            "local/state/dopamine/ambient/latest.json",
        ),
        receipts_path: normalize_path(
            root,
            dopamine.and_then(|v| v.get("receipts_path")),
            "local/state/dopamine/ambient/receipts.jsonl",
        ),
        runtime_script: normalize_path(
            root,
            dopamine.and_then(|v| v.get("runtime_script")),
            "client/cognition/habits/scripts/dopamine_ambient_snapshot.js",
        ),
        status_path: normalize_path(
            root,
            state.and_then(|v| v.get("status_path")),
            "local/state/ops/mech_suit_mode/latest.json",
        ),
        history_path: normalize_path(
            root,
            state.and_then(|v| v.get("history_path")),
            "local/state/ops/mech_suit_mode/history.jsonl",
        ),
        policy_path,
    }
}

fn summary_number(summary: &Value, key: &str) -> f64 {
    parse_optional_number(summary.get(key)).unwrap_or(0.0)
}

fn classify_threshold(summary: &Value) -> (String, bool, Vec<String>) {
    let mut reasons = Vec::<String>::new();
    let directive_pain_active = summary
        .get("directive_pain")
        .and_then(|v| v.get("active"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let sds = summary_number(summary, "sds");
    let drift_minutes = summary_number(summary, "drift_minutes");

    if directive_pain_active {
        reasons.push("directive_pain_active".to_string());
    }
    if sds <= 0.0 {
        reasons.push("sds_non_positive".to_string());
    }
    if drift_minutes >= 120.0 {
        reasons.push("drift_over_threshold".to_string());
    }

    let severity = if directive_pain_active {
        "critical"
    } else if sds <= 0.0 || drift_minutes >= 120.0 {
        "warn"
    } else {
        "info"
    };
    (severity.to_string(), !reasons.is_empty(), reasons)
}

fn parse_summary_from_flags(flags: &BTreeMap<String, String>) -> Result<Option<Value>, String> {
    if let Some(raw) = flags.get("summary-json-base64") {
        let bytes = base64::engine::general_purpose::STANDARD
            .decode(raw.as_bytes())
            .map_err(|err| format!("summary_json_base64_invalid:{err}"))?;
        let text =
            String::from_utf8(bytes).map_err(|err| format!("summary_json_utf8_invalid:{err}"))?;
        let summary = serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("summary_json_invalid:{err}"))?;
        return Ok(Some(summary));
    }
    if let Some(raw) = flags.get("summary-json") {
        let summary = serde_json::from_str::<Value>(raw)
            .map_err(|err| format!("summary_json_invalid:{err}"))?;
        return Ok(Some(summary));
    }
    Ok(None)
}
