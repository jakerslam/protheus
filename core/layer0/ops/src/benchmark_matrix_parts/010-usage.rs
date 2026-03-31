// SPDX-License-Identifier: Apache-2.0
use crate::{
    clean, deterministic_receipt_hash, now_iso, parse_args, run_runtime_efficiency_floor,
    status_runtime_efficiency_floor,
};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::hint::black_box;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};
use sysinfo::System;
use walkdir::WalkDir;

const LANE_ID: &str = "benchmark_matrix";
const DEFAULT_SNAPSHOT_REL: &str =
    "client/runtime/config/competitive_benchmark_snapshot_2026_02.json";
const TOP1_BENCHMARK_SNAPSHOT_REL: &str =
    "docs/client/reports/runtime_snapshots/ops/proof_pack/top1_benchmark_snapshot.json";
const STATE_LATEST_REL: &str = "local/state/ops/competitive_benchmark_matrix/latest.json";
const STATE_HISTORY_REL: &str = "local/state/ops/competitive_benchmark_matrix/history.jsonl";
const MIN_BAR_WIDTH: usize = 10;
const MAX_BAR_WIDTH: usize = 80;
const DEFAULT_BAR_WIDTH: usize = 44;
const SHARED_THROUGHPUT_SOURCE: &str = "live_hash_workload_v1_shared_pre_profile_baseline";
const SHARED_THROUGHPUT_SAMPLE_MS: u64 = 800;
const SHARED_THROUGHPUT_ROUNDS: usize = 5;
const SHARED_THROUGHPUT_WARMUP_ROUNDS: usize = 2;
const SHARED_THROUGHPUT_DEFAULT_UNCACHED: bool = true;
const SHARED_THROUGHPUT_WORK_FACTOR: u32 = 16;
const BENCHMARK_PREFLIGHT_ENABLED_DEFAULT: bool = true;
const BENCHMARK_PREFLIGHT_MAX_LOAD_PER_CORE_DEFAULT: f64 = 0.90;
const BENCHMARK_PREFLIGHT_MAX_NOISE_CV_PCT_DEFAULT: f64 = 12.5;
const BENCHMARK_PREFLIGHT_NOISE_SAMPLE_MS_DEFAULT: u64 = 250;
const BENCHMARK_PREFLIGHT_NOISE_ROUNDS_DEFAULT: usize = 3;
const SECURITY_MERGE_GUARD_SOURCE_REL: &str = "client/runtime/config/guard_check_registry.json";
const PLATFORM_ADAPTER_SOURCE_REL: &str = "client/runtime/config/platform_adaptation_channels.json";
const PROVIDER_ONBOARDING_SOURCE_REL: &str =
    "client/runtime/config/provider_onboarding_manifest.json";
const MODEL_RECOVERY_SOURCE_REL: &str = "client/runtime/config/model_health_auto_recovery_policy.json";
const DATA_CHANNELS_SOURCE_REL: &str = "client/runtime/config/data_channels_policy.json";
const PLUGIN_TRUST_POLICY_SOURCE_REL: &str =
    "client/runtime/config/signed_plugin_trust_marketplace_policy.json";

#[derive(Clone)]
struct ThroughputSampling {
    tasks_per_sec: f64,
    warmup_samples: Vec<f64>,
    measured_samples: Vec<f64>,
    stddev: f64,
    min: f64,
    max: f64,
    uncached: bool,
    workload_seed: String,
}

#[derive(Clone, Copy)]
struct BenchmarkPreflightConfig {
    enabled: bool,
    max_load_per_core: f64,
    max_noise_cv_pct: f64,
    noise_sample_ms: u64,
    noise_rounds: usize,
}

#[derive(Clone, Copy)]
struct Category {
    key: &'static str,
    label: &'static str,
    lower_is_better: bool,
    unit: &'static str,
}

const CATEGORIES: [Category; 7] = [
    Category {
        key: "cold_start_ms",
        label: "Cold Start Time (lower is better)",
        lower_is_better: true,
        unit: "ms",
    },
    Category {
        key: "idle_memory_mb",
        label: "Idle Memory Usage (lower is better)",
        lower_is_better: true,
        unit: "MB",
    },
    Category {
        key: "install_size_mb",
        label: "Install Size (lower is better)",
        lower_is_better: true,
        unit: "MB",
    },
    Category {
        key: "tasks_per_sec",
        label: "Throughput (ops/sec, higher is better)",
        lower_is_better: false,
        unit: "ops/sec",
    },
    Category {
        key: "security_systems",
        label: "Security Checks (merge guard, higher is better)",
        lower_is_better: false,
        unit: "count",
    },
    Category {
        key: "channel_adapters",
        label: "Platform Adapters (higher is better)",
        lower_is_better: false,
        unit: "count",
    },
    Category {
        key: "llm_providers",
        label: "LLM Providers (onboarded, higher is better)",
        lower_is_better: false,
        unit: "count",
    },
];

