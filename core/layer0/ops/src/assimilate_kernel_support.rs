// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

use crate::contract_lane_utils as lane_utils;
use crate::now_iso;

pub const DEFAULT_SHOWCASE_DURATION_MS: u64 = 10_000;
pub const DEFAULT_REALTIME_DURATION_MS: u64 = 0;
const DEFAULT_PREWARM_TTL_MS: i64 = 5 * 60 * 1000;
const BAR_WIDTH: usize = 64;
const FILLED_CHAR: char = '█';
const EMPTY_CHAR: char = '░';
const STATE_DIR_REL: &str = "local/state/tools/assimilate";
const PREWARM_STATE_REL: &str = "local/state/tools/assimilate/prewarm.json";
const METRICS_STATE_REL: &str = "local/state/tools/assimilate/metrics.json";

#[derive(Clone, Copy)]
pub struct Stage {
    pub percent: u32,
    pub label: &'static str,
    pub weight: f64,
}

pub const STAGES: [Stage; 5] = [
    Stage {
        percent: 20,
        label: "Spinning up swarm (5,000 agents)",
        weight: 0.2,
    },
    Stage {
        percent: 50,
        label: "Parallel analysis (manifest + docs)",
        weight: 0.3,
    },
    Stage {
        percent: 80,
        label: "Building bridges & adapters",
        weight: 0.3,
    },
    Stage {
        percent: 95,
        label: "Validating + signing receipts",
        weight: 0.15,
    },
    Stage {
        percent: 100,
        label: "Assimilation complete. Ready to use.",
        weight: 0.05,
    },
];

