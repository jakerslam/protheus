use crate::lane_contracts::{build_receipt_row, ClaimEvidenceRow};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

#[derive(Clone, Debug)]
struct TransitionPaths {
    latest_path: PathBuf,
    receipts_path: PathBuf,
    selector_path: PathBuf,
    benchmark_path: PathBuf,
    benchmark_latest_path: PathBuf,
    benchmark_report_path: PathBuf,
    memory_index_path: PathBuf,
    rust_crate_path: PathBuf,
}

#[derive(Clone, Debug)]
struct TransitionThresholds {
    min_speedup_for_cutover: f64,
    max_parity_error_count: i64,
    min_stable_runs_for_retirement: usize,
}

#[derive(Clone, Debug)]
struct TransitionBenchmark {
    mode: String,
    require_rust_transport: String,
}

#[derive(Clone, Debug)]
struct TransitionPolicy {
    version: String,
    enabled: bool,
    shadow_only: bool,
    paths: TransitionPaths,
    thresholds: TransitionThresholds,
    benchmark: TransitionBenchmark,
    raw_soak: Value,
}

#[derive(Clone, Debug)]
struct AutoDecision {
    backend: String,
    active_engine: String,
    eligible: bool,
    stable_runs: usize,
    avg_speedup: f64,
    max_parity_errors: i64,
    auto_reason: String,
}

fn now_iso() -> String {
    OffsetDateTime::now_utc()
        .format(&Rfc3339)
        .unwrap_or_else(|_| "1970-01-01T00:00:00Z".to_string())
}

fn parse_kv_args(args: &[String]) -> HashMap<String, String> {
    let mut out: HashMap<String, String> = HashMap::new();
    let mut idx = 0usize;
    while idx < args.len() {
        let token = &args[idx];
        if !token.starts_with("--") {
            idx += 1;
            continue;
        }
        let raw = token.trim_start_matches("--");
        if let Some(eq_idx) = raw.find('=') {
            out.insert(raw[..eq_idx].to_string(), raw[eq_idx + 1..].to_string());
            idx += 1;
            continue;
        }
        if idx + 1 < args.len() && !args[idx + 1].starts_with("--") {
            out.insert(raw.to_string(), args[idx + 1].clone());
            idx += 2;
            continue;
        }
        out.insert(raw.to_string(), "true".to_string());
        idx += 1;
    }
    out
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .trim()
        .chars()
        .take(max_len)
        .collect::<String>()
}

fn normalize_token(raw: &str, max_len: usize) -> String {
    clean_text(raw, max_len)
        .to_lowercase()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .trim_matches('_')
        .to_string()
}

fn to_bool(raw: Option<&Value>, fallback: bool) -> bool {
    match raw {
        Some(Value::Bool(v)) => *v,
        Some(Value::Number(v)) => v.as_i64().unwrap_or(0) != 0,
        Some(Value::String(v)) => {
            let norm = normalize_token(v, 20);
            if ["1", "true", "yes", "on"].contains(&norm.as_str()) {
                true
            } else if ["0", "false", "no", "off"].contains(&norm.as_str()) {
                false
            } else {
                fallback
            }
        }
        _ => fallback,
    }
}

fn clamp_i64(raw: Option<&Value>, min: i64, max: i64, fallback: i64) -> i64 {
    let base = match raw {
        Some(Value::Number(v)) => v.as_i64().unwrap_or(fallback),
        Some(Value::String(v)) => v.parse::<i64>().unwrap_or(fallback),
        _ => fallback,
    };
    base.max(min).min(max)
}

fn clamp_usize(raw: Option<&Value>, min: usize, max: usize, fallback: usize) -> usize {
    let base = match raw {
        Some(Value::Number(v)) => v.as_u64().unwrap_or(fallback as u64) as usize,
        Some(Value::String(v)) => v.parse::<usize>().unwrap_or(fallback),
        _ => fallback,
    };
    base.max(min).min(max)
}

fn parse_f64(raw: Option<&Value>, fallback: f64) -> f64 {
    match raw {
        Some(Value::Number(v)) => v.as_f64().unwrap_or(fallback),
        Some(Value::String(v)) => v.parse::<f64>().unwrap_or(fallback),
        _ => fallback,
    }
}

