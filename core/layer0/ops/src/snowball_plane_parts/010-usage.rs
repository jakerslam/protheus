// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::snowball_plane (authoritative)

use crate::directive_kernel;
use crate::v8_kernel::{
    append_jsonl, attach_conduit, build_conduit_enforcement, conduit_bypass_requested,
    load_json_or, parse_bool, parse_u64, read_json, scoped_state_root, sha256_hex_str, write_json,
    write_receipt,
};
use crate::{clean, parse_args};
use serde_json::{json, Value};
use std::collections::{BTreeMap, HashSet};
use std::fs;
use std::path::{Component, Path, PathBuf};

const STATE_ENV: &str = "SNOWBALL_PLANE_STATE_ROOT";
const STATE_SCOPE: &str = "snowball_plane";
const CONTRACT_PATH: &str = "planes/contracts/apps/snowball_engine_contract_v1.json";
const DEFAULT_BENCHMARK_REPORT_PATH: &str =
    "docs/client/reports/benchmark_matrix_run_2026-03-06.json";

fn usage() {
    println!("Usage:");
    println!("  infring-ops snowball-plane status [--cycle-id=<id>]");
    println!(
        "  infring-ops snowball-plane start [--cycle-id=<id>] [--drops=<csv>] [--parallel=<n>] [--deps-json=<json>] [--strict=1|0]"
    );
    println!(
        "  infring-ops snowball-plane melt-refine|regress [--cycle-id=<id>] [--regression-suite=<id>] [--regression-pass=1|0] [--strict=1|0]"
    );
    println!("  infring-ops snowball-plane compact [--cycle-id=<id>] [--strict=1|0]");
    println!(
        "      compact flags: [--benchmark-report=<path>] [--assimilations-json=<json>] [--reliability-before=<f>] [--reliability-after=<f>]"
    );
    println!(
        "  infring-ops snowball-plane fitness-review [--cycle-id=<id>] [--benchmark-report=<path>] [--assimilations-json=<json>] [--reliability-before=<f>] [--reliability-after=<f>] [--strict=1|0]"
    );
    println!("  infring-ops snowball-plane archive-discarded [--cycle-id=<id>] [--strict=1|0]");
    println!(
        "  infring-ops snowball-plane publish-benchmarks [--cycle-id=<id>] [--benchmark-report=<path>] [--readme-path=<path>] [--strict=1|0]"
    );
    println!(
        "  infring-ops snowball-plane promote [--cycle-id=<id>] [--allow-neutral=1|0] [--neutral-justification=<text>] [--strict=1|0]"
    );
    println!(
        "  infring-ops snowball-plane prime-update [--cycle-id=<id>] [--directive=<text>] [--signer=<id>] [--strict=1|0]"
    );
    println!(
        "  infring-ops snowball-plane backlog-pack [--cycle-id=<id>] [--unresolved-json=<json>] [--strict=1|0]"
    );
    println!(
        "  infring-ops snowball-plane control --op=<pause|resume|abort> [--cycle-id=<id>] [--strict=1|0]"
    );
}

fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn latest_path(root: &Path) -> PathBuf {
    state_root(root).join("latest.json")
}

fn cycles_path(root: &Path) -> PathBuf {
    state_root(root).join("cycles").join("registry.json")
}

fn snapshot_dir(root: &Path, cycle_id: &str) -> PathBuf {
    state_root(root).join("snapshots").join(cycle_id)
}

fn backlog_path(root: &Path, cycle_id: &str) -> PathBuf {
    state_root(root)
        .join("backlog")
        .join(format!("{cycle_id}-next.json"))
}

fn assimilation_plan_path(root: &Path, cycle_id: &str) -> PathBuf {
    state_root(root)
        .join("evolution")
        .join(format!("{cycle_id}-assimilation-plan.json"))
}

fn kept_path(root: &Path, cycle_id: &str) -> PathBuf {
    state_root(root)
        .join("evolution")
        .join(format!("{cycle_id}-kept.json"))
}

fn discarded_path(root: &Path, cycle_id: &str) -> PathBuf {
    state_root(root)
        .join("evolution")
        .join(format!("{cycle_id}-discarded.json"))
}