#[derive(Debug, Default)]
pub struct Options {
    pub target: String,
    pub duration_ms: Option<u64>,
    pub showcase: bool,
    pub scaffold_payload: bool,
    pub json: bool,
    pub prewarm: bool,
    pub core_domain: String,
    pub core_args_base64: String,
    pub help: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Route {
    pub domain: String,
    pub args: Vec<String>,
}

#[derive(Debug)]
pub struct RunResult {
    pub status: i32,
    pub latency_ms: u64,
    pub payload: Option<Value>,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TargetMetrics {
    pub count: u64,
    pub ok_count: u64,
    pub fail_count: u64,
    pub last_latency_ms: u64,
    pub p50_ms: u64,
    pub p95_ms: u64,
    pub updated_at: String,
    #[serde(default)]
    pub latencies_ms: Vec<u64>,
}

impl Default for TargetMetrics {
    fn default() -> Self {
        Self {
            count: 0,
            ok_count: 0,
            fail_count: 0,
            last_latency_ms: 0,
            p50_ms: 0,
            p95_ms: 0,
            updated_at: now_iso(),
            latencies_ms: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct MetricsState {
    schema_version: String,
    #[serde(default)]
    targets: BTreeMap<String, TargetMetrics>,
}

impl Default for MetricsState {
    fn default() -> Self {
        Self {
            schema_version: "assimilate_metrics_v1".to_string(),
            targets: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct PrewarmState {
    ts_ms: i64,
    ts: String,
}

impl Default for PrewarmState {
    fn default() -> Self {
        Self {
            ts_ms: 0,
            ts: now_iso(),
        }
    }
}

pub fn usage() {
    println!("Usage: infring assimilate <target> [--payload-base64=...] [--strict=1] [--showcase=1] [--duration-ms=<n>] [--json=1] [--scaffold-payload=1]");
    println!();
    println!("Known targets route to governed core bridge lanes. Unknown targets run local simulation mode.");
}

fn parse_bool_flag(raw: Option<&str>, fallback: bool) -> bool {
    lane_utils::parse_bool(raw, fallback)
}

fn normalize_target(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if out.len() >= 120 {
            break;
        }
        if ch.is_control() {
            continue;
        }
        out.push(ch);
    }
    out.trim().to_string()
}

pub fn parse_args(argv: &[String]) -> Options {
    let mut out = Options {
        json: parse_bool_flag(std::env::var("PROTHEUS_GLOBAL_JSON").ok().as_deref(), false),
        prewarm: true,
        ..Options::default()
    };
    for token in argv {
        let trimmed = token.trim();
        if trimmed == "--help" || trimmed == "-h" {
            out.help = true;
            continue;
        }
        if let Some(raw) = trimmed.strip_prefix("--showcase=") {
            out.showcase = parse_bool_flag(Some(raw), false);
            continue;
        }
        if trimmed == "--showcase" {
            out.showcase = true;
            continue;
        }
        if let Some(raw) = trimmed.strip_prefix("--scaffold-payload=") {
            out.scaffold_payload = parse_bool_flag(Some(raw), false);
            continue;
        }
        if trimmed == "--scaffold-payload" {
            out.scaffold_payload = true;
            continue;
        }
        if trimmed == "--no-prewarm" {
            out.prewarm = false;
            continue;
        }
        if let Some(raw) = trimmed.strip_prefix("--prewarm=") {
            out.prewarm = parse_bool_flag(Some(raw), true);
            continue;
        }
        if let Some(raw) = trimmed.strip_prefix("--duration-ms=") {
            if let Ok(parsed) = raw.parse::<u64>() {
                out.duration_ms = Some(parsed);
            }
            continue;
        }
        if let Some(raw) = trimmed.strip_prefix("--json=") {
            out.json = parse_bool_flag(Some(raw), out.json);
            continue;
        }
        if let Some(raw) = trimmed.strip_prefix("--core-domain=") {
            out.core_domain = raw.trim().to_string();
            continue;
        }
        if let Some(raw) = trimmed.strip_prefix("--core-args-base64=") {
            out.core_args_base64 = raw.trim().to_string();
            continue;
        }
        if let Some(raw) = trimmed.strip_prefix("--target=") {
            out.target = raw.trim().to_string();
            continue;
        }
        if !trimmed.starts_with("--") && out.target.is_empty() {
            out.target = trimmed.to_string();
        }
    }
    out.target = normalize_target(&out.target);
    out
}

pub fn build_receipt_hash(target: &str, ts_iso: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{target}|assimilation|{ts_iso}").as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

pub fn decode_injected_route(options: &Options) -> Result<Option<Route>, String> {
    let domain = options.core_domain.trim();
    if domain.is_empty() {
        return Ok(None);
    }
    let raw_b64 = options.core_args_base64.trim();
    if raw_b64.is_empty() {
        return Err("core-args-base64 is required when core-domain is provided".to_string());
    }
    let decoded = BASE64_STANDARD
        .decode(raw_b64.as_bytes())
        .map_err(|_| "invalid core route payload".to_string())?;
    let text = String::from_utf8(decoded).map_err(|_| "invalid core route payload".to_string())?;
    let rows = serde_json::from_str::<Vec<String>>(&text)
        .map_err(|_| "core route args must be a string array".to_string())?;
    Ok(Some(Route {
        domain: domain.to_string(),
        args: rows,
    }))
}

pub fn payload_scaffold_for(target: &str) -> Value {
    let normalized = target.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "haystack" | "workflow://haystack" | "rag://haystack" => json!({
            "name": "example-haystack-pipeline",
            "components": [{
                "id": "retriever",
                "stage_type": "retriever",
                "input_type": "text",
                "output_type": "docs",
                "parallel": false,
                "spawn": false,
                "budget": 128
            }]
        }),
        "langchain" | "workflow://langchain" | "chains://langchain" => {
            json!({"name":"langchain-integration","integration_type":"tool","capabilities":["retrieve"]})
        }
        "dspy" | "workflow://dspy" | "optimizer://dspy" => {
            json!({"name":"dspy-integration","kind":"retriever","capabilities":["retrieve"]})
        }
        "pydantic-ai" | "workflow://pydantic-ai" | "agents://pydantic-ai" => {
            json!({"name":"pydantic-agent","model":"gpt-4o-mini","tools":[]})
        }
        "camel" | "workflow://camel" | "society://camel" => {
            json!({"name":"camel-dataset","dataset":{"rows":[]}})
        }
        "llamaindex" | "rag://llamaindex" => {
            json!({"name":"llamaindex-connector","connector_type":"filesystem","root_path":"./docs"})
        }
        "google-adk" | "workflow://google-adk" => {
            json!({"name":"google-adk-tool-manifest","tools":[]})
        }
        "mastra" | "workflow://mastra" => json!({"name":"mastra-graph","nodes":[],"edges":[]}),
        "shannon" | "workflow://shannon" => json!({"profile":"rich","task":"assimilate"}),
        _ => json!({
            "target": if normalized.is_empty() { "unknown" } else { &normalized },
            "hint": "No specialized scaffold exists for this target. Use --payload-base64 with target-specific JSON."
        }),
    }
}

fn parse_last_json_object(raw: &str) -> Option<Value> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return Some(value);
    }
    for line in trimmed.lines().rev() {
        let row = line.trim();
        if row.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(row) {
            return Some(value);
        }
    }
    None
}

fn ensure_state_dir(root: &Path) {
    let _ = fs::create_dir_all(root.join(STATE_DIR_REL));
}

fn read_metrics(path: &Path) -> MetricsState {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<MetricsState>(&raw).ok())
        .unwrap_or_default()
}

fn write_metrics(path: &Path, metrics: &MetricsState) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string_pretty(metrics) {
        let _ = fs::write(path, format!("{raw}\n"));
    }
}

fn percentile(sorted: &[u64], p: usize) -> u64 {
    if sorted.is_empty() {
        return 0;
    }
    let idx = ((p as f64 / 100.0) * sorted.len() as f64).ceil() as isize - 1;
    let bounded = idx.clamp(0, sorted.len() as isize - 1) as usize;
    sorted[bounded]
}

pub fn update_metrics(root: &Path, target: &str, latency_ms: u64, ok: bool) -> TargetMetrics {
    let metrics_path = root.join(METRICS_STATE_REL);
    let mut metrics = read_metrics(&metrics_path);
    let row = metrics.targets.entry(target.to_string()).or_default();
    row.count += 1;
    if ok {
        row.ok_count += 1;
    } else {
        row.fail_count += 1;
    }
    row.last_latency_ms = latency_ms;
    row.updated_at = now_iso();
    if ok {
        row.latencies_ms.push(latency_ms);
        if row.latencies_ms.len() > 200 {
            let keep_from = row.latencies_ms.len() - 200;
            row.latencies_ms = row.latencies_ms.split_off(keep_from);
        }
        let mut sorted = row.latencies_ms.clone();
        sorted.sort_unstable();
        row.p50_ms = percentile(&sorted, 50);
        row.p95_ms = percentile(&sorted, 95);
    }
    let out = row.clone();
    write_metrics(&metrics_path, &metrics);
    out
}

pub fn maybe_prewarm(root: &Path, enabled: bool) {
    if !enabled {
        return;
    }
    let path = root.join(PREWARM_STATE_REL);
    let state = fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<PrewarmState>(&raw).ok())
        .unwrap_or_default();
    let now_ms = chrono::Utc::now().timestamp_millis();
    if now_ms - state.ts_ms < DEFAULT_PREWARM_TTL_MS {
        return;
    }
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("protheus-ops"));
    let _ = Command::new(exe)
        .current_dir(root)
        .arg("health-status")
        .arg("status")
        .arg("--fast=1")
        .output();
    ensure_state_dir(root);
    let next = PrewarmState {
        ts_ms: now_ms,
        ts: now_iso(),
    };
    if let Ok(raw) = serde_json::to_string_pretty(&next) {
        let _ = fs::write(path, format!("{raw}\n"));
    }
}

pub fn run_core_assimilation(root: &Path, domain: &str, args: &[String]) -> RunResult {
    let start = Instant::now();
    let exe = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("protheus-ops"));
    match Command::new(exe)
        .current_dir(root)
        .arg(domain)
        .args(args)
        .output()
    {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            RunResult {
                status: out.status.code().unwrap_or(1),
                latency_ms: start.elapsed().as_millis() as u64,
                payload: parse_last_json_object(&stdout),
                stderr,
            }
        }
        Err(err) => RunResult {
            status: 1,
            latency_ms: start.elapsed().as_millis() as u64,
            payload: None,
            stderr: format!("spawn_failed:{err}"),
        },
    }
}

pub fn render_bar(percent: u32) -> String {
    let bounded = percent.clamp(0, 100) as f64;
    let filled = ((bounded / 100.0) * BAR_WIDTH as f64).round() as usize;
    format!(
        "[{}{}]",
        FILLED_CHAR.to_string().repeat(filled),
        EMPTY_CHAR
            .to_string()
            .repeat(BAR_WIDTH.saturating_sub(filled))
    )
}