fn resolve_path(root: &Path, raw: Option<&Value>, fallback_rel: &str) -> PathBuf {
    let text = raw
        .and_then(|v| v.as_str())
        .map(|v| v.trim().to_string())
        .unwrap_or_default();
    if text.is_empty() {
        return root.join(fallback_rel);
    }
    let p = PathBuf::from(text);
    if p.is_absolute() {
        p
    } else {
        root.join(p)
    }
}

fn read_json(path: &Path, fallback: Value) -> Value {
    let Ok(raw) = fs::read_to_string(path) else {
        return fallback;
    };
    serde_json::from_str::<Value>(&raw).unwrap_or(fallback)
}

fn write_json_atomic(path: &Path, payload: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir_failed:{e}"))?;
    }
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|d| d.as_nanos())
            .unwrap_or(0)
    ));
    let body = serde_json::to_string_pretty(payload).map_err(|e| format!("encode_failed:{e}"))?;
    fs::write(&tmp, format!("{body}\n")).map_err(|e| format!("tmp_write_failed:{e}"))?;
    fs::rename(&tmp, path).map_err(|e| format!("rename_failed:{e}"))?;
    Ok(())
}

fn append_jsonl(path: &Path, payload: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| format!("mkdir_failed:{e}"))?;
    }
    let row = serde_json::to_string(payload).map_err(|e| format!("encode_failed:{e}"))?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("open_failed:{e}"))?;
    file.write_all(format!("{row}\n").as_bytes())
        .map_err(|e| format!("append_failed:{e}"))
}

fn stable_hash_text(raw: &str, len: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    let digest = hasher.finalize();
    let hex = hex::encode(digest);
    let keep = len.min(hex.len());
    hex[..keep].to_string()
}

fn default_policy(root: &Path) -> TransitionPolicy {
    TransitionPolicy {
        version: "1.0".to_string(),
        enabled: true,
        shadow_only: true,
        paths: TransitionPaths {
            latest_path: root.join("local/state/client/memory/rust_transition/latest.json"),
            receipts_path: root.join("local/state/client/memory/rust_transition/receipts.jsonl"),
            selector_path: root
                .join("local/state/client/memory/rust_transition/backend_selector.json"),
            benchmark_path: root
                .join("local/state/client/memory/rust_transition/benchmark_history.json"),
            benchmark_latest_path: root
                .join("local/state/client/memory/rust_transition/benchmark_latest.json"),
            benchmark_report_path: root.join("benchmarks/memory-stage1.md"),
            memory_index_path: root.join("client/memory/MEMORY_INDEX.md"),
            rust_crate_path: root.join("core/layer0/memory"),
        },
        thresholds: TransitionThresholds {
            min_speedup_for_cutover: 1.2,
            max_parity_error_count: 0,
            min_stable_runs_for_retirement: 10,
        },
        benchmark: TransitionBenchmark {
            mode: "probe_commands".to_string(),
            require_rust_transport: "any".to_string(),
        },
        raw_soak: json!({
            "enabled": true,
            "window_hours": 24,
            "max_window_hours": 48,
            "min_rows": 20,
            "min_pass_rate": 0.997,
            "max_fallback_trigger_count": 0,
            "max_restart_count": 2,
            "max_rust_p99_ms": 2000,
            "restart_history_path": "local/state/client/memory/rust_transition/daemon_restart_history.jsonl",
            "promotion_decisions_path": "local/state/client/memory/rust_transition/soak_promotion_decisions.jsonl"
        }),
    }
}