fn discarded_blob_index_path(root: &Path, cycle_id: &str) -> PathBuf {
    state_root(root)
        .join("blob_archive")
        .join("discarded_ideas")
        .join(format!("{cycle_id}-index.json"))
}

fn discarded_blob_dir(root: &Path, cycle_id: &str) -> PathBuf {
    state_root(root)
        .join("blob_archive")
        .join("discarded_ideas")
        .join(cycle_id)
}

fn prime_directive_compacted_state_path(root: &Path) -> PathBuf {
    state_root(root).join("prime_directive_compacted_state.json")
}

fn fitness_review_path(root: &Path, cycle_id: &str) -> PathBuf {
    state_root(root)
        .join("evolution")
        .join(format!("{cycle_id}-fitness-review.json"))
}

fn promotion_path(root: &Path, cycle_id: &str) -> PathBuf {
    state_root(root)
        .join("promotion")
        .join(format!("{cycle_id}.json"))
}

fn benchmark_publication_path(root: &Path, cycle_id: &str) -> PathBuf {
    state_root(root)
        .join("benchmark_publication")
        .join(format!("{cycle_id}.json"))
}

fn readme_path(root: &Path, parsed: &crate::ParsedArgs) -> PathBuf {
    resolve_arg_path(
        root,
        parsed.flags.get("readme-path"),
        "README.md",
        "README.md",
    )
}

fn resolve_arg_path(
    root: &Path,
    raw: Option<&String>,
    fallback: &str,
    allowed_root_prefix: &str,
) -> PathBuf {
    let configured = clean(raw.map(String::as_str).unwrap_or(fallback), 240);
    let candidate = PathBuf::from(if configured.is_empty() {
        fallback.to_string()
    } else {
        configured
    });
    if candidate
        .components()
        .any(|c| matches!(c, Component::ParentDir))
    {
        return root.join(fallback);
    }
    if candidate.is_absolute() {
        if candidate.starts_with(root) {
            candidate
        } else {
            root.join(fallback)
        }
    } else {
        let prefixed = root.join(&candidate);
        if prefixed.starts_with(root.join(allowed_root_prefix)) || prefixed.starts_with(root) {
            prefixed
        } else {
            root.join(fallback)
        }
    }
}

fn benchmark_report_path(root: &Path, parsed: &crate::ParsedArgs) -> PathBuf {
    resolve_arg_path(
        root,
        parsed.flags.get("benchmark-report"),
        DEFAULT_BENCHMARK_REPORT_PATH,
        "docs/client/reports",
    )
}

fn print_payload(payload: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(payload)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn emit(root: &Path, payload: Value) -> i32 {
    match write_receipt(root, STATE_ENV, STATE_SCOPE, payload) {
        Ok(out) => {
            print_payload(&out);
            if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                0
            } else {
                1
            }
        }
        Err(err) => {
            print_payload(&json!({
                "ok": false,
                "type": "snowball_plane_error",
                "error": clean(err, 240)
            }));
            1
        }
    }
}

fn parse_json_flag(raw: Option<&String>) -> Option<Value> {
    raw.and_then(|text| serde_json::from_str::<Value>(text).ok())
}

fn parse_f64(raw: Option<&String>, default: f64) -> f64 {
    raw.and_then(|text| text.trim().parse::<f64>().ok())
        .unwrap_or(default)
}