fn usage() {
    println!("Usage:");
    println!(
        "  protheus-ops benchmark-matrix run [--snapshot=<path>] [--refresh-runtime=1|0] [--bar-width=44] [--throughput-uncached=1|0] [--benchmark-preflight=1|0] [--preflight-max-load-per-core=0.90] [--preflight-max-noise-cv-pct=12.5] [--preflight-noise-sample-ms=250] [--preflight-noise-rounds=3]"
    );
    println!(
        "  protheus-ops benchmark-matrix status [--snapshot=<path>] [--refresh-runtime=1|0] [--bar-width=44] [--throughput-uncached=1|0] [--benchmark-preflight=1|0] [--preflight-max-load-per-core=0.90] [--preflight-max-noise-cv-pct=12.5] [--preflight-noise-sample-ms=250] [--preflight-noise-rounds=3]"
    );
}

fn parse_bool_flag(raw: Option<&str>, fallback: bool) -> bool {
    match raw.map(|v| v.trim().to_ascii_lowercase()) {
        Some(v) if matches!(v.as_str(), "1" | "true" | "yes" | "on") => true,
        Some(v) if matches!(v.as_str(), "0" | "false" | "no" | "off") => false,
        _ => fallback,
    }
}

fn parse_bar_width(raw: Option<&str>) -> usize {
    let n = raw
        .and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(DEFAULT_BAR_WIDTH);
    n.clamp(MIN_BAR_WIDTH, MAX_BAR_WIDTH)
}