fn load_policy(root: &Path, policy_path: &Path) -> TransitionPolicy {
    let defaults = default_policy(root);
    let raw = read_json(policy_path, json!({}));
    let paths_raw = raw.get("paths").cloned().unwrap_or_else(|| json!({}));
    let thresholds_raw = raw.get("thresholds").cloned().unwrap_or_else(|| json!({}));
    let benchmark_raw = raw.get("benchmark").cloned().unwrap_or_else(|| json!({}));
    let soak_raw = raw
        .get("soak")
        .cloned()
        .unwrap_or(defaults.raw_soak.clone());

    TransitionPolicy {
        version: clean_text(
            raw.get("version")
                .and_then(Value::as_str)
                .unwrap_or(&defaults.version),
            32,
        ),
        enabled: to_bool(raw.get("enabled"), defaults.enabled),
        shadow_only: to_bool(raw.get("shadow_only"), defaults.shadow_only),
        paths: TransitionPaths {
            latest_path: resolve_path(
                root,
                paths_raw.get("latest_path"),
                "local/state/client/memory/rust_transition/latest.json",
            ),
            receipts_path: resolve_path(
                root,
                paths_raw.get("receipts_path"),
                "local/state/client/memory/rust_transition/receipts.jsonl",
            ),
            selector_path: resolve_path(
                root,
                paths_raw.get("selector_path"),
                "local/state/client/memory/rust_transition/backend_selector.json",
            ),
            benchmark_path: resolve_path(
                root,
                paths_raw.get("benchmark_path"),
                "local/state/client/memory/rust_transition/benchmark_history.json",
            ),
            benchmark_latest_path: resolve_path(
                root,
                paths_raw.get("benchmark_latest_path"),
                "local/state/client/memory/rust_transition/benchmark_latest.json",
            ),
            benchmark_report_path: resolve_path(
                root,
                paths_raw.get("benchmark_report_path"),
                "benchmarks/memory-stage1.md",
            ),
            memory_index_path: resolve_path(
                root,
                paths_raw.get("memory_index_path"),
                "client/memory/MEMORY_INDEX.md",
            ),
            rust_crate_path: resolve_path(
                root,
                paths_raw.get("rust_crate_path"),
                "core/layer0/memory",
            ),
        },
        thresholds: TransitionThresholds {
            min_speedup_for_cutover: parse_f64(
                thresholds_raw.get("min_speedup_for_cutover"),
                defaults.thresholds.min_speedup_for_cutover,
            ),
            max_parity_error_count: clamp_i64(
                thresholds_raw.get("max_parity_error_count"),
                0,
                1_000_000,
                defaults.thresholds.max_parity_error_count,
            ),
            min_stable_runs_for_retirement: clamp_usize(
                thresholds_raw.get("min_stable_runs_for_retirement"),
                1,
                1_000_000,
                defaults.thresholds.min_stable_runs_for_retirement,
            ),
        },
        benchmark: TransitionBenchmark {
            mode: normalize_token(
                benchmark_raw
                    .get("mode")
                    .and_then(Value::as_str)
                    .unwrap_or(&defaults.benchmark.mode),
                40,
            ),
            require_rust_transport: normalize_token(
                benchmark_raw
                    .get("require_rust_transport")
                    .and_then(Value::as_str)
                    .unwrap_or(&defaults.benchmark.require_rust_transport),
                20,
            ),
        },
        raw_soak: soak_raw,
    }
}

fn policy_scope_id(policy: &TransitionPolicy) -> String {
    stable_hash_text(
        &[
            clean_text(&policy.version, 32),
            clean_text(policy.paths.benchmark_path.to_string_lossy().as_ref(), 240),
            clean_text(
                policy
                    .paths
                    .benchmark_report_path
                    .to_string_lossy()
                    .as_ref(),
                240,
            ),
            clean_text(
                policy.paths.memory_index_path.to_string_lossy().as_ref(),
                240,
            ),
            clean_text(policy.paths.rust_crate_path.to_string_lossy().as_ref(), 240),
            clean_text(&policy.benchmark.mode, 40),
            clean_text(&policy.benchmark.require_rust_transport, 20),
        ]
        .join("|"),
        24,
    )
}

fn transition_claims(
    claim: &str,
    evidence: Vec<String>,
    lenses: Vec<&str>,
) -> Vec<ClaimEvidenceRow> {
    vec![ClaimEvidenceRow {
        claim: claim.to_string(),
        evidence,
        persona_lenses: lenses.into_iter().map(|v| v.to_string()).collect(),
    }]
}

fn write_transition_receipt(
    policy: &TransitionPolicy,
    payload: &Value,
    claims: &[ClaimEvidenceRow],