fn clean_id(raw: Option<&str>, fallback: &str) -> String {
    let mut out = String::new();
    if let Some(v) = raw {
        for ch in v.trim().chars() {
            if out.len() >= 96 {
                break;
            }
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                out.push(ch.to_ascii_lowercase());
            } else {
                out.push('-');
            }
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn parse_csv_unique(raw: Option<&String>, fallback: &[&str]) -> Vec<String> {
    let mut seen = HashSet::new();
    let mut out = Vec::<String>::new();
    let rows = raw
        .map(|v| v.split(',').map(str::to_string).collect::<Vec<_>>())
        .unwrap_or_else(|| fallback.iter().map(|v| v.to_string()).collect::<Vec<_>>());
    for row in rows {
        let item = clean(row, 80)
            .to_ascii_lowercase()
            .chars()
            .map(|ch| {
                if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                    ch
                } else {
                    '-'
                }
            })
            .collect::<String>()
            .trim_matches('-')
            .to_string();
        if item.is_empty() {
            continue;
        }
        if seen.insert(item.clone()) {
            out.push(item);
        }
        if out.len() >= 24 {
            break;
        }
    }
    if out.is_empty() {
        fallback.iter().map(|v| v.to_string()).collect()
    } else {
        out
    }
}

fn parse_mode_metrics(report: &Value, key: &str) -> Value {
    report
        .get(key)
        .and_then(Value::as_object)
        .map(|obj| {
            json!({
                "cold_start_ms": obj.get("cold_start_ms").and_then(Value::as_f64).unwrap_or(0.0),
                "idle_memory_mb": obj.get("idle_memory_mb").and_then(Value::as_f64).unwrap_or(0.0),
                "install_size_mb": obj.get("install_size_mb").and_then(Value::as_f64).unwrap_or(0.0),
                "tasks_per_sec": obj.get("tasks_per_sec").and_then(Value::as_f64).unwrap_or(0.0),
                "security_systems": obj.get("security_systems").and_then(Value::as_f64).unwrap_or(0.0),
                "channel_adapters": obj.get("channel_adapters").and_then(Value::as_f64).unwrap_or(0.0),
                "llm_providers": obj.get("llm_providers").and_then(Value::as_f64).unwrap_or(0.0)
            })
        })
        .unwrap_or_else(|| {
            json!({
                "cold_start_ms": 0.0,
                "idle_memory_mb": 0.0,
                "install_size_mb": 0.0,
                "tasks_per_sec": 0.0,
                "security_systems": 0.0,
                "channel_adapters": 0.0,
                "llm_providers": 0.0
            })
        })
}

fn benchmark_modes_from_report(report: &Value) -> Value {
    json!({
        "infring": parse_mode_metrics(report, "infring_measured"),
        "pure_workspace": parse_mode_metrics(report, "pure_workspace_measured"),
        "pure_workspace_tiny_max": parse_mode_metrics(report, "pure_workspace_tiny_max_measured")
    })
}

fn load_benchmark_modes(path: &Path) -> Value {
    read_json(path)
        .map(|v| benchmark_modes_from_report(&v))
        .unwrap_or_else(|| {
            json!({
                "infring": parse_mode_metrics(&Value::Null, ""),
                "pure_workspace": parse_mode_metrics(&Value::Null, ""),
                "pure_workspace_tiny_max": parse_mode_metrics(&Value::Null, "")
            })
        })
}

fn metric_from_mode(mode: &Value, key: &str) -> f64 {
    mode.get(key).and_then(Value::as_f64).unwrap_or(0.0)
}

fn mode_delta(before: &Value, after: &Value) -> Value {
    let cold_before = metric_from_mode(before, "cold_start_ms");
    let cold_after = metric_from_mode(after, "cold_start_ms");
    let idle_before = metric_from_mode(before, "idle_memory_mb");
    let idle_after = metric_from_mode(after, "idle_memory_mb");
    let install_before = metric_from_mode(before, "install_size_mb");
    let install_after = metric_from_mode(after, "install_size_mb");
    let throughput_before = metric_from_mode(before, "tasks_per_sec");
    let throughput_after = metric_from_mode(after, "tasks_per_sec");
    let cold_improved = cold_before > 0.0 && cold_after > 0.0 && cold_after < cold_before;
    let cold_regressed = cold_before > 0.0 && cold_after > 0.0 && cold_after > cold_before;
    let idle_improved = idle_before > 0.0 && idle_after > 0.0 && idle_after < idle_before;
    let idle_regressed = idle_before > 0.0 && idle_after > 0.0 && idle_after > idle_before;
    let install_improved =
        install_before > 0.0 && install_after > 0.0 && install_after < install_before;
    let install_regressed =
        install_before > 0.0 && install_after > 0.0 && install_after > install_before;
    let throughput_improved =
        throughput_before > 0.0 && throughput_after > 0.0 && throughput_after > throughput_before;
    let throughput_regressed =
        throughput_before > 0.0 && throughput_after > 0.0 && throughput_after < throughput_before;
    let improved_count = [
        cold_improved,
        idle_improved,
        install_improved,
        throughput_improved,
    ]
    .iter()
    .filter(|v| **v)
    .count();
    let regressed_count = [
        cold_regressed,
        idle_regressed,
        install_regressed,
        throughput_regressed,
    ]
    .iter()
    .filter(|v| **v)
    .count();
    json!({
        "cold_start_ms_before": cold_before,
        "cold_start_ms_after": cold_after,
        "idle_memory_mb_before": idle_before,
        "idle_memory_mb_after": idle_after,
        "install_size_mb_before": install_before,
        "install_size_mb_after": install_after,
        "tasks_per_sec_before": throughput_before,
        "tasks_per_sec_after": throughput_after,
        "cold_improved": cold_improved,
        "cold_regressed": cold_regressed,
        "idle_improved": idle_improved,
        "idle_regressed": idle_regressed,
        "install_improved": install_improved,
        "install_regressed": install_regressed,
        "throughput_improved": throughput_improved,
        "throughput_regressed": throughput_regressed,
        "improved_count": improved_count,
        "regressed_count": regressed_count
    })
}

fn benchmark_delta(before: &Value, after: &Value) -> Value {
    let infring_before = before.get("infring").cloned().unwrap_or(Value::Null);
    let infring_after = after.get("infring").cloned().unwrap_or(Value::Null);
    let pure_before = before.get("pure_workspace").cloned().unwrap_or(Value::Null);
    let pure_after = after.get("pure_workspace").cloned().unwrap_or(Value::Null);
    let tiny_before = before
        .get("pure_workspace_tiny_max")
        .cloned()
        .unwrap_or(Value::Null);
    let tiny_after = after
        .get("pure_workspace_tiny_max")
        .cloned()
        .unwrap_or(Value::Null);
    let infring = mode_delta(&infring_before, &infring_after);
    let pure = mode_delta(&pure_before, &pure_after);
    let tiny = mode_delta(&tiny_before, &tiny_after);
    let improved_total = infring
        .get("improved_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        + pure
            .get("improved_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
        + tiny
            .get("improved_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
    let regressed_total = infring
        .get("regressed_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        + pure
            .get("regressed_count")
            .and_then(Value::as_u64)
            .unwrap_or(0)
        + tiny
            .get("regressed_count")
            .and_then(Value::as_u64)
            .unwrap_or(0);
    json!({
        "infring": infring,
        "pure_workspace": pure,
        "pure_workspace_tiny_max": tiny,
        "improved_metric_count": improved_total,
        "regressed_metric_count": regressed_total
    })
}

fn load_assimilation_plan(root: &Path, cycle_id: &str, parsed: &crate::ParsedArgs) -> Vec<Value> {
    parse_json_flag(parsed.flags.get("assimilations-json"))
        .and_then(|v| v.as_array().cloned())
        .or_else(|| {
            read_json(&assimilation_plan_path(root, cycle_id))
                .and_then(|v| v.get("items").and_then(Value::as_array).cloned())
        })
        .unwrap_or_default()
}

fn first_failed_gate(gates: &Value) -> &'static str {
    let checks = [
        ("metrics", "metrics_no_gain"),
        ("tiny_pure", "tiny_pure_not_strengthened"),
        ("rsi_organism", "rsi_utility_not_improved"),
        ("tiny_hardware", "tiny_hardware_not_supported"),
        ("reliability", "reliability_regressed"),
    ];
    for (key, reason) in checks {
        if gates.get(key).and_then(Value::as_bool) == Some(false) {
            return reason;
        }
    }
    "unknown_rejection_reason"
}

fn score_candidate(gates: &Value, bench_delta: &Value) -> f64 {
    let improved = bench_delta
        .get("improved_metric_count")
        .and_then(Value::as_u64)
        .unwrap_or(0) as f64;
    let gate_score = [
        "metrics",
        "tiny_pure",
        "rsi_organism",
        "tiny_hardware",
        "reliability",
    ]
    .iter()
    .filter(|key| gates.get(**key).and_then(Value::as_bool) == Some(true))
    .count() as f64;
    ((gate_score / 5.0) * 70.0) + improved.min(10.0) * 3.0
}