fn parse_u64_flag(raw: Option<&str>, fallback: u64, min: u64, max: u64) -> u64 {
    raw.and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn parse_usize_flag(raw: Option<&str>, fallback: usize, min: usize, max: usize) -> usize {
    raw.and_then(|v| v.trim().parse::<usize>().ok())
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn parse_f64_flag(raw: Option<&str>, fallback: f64, min: f64, max: f64) -> f64 {
    raw.and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn read_json(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path)
        .map_err(|err| format!("read_json_failed:{}:{err}", path.display()))?;
    serde_json::from_str::<Value>(&raw)
        .map_err(|err| format!("parse_json_failed:{}:{err}", path.display()))
}

fn get_f64(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64)
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create_dir_failed:{}:{err}", parent.display()))?;
    }
    let tmp = path.with_extension("tmp");
    let payload = serde_json::to_string_pretty(value)
        .map_err(|err| format!("encode_json_failed:{}:{err}", path.display()))?;
    fs::write(&tmp, format!("{payload}\n"))
        .map_err(|err| format!("write_tmp_failed:{}:{err}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|err| format!("rename_tmp_failed:{}:{err}", path.display()))
}

fn append_jsonl(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create_dir_failed:{}:{err}", parent.display()))?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("open_jsonl_failed:{}:{err}", path.display()))?;
    let line = serde_json::to_string(value)
        .map_err(|err| format!("encode_jsonl_failed:{}:{err}", path.display()))?;
    writeln!(file, "{line}").map_err(|err| format!("append_jsonl_failed:{}:{err}", path.display()))
}

fn count_guard_checks(root: &Path) -> Result<f64, String> {
    let payload = read_json(&root.join(SECURITY_MERGE_GUARD_SOURCE_REL))?;
    let count = payload
        .get("merge_guard")
        .and_then(|v| v.get("checks"))
        .and_then(Value::as_array)
        .map(|rows| rows.len() as f64)
        .unwrap_or(0.0);
    Ok(count)
}

fn count_channel_adapters(root: &Path) -> Result<f64, String> {
    let payload = read_json(&root.join(PLATFORM_ADAPTER_SOURCE_REL))?;
    let count = payload
        .get("channels")
        .and_then(Value::as_array)
        .map(|rows| rows.len() as f64)
        .unwrap_or(0.0);
    Ok(count)
}

fn count_llm_providers(root: &Path) -> Result<f64, String> {
    let mut providers = BTreeSet::<String>::new();

    let onboarding = read_json(&root.join(PROVIDER_ONBOARDING_SOURCE_REL))?;
    if let Some(entries) = onboarding.get("providers").and_then(Value::as_object) {
        for record in entries.values() {
            if let Some(provider_key) = record.get("provider_key").and_then(Value::as_str) {
                let normalized = provider_key.trim().to_ascii_lowercase();
                if !normalized.is_empty() {
                    providers.insert(normalized);
                }
            }
        }
    }

    let recovery = read_json(&root.join(MODEL_RECOVERY_SOURCE_REL))?;
    if let Some(items) = recovery.get("providers").and_then(Value::as_array) {
        for item in items {
            if let Some(name) = item.as_str() {
                let normalized = name.trim().to_ascii_lowercase();
                if !normalized.is_empty() {
                    providers.insert(normalized);
                }
            }
        }
    }

    Ok(providers.len() as f64)
}

fn count_data_channels(root: &Path) -> Result<f64, String> {
    let payload = read_json(&root.join(DATA_CHANNELS_SOURCE_REL))?;
    let count = payload
        .get("channels")
        .and_then(Value::as_object)
        .map(|rows| rows.len() as f64)
        .unwrap_or(0.0);
    Ok(count)
}

fn count_plugin_marketplace_checks(root: &Path) -> Result<f64, String> {
    let payload = read_json(&root.join(PLUGIN_TRUST_POLICY_SOURCE_REL))?;
    let count = payload
        .get("checks")
        .and_then(Value::as_array)
        .map(|rows| rows.len() as f64)
        .unwrap_or(0.0);
    Ok(count)
}

fn count_policy_checks_total(root: &Path) -> Result<f64, String> {
    let config_root = root.join("client/runtime/config");
    let mut total = 0usize;
    let entries = fs::read_dir(&config_root)
        .map_err(|err| format!("read_dir_failed:{}:{err}", config_root.display()))?;
    for entry in entries {
        let entry =
            entry.map_err(|err| format!("read_dir_entry_failed:{}:{err}", config_root.display()))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let payload = match read_json(&path) {
            Ok(value) => value,
            Err(_) => continue,
        };
        if let Some(count) = payload
            .get("checks")
            .and_then(Value::as_array)
            .map(|rows| rows.len())
        {
            total = total.saturating_add(count);
        }
    }
    Ok(total as f64)
}

fn benchmark_counter_definitions() -> Value {
    json!({
        "security_systems": {
            "label": "security_merge_guard_checks",
            "source": SECURITY_MERGE_GUARD_SOURCE_REL,
            "path": "merge_guard.checks.length"
        },
        "channel_adapters": {
            "label": "platform_adapters",
            "source": PLATFORM_ADAPTER_SOURCE_REL,
            "path": "channels.length"
        },
        "llm_providers": {
            "label": "onboarded_llm_providers",
            "sources": [
                PROVIDER_ONBOARDING_SOURCE_REL,
                MODEL_RECOVERY_SOURCE_REL
            ],
            "path": "union(provider_onboarding_manifest.providers.*.provider_key, model_health_auto_recovery_policy.providers[])"
        },
        "data_channels": {
            "label": "runtime_data_channels",
            "source": DATA_CHANNELS_SOURCE_REL,
            "path": "channels.keys.length"
        },
        "plugin_marketplace_checks": {
            "label": "plugin_marketplace_guard_checks",
            "source": PLUGIN_TRUST_POLICY_SOURCE_REL,
            "path": "checks.length"
        },
        "security_policy_checks_total": {
            "label": "total_policy_checks",
            "source_glob": "client/runtime/config/*.json",
            "path": "sum(top_level.checks.length)"
        }
    })
}

fn extract_runtime_metrics(runtime_json: &Value) -> Option<(f64, f64, f64)> {
    let latest = runtime_json
        .get("latest")
        .cloned()
        .unwrap_or_else(|| runtime_json.clone());
    let metrics = latest.get("metrics")?;
    let cold_start_ms =
        get_f64(metrics, "cold_start_p50_ms").or_else(|| get_f64(metrics, "cold_start_p95_ms"))?;
    let idle_memory_mb =
        get_f64(metrics, "idle_rss_p50_mb").or_else(|| get_f64(metrics, "idle_rss_p95_mb"))?;
    let install_size_mb = get_f64(metrics, "install_artifact_total_mb")
        .or_else(|| get_f64(metrics, "full_install_total_mb"))?;
    Some((cold_start_ms, idle_memory_mb, install_size_mb))
}

fn extract_top1_snapshot_metrics(snapshot_json: &Value) -> Option<(f64, f64, f64)> {
    let metrics = snapshot_json.get("metrics")?;
    let cold_start_ms = get_f64(metrics, "cold_start_ms")?;
    let idle_memory_mb = get_f64(metrics, "idle_rss_mb")?;
    let install_size_mb = get_f64(metrics, "install_size_mb")?;
    Some((cold_start_ms, idle_memory_mb, install_size_mb))
}

fn path_size_mb(root: &Path, rel: &str) -> f64 {
    let abs = root.join(rel);
    if !abs.exists() {
        return 0.0;
    }
    if abs.is_file() {
        return fs::metadata(abs)
            .map(|m| m.len() as f64 / (1024.0 * 1024.0))
            .unwrap_or(0.0);
    }
    let mut bytes = 0u64;
    for entry in WalkDir::new(abs).into_iter().flatten() {
        if let Ok(meta) = entry.metadata() {
            if meta.is_file() {
                bytes = bytes.saturating_add(meta.len());
            }
        }
    }
    bytes as f64 / (1024.0 * 1024.0)
}

fn local_full_install_probe_mb(root: &Path) -> Option<f64> {
    let mut paths = vec![
        "node_modules".to_string(),
        "client/runtime".to_string(),
        "core/layer0/ops".to_string(),
    ];

    for rel in [
        "target/x86_64-unknown-linux-musl/release/protheusd",
        "target/release/protheusd",
        "target/debug/protheusd",
    ] {
        if root.join(rel).exists() {
            paths.push(rel.to_string());
            break;
        }
    }

    let total: f64 = paths.into_iter().map(|rel| path_size_mb(root, &rel)).sum();
    if total <= 0.0 {
        None
    } else {
        Some((total * 1000.0).round() / 1000.0)
    }
}

fn locate_binary(root: &Path, candidates: &[&str]) -> Option<String> {
    candidates
        .iter()
        .map(|rel| root.join(rel))
        .find(|path| path.exists())
        .map(|path| path.to_string_lossy().to_string())
}

fn command_elapsed_ms(program: &str, args: &[&str]) -> Result<f64, String> {
    let started = Instant::now();
    let status = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|err| format!("probe_spawn_failed:{program}:{err}"))?;
    if !status.success() {
        return Err(format!(
            "probe_exit_failed:{program}:{}",
            status.code().unwrap_or(1)
        ));
    }
    Ok(started.elapsed().as_secs_f64() * 1000.0)
}

fn percentile(sorted: &[f64], q: f64) -> f64 {
    if sorted.is_empty() {
        return 0.0;
    }
    let quantile = q.clamp(0.0, 1.0);
    let idx = ((sorted.len() as f64 * quantile).ceil() as usize)
        .saturating_sub(1)
        .min(sorted.len().saturating_sub(1));
    sorted[idx]
}

fn sample_command_quantiles_ms(
    program: &str,
    args: &[&str],
    warmup_runs: usize,
    samples: usize,
) -> Result<(f64, f64, f64), String> {
    for _ in 0..warmup_runs {
        let _ = command_elapsed_ms(program, args)?;
    }
    let mut rows = Vec::new();
    for _ in 0..samples.max(1) {
        rows.push(command_elapsed_ms(program, args)?);
    }
    rows.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Ok((
        percentile(&rows, 0.50),
        percentile(&rows, 0.95),
        percentile(&rows, 0.99),
    ))
}

fn sample_dual_command_quantiles_ms(
    program_a: &str,
    args_a: &[&str],
    program_b: &str,
    args_b: &[&str],
    warmup_runs: usize,
    samples: usize,
) -> Result<((f64, f64, f64), (f64, f64, f64)), String> {
    for _ in 0..warmup_runs {
        let _ = command_elapsed_ms(program_a, args_a)?;
        let _ = command_elapsed_ms(program_b, args_b)?;
    }

    let mut rows_a = Vec::new();
    let mut rows_b = Vec::new();
    for idx in 0..samples.max(1) {
        if idx % 2 == 0 {
            rows_a.push(command_elapsed_ms(program_a, args_a)?);
            rows_b.push(command_elapsed_ms(program_b, args_b)?);
        } else {
            rows_b.push(command_elapsed_ms(program_b, args_b)?);
            rows_a.push(command_elapsed_ms(program_a, args_a)?);
        }
    }
    rows_a.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    rows_b.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
    Ok((
        (
            percentile(&rows_a, 0.50),
            percentile(&rows_a, 0.95),
            percentile(&rows_a, 0.99),
        ),
        (
            percentile(&rows_b, 0.50),
            percentile(&rows_b, 0.95),
            percentile(&rows_b, 0.99),
        ),
    ))
}

fn sample_child_rss_mb(program: &str, args: &[&str]) -> Result<f64, String> {
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .map_err(|err| format!("rss_spawn_failed:{program}:{err}"))?;
    thread::sleep(Duration::from_millis(80));
    let pid = child.id().to_string();
    let out = Command::new("ps")
        .args(["-o", "rss=", "-p", &pid])
        .stdin(Stdio::null())
        .output()
        .map_err(|err| format!("rss_ps_failed:{err}"))?;
    let _ = child.kill();
    let _ = child.wait();
    if !out.status.success() {
        return Err(format!(
            "rss_ps_exit_failed:{}",
            out.status.code().unwrap_or(1)
        ));
    }
    let kib = String::from_utf8_lossy(&out.stdout)
        .split_whitespace()
        .next()
        .and_then(|v| v.parse::<f64>().ok())
        .ok_or_else(|| "rss_parse_failed".to_string())?;
    Ok(kib / 1024.0)
}
