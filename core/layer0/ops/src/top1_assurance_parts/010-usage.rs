// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::top1_assurance (authoritative)

use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso, parse_args, status_runtime_efficiency_floor};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::ffi::OsStr;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, Instant};

const LANE_ID: &str = "top1_assurance";
const DEFAULT_POLICY_REL: &str = "client/runtime/config/top1_assurance_policy.json";
const PROVIDER_FAMILY_CONTRACT_TARGETS: &[&str] =
    &["anthropic", "fal", "google", "minimax", "moonshot"];

#[derive(Debug, Clone)]
struct ProofCoveragePolicy {
    map_path: String,
    min_proven_ratio: f64,
    check_toolchains_default: bool,
}

#[derive(Debug, Clone)]
struct ProofVmPolicy {
    dockerfile_path: String,
    replay_script_path: String,
    manifest_path: String,
}

#[derive(Debug, Clone)]
struct SizeGatePolicy {
    binary_path: String,
    min_mb: f64,
    max_mb: f64,
    require_static: bool,
}

#[derive(Debug, Clone)]
struct BenchmarkThresholdPolicy {
    benchmark_path: String,
    cold_start_max_ms: f64,
    idle_rss_max_mb: f64,
    tasks_per_sec_min: f64,
    sample_ms: u64,
}

#[derive(Debug, Clone)]
struct ComparisonMatrixPolicy {
    snapshot_path: String,
    output_path: String,
}

#[derive(Debug, Clone)]
struct OutputPolicy {
    latest_path: String,
    history_path: String,
}

#[derive(Debug, Clone)]
struct Top1Policy {
    version: String,
    strict_default: bool,
    proof_coverage: ProofCoveragePolicy,
    proof_vm: ProofVmPolicy,
    size_gate: SizeGatePolicy,
    benchmark: BenchmarkThresholdPolicy,
    comparison: ComparisonMatrixPolicy,
    outputs: OutputPolicy,
}

fn usage() {
    println!("Usage:");
    println!("  infring-ops top1-assurance status");
    println!("  infring-ops top1-assurance proof-coverage [--strict=1|0] [--check-toolchains=1|0] [--execute-proofs=1|0] [--execute-optional-proofs=1|0]");
    println!("  infring-ops top1-assurance proof-vm [--strict=1|0] [--write-manifest=1|0]");
    println!("  infring-ops top1-assurance size-gate [--strict=1|0] [--binary-path=<path>] [--min-mb=<n>] [--max-mb=<n>]");
    println!("  infring-ops top1-assurance benchmark-thresholds [--strict=1|0] [--benchmark-path=<path>] [--sample-ms=<n>] [--refresh=1|0]");
    println!("  infring-ops top1-assurance comparison-matrix [--strict=1|0] [--snapshot-path=<path>] [--output-path=<path>] [--apply=1|0]");
    println!("  infring-ops top1-assurance run-all [--strict=1|0]");
    println!(
        "  provider-family contract targets: {}",
        PROVIDER_FAMILY_CONTRACT_TARGETS.join(",")
    );
}

fn parse_bool(raw: Option<&String>, fallback: bool) -> bool {
    lane_utils::parse_bool(raw.map(String::as_str), fallback)
}

fn parse_f64(raw: Option<&String>, fallback: f64, lo: f64, hi: f64) -> f64 {
    let parsed = raw
        .and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(fallback);
    if !parsed.is_finite() {
        return fallback;
    }
    parsed.clamp(lo, hi)
}

fn parse_u64(raw: Option<&String>, fallback: u64, lo: u64, hi: u64) -> u64 {
    raw.and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(fallback)
        .clamp(lo, hi)
}

fn normalize_rel(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn rel_path(root: &Path, path: &Path) -> String {
    let relative = lane_utils::rel_path(root, path);
    if relative == normalize_rel(path) {
        normalize_rel(path)
    } else {
        relative
    }
}

fn read_json(path: &Path) -> Option<Value> {
    lane_utils::read_json(path)
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    lane_utils::write_json(path, value)
}

fn append_jsonl(path: &Path, value: &Value) -> Result<(), String> {
    lane_utils::append_jsonl(path, value)
}

fn sha256_file(path: &Path) -> Result<String, String> {
    let data = fs::read(path).map_err(|err| format!("read_failed:{}:{err}", path.display()))?;
    Ok(hex::encode(Sha256::digest(data)))
}

fn run_command_with<P, S>(program: P, args: &[S]) -> Value
where
    P: AsRef<OsStr>,
    S: AsRef<OsStr>,
{
    let started = Instant::now();
    let out = Command::new(program)
        .args(args.iter().map(|arg| arg.as_ref()))
        .output();
    match out {
        Ok(run) => {
            let stdout = String::from_utf8_lossy(&run.stdout)
                .trim()
                .chars()
                .take(600)
                .collect::<String>();
            let stderr = String::from_utf8_lossy(&run.stderr)
                .trim()
                .chars()
                .take(600)
                .collect::<String>();
            json!({
                "ok": run.status.success(),
                "status": run.status.code().unwrap_or(1),
                "elapsed_ms": started.elapsed().as_millis(),
                "stdout": stdout,
                "stderr": stderr
            })
        }
        Err(err) => json!({
            "ok": false,
            "status": 1,
            "elapsed_ms": started.elapsed().as_millis(),
            "spawn_error": err.to_string()
        }),
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var_os("HOME")
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
}

fn push_toolchain_candidate(
    rows: &mut Vec<(PathBuf, Vec<String>, &'static str)>,
    program: PathBuf,
    args: Vec<String>,
    source: &'static str,
) {
    if rows.iter().any(|(existing_program, existing_args, _)| {
        existing_program == &program && existing_args == &args
    }) {
        return;
    }
    rows.push((program, args, source));
}

fn toolchain_candidates(id: &str) -> Vec<(PathBuf, Vec<String>, &'static str)> {
    let mut rows = Vec::<(PathBuf, Vec<String>, &'static str)>::new();
    match id {
        "kani_toolchain" => {
            push_toolchain_candidate(
                &mut rows,
                PathBuf::from("cargo"),
                vec!["kani".to_string(), "--version".to_string()],
                "path:cargo",
            );
            push_toolchain_candidate(
                &mut rows,
                PathBuf::from("cargo-kani"),
                vec!["--version".to_string()],
                "path:cargo-kani",
            );
            if let Some(home) = home_dir() {
                push_toolchain_candidate(
                    &mut rows,
                    home.join(".cargo/bin/cargo"),
                    vec!["kani".to_string(), "--version".to_string()],
                    "home:.cargo/bin/cargo",
                );
                push_toolchain_candidate(
                    &mut rows,
                    home.join(".cargo/bin/cargo-kani"),
                    vec!["--version".to_string()],
                    "home:.cargo/bin/cargo-kani",
                );
            }
        }
        "prusti_toolchain" => {
            push_toolchain_candidate(
                &mut rows,
                PathBuf::from("prusti-rustc"),
                vec!["--version".to_string()],
                "path:prusti-rustc",
            );
            if let Some(home) = home_dir() {
                push_toolchain_candidate(
                    &mut rows,
                    home.join(".cargo/bin/prusti-rustc"),
                    vec!["--version".to_string()],
                    "home:.cargo/bin/prusti-rustc",
                );
            }
        }
        "lean_toolchain" => {
            push_toolchain_candidate(
                &mut rows,
                PathBuf::from("lean"),
                vec!["--version".to_string()],
                "path:lean",
            );
            if let Some(home) = home_dir() {
                push_toolchain_candidate(
                    &mut rows,
                    home.join(".elan/bin/lean"),
                    vec!["--version".to_string()],
                    "home:.elan/bin/lean",
                );
            }
        }
        _ => {}
    }
    rows
}

fn run_toolchain_check(id: &str) -> Value {
    let mut attempts = Vec::<Value>::new();
    for (program, args, source) in toolchain_candidates(id) {
        let run = run_command_with(&program, &args);
        let ok = run.get("ok").and_then(Value::as_bool).unwrap_or(false);
        attempts.push(json!({
            "program": program.display().to_string(),
            "args": args,
            "source": source,
            "run": run
        }));
        if ok {
            return json!({
                "ok": true,
                "resolved_bin": program.display().to_string(),
                "resolution_source": source,
                "attempts": attempts
            });
        }
    }
    json!({
        "ok": false,
        "resolved_bin": Value::Null,
        "attempts": attempts
    })
}

fn default_policy() -> Top1Policy {
    Top1Policy {
        version: "1.0".to_string(),
        strict_default: true,
        proof_coverage: ProofCoveragePolicy {
            map_path: "proofs/layer0/core_formal_coverage_map.json".to_string(),
            min_proven_ratio: 0.20,
            check_toolchains_default: true,
        },
        proof_vm: ProofVmPolicy {
            dockerfile_path: "proofs/layer0/ProofVM.Dockerfile".to_string(),
            replay_script_path: "proofs/layer0/replay.sh".to_string(),
            manifest_path: "local/state/ops/top1_assurance/proof_vm_manifest.json".to_string(),
        },
        size_gate: SizeGatePolicy {
            binary_path: "target/x86_64-unknown-linux-musl/release/infringd".to_string(),
            min_mb: 0.0,
            max_mb: 35.0,
            require_static: true,
        },
        benchmark: BenchmarkThresholdPolicy {
            benchmark_path: "local/state/ops/top1_assurance/benchmark_latest.json".to_string(),
            cold_start_max_ms: 80.0,
            idle_rss_max_mb: 25.0,
            tasks_per_sec_min: 5000.0,
            sample_ms: 800,
        },
        comparison: ComparisonMatrixPolicy {
            snapshot_path: "client/runtime/config/competitive_benchmark_snapshot_2026_02.json"
                .to_string(),
            output_path: "docs/comparison/infring_vs_x.md".to_string(),
        },
        outputs: OutputPolicy {
            latest_path: "local/state/ops/top1_assurance/latest.json".to_string(),
            history_path: "local/state/ops/top1_assurance/history.jsonl".to_string(),
        },
    }
}
