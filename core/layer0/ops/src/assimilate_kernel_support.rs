// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;
use walkdir::WalkDir;

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
const RECON_MAX_FILES: usize = 2500;
const RECON_MAX_DEPTH: usize = 8;

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
    pub allow_local_simulation: bool,
    pub plan_only: bool,
    pub hard_selector: String,
    pub selector_bypass: bool,
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
    println!("Usage: infring assimilate <target> [--payload-base64=...] [--strict=1] [--showcase=1] [--duration-ms=<n>] [--json=1] [--scaffold-payload=1] [--allow-local-simulation=1] [--plan-only=1] [--hard-selector=<selector>] [--selector-bypass=1]");
    println!();
    println!("Known targets route to governed core bridge lanes. Unknown targets fail as unadmitted unless --allow-local-simulation=1 is set.");
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
        allow_local_simulation: parse_bool_flag(
            std::env::var("INFRING_ALLOW_LOCAL_SIMULATION")
                .ok()
                .or_else(|| std::env::var("PROTHEUS_ALLOW_LOCAL_SIMULATION").ok())
                .as_deref(),
            false,
        ),
        plan_only: false,
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
        if let Some(raw) = trimmed.strip_prefix("--allow-local-simulation=") {
            out.allow_local_simulation = parse_bool_flag(Some(raw), out.allow_local_simulation);
            continue;
        }
        if trimmed == "--allow-local-simulation" {
            out.allow_local_simulation = true;
            continue;
        }
        if let Some(raw) = trimmed.strip_prefix("--plan-only=") {
            out.plan_only = parse_bool_flag(Some(raw), out.plan_only);
            continue;
        }
        if trimmed == "--plan-only" {
            out.plan_only = true;
            continue;
        }
        if let Some(raw) = trimmed.strip_prefix("--hard-selector=") {
            out.hard_selector = raw.trim().to_string();
            continue;
        }
        if let Some(raw) = trimmed.strip_prefix("--selector-bypass=") {
            out.selector_bypass = parse_bool_flag(Some(raw), out.selector_bypass);
            continue;
        }
        if trimmed == "--selector-bypass" {
            out.selector_bypass = true;
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
        "workflow_chain" | "workflow://workflow_chain" | "chains://workflow_chain" => {
            json!({"name":"workflow_chain-integration","integration_type":"tool","capabilities":["retrieve"]})
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

fn normalized_path_text(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn should_skip_scan_path(path: &Path) -> bool {
    let normalized = normalized_path_text(path).to_ascii_lowercase();
    normalized.contains("/.git/")
        || normalized.ends_with("/.git")
        || normalized.contains("/node_modules/")
        || normalized.ends_with("/node_modules")
        || normalized.contains("/target/")
        || normalized.ends_with("/target")
        || normalized.contains("/dist/")
        || normalized.ends_with("/dist")
        || normalized.contains("/build/")
        || normalized.ends_with("/build")
        || normalized.contains("/.next/")
        || normalized.ends_with("/.next")
        || normalized.contains("/__pycache__/")
        || normalized.ends_with("/__pycache__")
        || normalized.contains("/.venv/")
        || normalized.ends_with("/.venv")
        || normalized.contains("/venv/")
        || normalized.ends_with("/venv")
        || normalized.contains("/local/state/")
}

fn repo_root_from_env_or_cwd() -> PathBuf {
    std::env::var("INFRING_ROOT")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var("PROTHEUS_ROOT")
                .ok()
                .filter(|v| !v.trim().is_empty())
                .map(PathBuf::from)
        })
        .or_else(|| std::env::current_dir().ok())
        .unwrap_or_else(|| PathBuf::from("."))
}

fn infer_target_root(target: &str) -> Option<PathBuf> {
    let candidate = target.trim();
    if candidate.is_empty() {
        return None;
    }
    if candidate.starts_with("http://")
        || candidate.starts_with("https://")
        || candidate.contains("://")
    {
        return None;
    }
    let path = PathBuf::from(candidate);
    if path.is_absolute() && path.exists() {
        return fs::canonicalize(path).ok();
    }
    let root = repo_root_from_env_or_cwd();
    let joined = root.join(path);
    if joined.exists() {
        return fs::canonicalize(joined).ok();
    }
    None
}

fn relative_or_absolute(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .ok()
        .map(normalized_path_text)
        .unwrap_or_else(|| normalized_path_text(path))
}

fn sha256_hex(raw: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw);
    format!("sha256:{:x}", hasher.finalize())
}

fn manifest_kind(path: &Path) -> Option<&'static str> {
    let name = path
        .file_name()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    match name.as_str() {
        "cargo.toml" => Some("cargo_toml"),
        "package.json" => Some("package_json"),
        "pyproject.toml" => Some("pyproject_toml"),
        "requirements.txt" => Some("requirements_txt"),
        "go.mod" => Some("go_mod"),
        "pom.xml" => Some("pom_xml"),
        "build.gradle" | "build.gradle.kts" => Some("gradle"),
        _ => None,
    }
}

fn parse_cargo_dependency_hints(raw: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();
    let mut in_dependency_table = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_dependency_table = trimmed == "[dependencies]"
                || trimmed == "[dev-dependencies]"
                || trimmed == "[workspace.dependencies]";
            continue;
        }
        if !in_dependency_table || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((name, _)) = trimmed.split_once('=') {
            let dep = name.trim();
            if !dep.is_empty() {
                out.insert(dep.to_string());
            }
        }
    }
    out
}

fn parse_package_json_dependency_hints(raw: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();
    let parsed = serde_json::from_str::<Value>(raw).ok();
    let Some(payload) = parsed else {
        return out;
    };
    let keys = [
        "dependencies",
        "devDependencies",
        "peerDependencies",
        "optionalDependencies",
    ];
    for key in keys {
        let Some(row) = payload.get(key).and_then(Value::as_object) else {
            continue;
        };
        for dep in row.keys() {
            out.insert(dep.to_string());
        }
    }
    out
}

fn parse_requirements_dependency_hints(raw: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let dep = trimmed
            .split(['=', '>', '<', '!', '~', ';', '['])
            .next()
            .unwrap_or("")
            .trim();
        if !dep.is_empty() {
            out.insert(dep.to_string());
        }
    }
    out
}

fn parse_pyproject_dependency_hints(raw: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();
    let mut in_dependencies = false;
    let mut in_optional_dependencies = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_dependencies = trimmed == "[project]" || trimmed == "[tool.poetry.dependencies]";
            in_optional_dependencies = trimmed.starts_with("[project.optional-dependencies.");
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if in_dependencies && trimmed.starts_with("dependencies") && trimmed.contains('[') {
            if let Some((_, rhs)) = trimmed.split_once('=') {
                for token in rhs.split([',', '[', ']', '"', '\'']) {
                    let dep = token.trim();
                    if dep.is_empty() || dep.contains(' ') || dep.contains('=') {
                        continue;
                    }
                    out.insert(dep.to_string());
                }
            }
            continue;
        }
        if in_optional_dependencies {
            for token in trimmed.split([',', '[', ']', '"', '\'']) {
                let dep = token.trim();
                if dep.is_empty() || dep.contains(' ') || dep.contains('=') {
                    continue;
                }
                out.insert(dep.to_string());
            }
            continue;
        }
        if trimmed.contains('=') && (in_dependencies || trimmed.starts_with("name")) {
            let dep = trimmed
                .split('=')
                .next()
                .unwrap_or("")
                .trim()
                .trim_matches('"')
                .trim_matches('\'');
            if !dep.is_empty() && dep != "python" && dep != "name" {
                out.insert(dep.to_string());
            }
        }
    }
    out
}

fn parse_go_mod_dependency_hints(raw: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();
    let mut in_require_block = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        if trimmed.starts_with("require (") {
            in_require_block = true;
            continue;
        }
        if in_require_block && trimmed == ")" {
            in_require_block = false;
            continue;
        }
        if in_require_block || trimmed.starts_with("require ") {
            let dep = trimmed
                .trim_start_matches("require")
                .trim()
                .split_whitespace()
                .next()
                .unwrap_or("")
                .trim();
            if !dep.is_empty() {
                out.insert(dep.to_string());
            }
        }
    }
    out
}

fn parse_pom_dependency_hints(raw: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();
    let mut search = raw;
    while let Some(start) = search.find("<artifactId>") {
        let rest = &search[start + "<artifactId>".len()..];
        let Some(end) = rest.find("</artifactId>") else {
            break;
        };
        let dep = rest[..end].trim();
        if !dep.is_empty() && dep != "${project.artifactId}" {
            out.insert(dep.to_string());
        }
        search = &rest[end + "</artifactId>".len()..];
    }
    out
}

fn parse_gradle_dependency_hints(raw: &str) -> BTreeSet<String> {
    let mut out = BTreeSet::<String>::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if !trimmed.contains("implementation")
            && !trimmed.contains("api ")
            && !trimmed.contains("testImplementation")
        {
            continue;
        }
        if let Some(idx) = trimmed.find('\'') {
            let rhs = &trimmed[idx + 1..];
            if let Some(end) = rhs.find('\'') {
                let dep = rhs[..end]
                    .split(':')
                    .nth(1)
                    .unwrap_or(rhs[..end].trim())
                    .trim();
                if !dep.is_empty() {
                    out.insert(dep.to_string());
                }
            }
        }
    }
    out
}

fn add_framework_markers(raw: &str, out: &mut BTreeSet<String>) {
    let lower = raw.to_ascii_lowercase();
    let markers = [
        "langgraph",
        "openai-agents",
        "openai_agents",
        "openai",
        "crewai",
        "mastra",
        "llamaindex",
        "llama-index",
        "haystack",
        "dspy",
        "pydantic-ai",
        "pydantic_ai",
        "camel-ai",
        "camel",
        "google-adk",
        "google_adk",
    ];
    for marker in markers {
        if lower.contains(marker) {
            out.insert(marker.to_string());
        }
    }
}

fn dependency_hints(kind: &str, raw: &str) -> Vec<String> {
    let mut out = match kind {
        "cargo_toml" => parse_cargo_dependency_hints(raw),
        "package_json" => parse_package_json_dependency_hints(raw),
        "pyproject_toml" => parse_pyproject_dependency_hints(raw),
        "requirements_txt" => parse_requirements_dependency_hints(raw),
        "go_mod" => parse_go_mod_dependency_hints(raw),
        "pom_xml" => parse_pom_dependency_hints(raw),
        "gradle" => parse_gradle_dependency_hints(raw),
        _ => BTreeSet::<String>::new(),
    };
    add_framework_markers(raw, &mut out);
    out.into_iter().take(120).collect::<Vec<_>>()
}

fn parse_cargo_package_name(raw: &str) -> Option<String> {
    let mut in_package = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_package = trimmed == "[package]";
            continue;
        }
        if !in_package || trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if let Some((left, right)) = trimmed.split_once('=') {
            if left.trim() == "name" {
                let value = right
                    .trim()
                    .trim_matches('"')
                    .trim_matches('\'')
                    .to_string();
                if !value.is_empty() {
                    return Some(value);
                }
            }
        }
    }
    None
}

fn parse_pyproject_package_name(raw: &str) -> Option<String> {
    let mut in_project = false;
    let mut in_poetry = false;
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            in_project = trimmed == "[project]";
            in_poetry = trimmed == "[tool.poetry]";
            continue;
        }
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        if in_project || in_poetry {
            if let Some((left, right)) = trimmed.split_once('=') {
                if left.trim() == "name" {
                    let value = right
                        .trim()
                        .trim_matches('"')
                        .trim_matches('\'')
                        .to_string();
                    if !value.is_empty() {
                        return Some(value);
                    }
                }
            }
        }
    }
    None
}

fn parse_go_module_name(raw: &str) -> Option<String> {
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("//") {
            continue;
        }
        if let Some(rest) = trimmed.strip_prefix("module ") {
            let value = rest.trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn parse_pom_package_name(raw: &str) -> Option<String> {
    if let Some(start) = raw.find("<artifactId>") {
        let rest = &raw[start + "<artifactId>".len()..];
        if let Some(end) = rest.find("</artifactId>") {
            let value = rest[..end].trim();
            if !value.is_empty() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn parse_gradle_package_name(raw: &str) -> Option<String> {
    for line in raw.lines() {
        let trimmed = line.trim();
        if let Some(rhs) = trimmed.strip_prefix("rootProject.name") {
            let value = rhs
                .split('=')
                .nth(1)
                .unwrap_or("")
                .trim()
                .trim_matches('"')
                .trim_matches('\'')
                .to_string();
            if !value.is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn parse_manifest_package_name(kind: &str, raw: &str) -> Option<String> {
    match kind {
        "cargo_toml" => parse_cargo_package_name(raw),
        "package_json" => serde_json::from_str::<Value>(raw)
            .ok()
            .and_then(|v| v.get("name").and_then(Value::as_str).map(|s| s.to_string())),
        "pyproject_toml" => parse_pyproject_package_name(raw),
        "go_mod" => parse_go_module_name(raw),
        "pom_xml" => parse_pom_package_name(raw),
        "gradle" => parse_gradle_package_name(raw),
        _ => None,
    }
}

fn normalize_dependency_token(raw: &str) -> String {
    raw.trim()
        .trim_matches('"')
        .trim_matches('\'')
        .to_ascii_lowercase()
        .replace('_', "-")
}

fn parse_manifest_inventory(root: &Path) -> Vec<Value> {
    let mut out = Vec::<Value>::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .max_depth(RECON_MAX_DEPTH)
        .into_iter()
        .filter_entry(|row| !should_skip_scan_path(row.path()))
        .filter_map(Result::ok)
    {
        if out.len() >= 80 {
            break;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let path = entry.path();
        let Some(kind) = manifest_kind(path) else {
            continue;
        };
        let raw = fs::read(path).unwrap_or_default();
        let text = String::from_utf8_lossy(&raw).to_string();
        let package_name = parse_manifest_package_name(kind, &text);
        out.push(json!({
            "path": relative_or_absolute(root, path),
            "kind": kind,
            "content_hash": sha256_hex(&raw),
            "package_name": package_name,
            "dependency_hints": dependency_hints(kind, &text)
        }));
    }
    out.sort_by(|a, b| {
        let left = a.get("path").and_then(Value::as_str).unwrap_or("");
        let right = b.get("path").and_then(Value::as_str).unwrap_or("");
        left.cmp(&right)
    });
    out
}

fn parse_license_surface(root: &Path) -> Vec<Value> {
    let mut out = Vec::<Value>::new();
    let mut seen = BTreeSet::<String>::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .max_depth(4)
        .into_iter()
        .filter_entry(|row| !should_skip_scan_path(row.path()))
        .filter_map(Result::ok)
    {
        if out.len() >= 24 {
            break;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry
            .path()
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let is_license = name == "license"
            || name.starts_with("license.")
            || name == "copying"
            || name == "notice"
            || name == "security.md"
            || name == "license_scope.md";
        if !is_license {
            continue;
        }
        let path = relative_or_absolute(root, entry.path());
        if seen.insert(path.clone()) {
            out.push(json!({
                "path": path,
                "kind": "license_artifact"
            }));
        }
    }
    out.sort_by(|a, b| {
        let left = a.get("path").and_then(Value::as_str).unwrap_or("");
        let right = b.get("path").and_then(Value::as_str).unwrap_or("");
        left.cmp(&right)
    });
    out
}

fn parse_test_surface(root: &Path) -> Value {
    let mut directories = BTreeSet::<String>::new();
    let mut sample_files = Vec::<String>::new();
    let mut test_file_count: u64 = 0;
    let mut scanned: usize = 0;
    for entry in WalkDir::new(root)
        .follow_links(false)
        .max_depth(RECON_MAX_DEPTH)
        .into_iter()
        .filter_entry(|row| !should_skip_scan_path(row.path()))
        .filter_map(Result::ok)
    {
        if scanned >= RECON_MAX_FILES {
            break;
        }
        scanned += 1;
        let path = entry.path();
        let rel = relative_or_absolute(root, path);
        if entry.file_type().is_dir() {
            let dir = path
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            if dir == "tests" || dir == "__tests__" || dir == "spec" {
                directories.insert(rel);
            }
            continue;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let name = path
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let is_test_file = name.ends_with("_test.rs")
            || name.ends_with("_test.py")
            || name.ends_with(".spec.ts")
            || name.ends_with(".spec.tsx")
            || name.ends_with(".spec.js")
            || name.ends_with(".test.ts")
            || name.ends_with(".test.tsx")
            || name.ends_with(".test.js")
            || name.ends_with(".test.rs")
            || name.ends_with(".feature");
        if is_test_file {
            test_file_count += 1;
            if sample_files.len() < 20 {
                sample_files.push(rel);
            }
        }
    }
    sample_files.sort_unstable();
    json!({
        "directory_hints": directories.into_iter().collect::<Vec<_>>(),
        "test_file_count": test_file_count,
        "sample_files": sample_files,
        "scanned_entries": scanned
    })
}

fn parse_api_surface(root: &Path) -> Vec<Value> {
    let mut out = Vec::<Value>::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .max_depth(RECON_MAX_DEPTH)
        .into_iter()
        .filter_entry(|row| !should_skip_scan_path(row.path()))
        .filter_map(Result::ok)
    {
        if out.len() >= 30 {
            break;
        }
        if !entry.file_type().is_file() {
            continue;
        }
        let name = entry
            .path()
            .file_name()
            .and_then(|v| v.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let kind = if name.ends_with(".proto") {
            Some("proto")
        } else if name == "openapi.yaml" || name == "openapi.yml" || name == "openapi.json" {
            Some("openapi")
        } else if name.ends_with(".graphql") || name.ends_with(".gql") {
            Some("graphql")
        } else {
            None
        };
        if let Some(kind) = kind {
            out.push(json!({
                "path": relative_or_absolute(root, entry.path()),
                "kind": kind
            }));
        }
    }
    out.sort_by(|a, b| {
        let left = a.get("path").and_then(Value::as_str).unwrap_or("");
        let right = b.get("path").and_then(Value::as_str).unwrap_or("");
        left.cmp(&right)
    });
    out
}

fn parse_structure_surface(root: &Path) -> Value {
    let mut extension_counts = BTreeMap::<String, u64>::new();
    let mut total_files: u64 = 0;
    let mut scanned: usize = 0;
    for entry in WalkDir::new(root)
        .follow_links(false)
        .max_depth(RECON_MAX_DEPTH)
        .into_iter()
        .filter_entry(|row| !should_skip_scan_path(row.path()))
        .filter_map(Result::ok)
    {
        if scanned >= RECON_MAX_FILES {
            break;
        }
        scanned += 1;
        if !entry.file_type().is_file() {
            continue;
        }
        total_files += 1;
        let ext = entry
            .path()
            .extension()
            .and_then(|v| v.to_str())
            .unwrap_or("")
            .to_ascii_lowercase();
        let key = if ext.is_empty() {
            "no_ext".to_string()
        } else {
            ext
        };
        *extension_counts.entry(key).or_insert(0) += 1;
    }
    let top_extensions = extension_counts
        .iter()
        .map(|(ext, count)| json!({ "extension": ext, "count": count }))
        .collect::<Vec<_>>();
    json!({
        "total_files": total_files,
        "scanned_entries": scanned,
        "top_extensions": top_extensions
    })
}

fn build_dependency_closure(manifest_inventory: &[Value]) -> (Vec<String>, Vec<Value>, Value) {
    let mut dependency_hints = BTreeSet::<String>::new();
    let mut package_index = BTreeMap::<String, (String, String)>::new();
    for row in manifest_inventory {
        let package_name = row
            .get("package_name")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim();
        let path = row
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        if !package_name.is_empty() && !path.is_empty() {
            package_index.insert(
                normalize_dependency_token(package_name),
                (package_name.to_string(), path),
            );
        }
        if let Some(hints) = row.get("dependency_hints").and_then(Value::as_array) {
            for hint in hints {
                if let Some(text) = hint.as_str() {
                    let cleaned = text.trim();
                    if !cleaned.is_empty() {
                        dependency_hints.insert(cleaned.to_string());
                    }
                }
            }
        }
    }
    let mut edges = Vec::<Value>::new();
    let mut edge_seen = BTreeSet::<String>::new();
    let mut internal_edge_count: usize = 0;
    let mut external_edge_count: usize = 0;
    for row in manifest_inventory {
        let source_path = row
            .get("path")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        if source_path.is_empty() {
            continue;
        }
        let source_kind = row
            .get("kind")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let source_package = row
            .get("package_name")
            .and_then(Value::as_str)
            .unwrap_or("unknown")
            .to_string();
        let Some(hints) = row.get("dependency_hints").and_then(Value::as_array) else {
            continue;
        };
        for hint in hints {
            let Some(dep_raw) = hint.as_str() else {
                continue;
            };
            let dep = dep_raw.trim();
            if dep.is_empty() {
                continue;
            }
            let key = format!("{source_path}::{dep}");
            if !edge_seen.insert(key) {
                continue;
            }
            let token = normalize_dependency_token(dep);
            let target = package_index.get(&token);
            let relation = if target.is_some() {
                internal_edge_count += 1;
                "internal_manifest_dependency"
            } else {
                external_edge_count += 1;
                "external_dependency_hint"
            };
            let target_package = target.map(|row| row.0.clone());
            let target_manifest = target.map(|row| row.1.clone());
            edges.push(json!({
                "id": dep,
                "source": "manifest_hint",
                "source_manifest": source_path,
                "source_kind": source_kind,
                "source_package": source_package,
                "relation": relation,
                "target_package": target_package,
                "target_manifest": target_manifest
            }));
        }
    }
    if edges.is_empty() {
        edges = dependency_hints
            .iter()
            .take(200)
            .map(|hint| {
                json!({
                    "id": hint,
                    "source": "manifest_hint",
                    "relation": "unresolved_dependency_hint"
                })
            })
            .collect::<Vec<_>>();
    }
    let summary = json!({
        "manifest_node_count": manifest_inventory.len(),
        "package_node_count": package_index.len(),
        "edge_count": edges.len(),
        "internal_edge_count": internal_edge_count,
        "external_edge_count": external_edge_count
    });
    (
        dependency_hints.into_iter().collect::<Vec<_>>(),
        edges,
        summary,
    )
}

fn framework_targets_from_hints(hints: &[String]) -> Vec<String> {
    let mut out = BTreeSet::<String>::new();
    for hint in hints {
        let normalized = hint.to_ascii_lowercase();
        if normalized.contains("langgraph") {
            out.insert("workflow://langgraph".to_string());
        }
        if normalized.contains("openai-agents") || normalized.contains("openai_agents") {
            out.insert("workflow://openai-agents".to_string());
        }
        if normalized.contains("crewai") {
            out.insert("workflow://crewai".to_string());
        }
        if normalized.contains("mastra") {
            out.insert("workflow://mastra".to_string());
        }
        if normalized.contains("llamaindex") || normalized.contains("llama-index") {
            out.insert("workflow://llamaindex".to_string());
        }
        if normalized.contains("haystack") {
            out.insert("workflow://haystack".to_string());
        }
        if normalized.contains("dspy") {
            out.insert("workflow://dspy".to_string());
        }
        if normalized.contains("pydantic-ai") || normalized.contains("pydantic_ai") {
            out.insert("workflow://pydantic-ai".to_string());
        }
        if normalized.contains("camel-ai") || normalized == "camel" {
            out.insert("workflow://camel".to_string());
        }
        if normalized.contains("google-adk") || normalized.contains("google_adk") {
            out.insert("workflow://google-adk".to_string());
        }
    }
    out.into_iter().collect::<Vec<_>>()
}

fn framework_targets_from_surfaces(
    hints: &[String],
    api_surface: &[Value],
    structure_surface: &Value,
) -> Vec<String> {
    let mut out = framework_targets_from_hints(hints)
        .into_iter()
        .collect::<BTreeSet<_>>();
    for api in api_surface {
        match api.get("kind").and_then(Value::as_str).unwrap_or("") {
            "openapi" => {
                out.insert("workflow://openapi-service".to_string());
            }
            "proto" => {
                out.insert("workflow://grpc-service".to_string());
            }
            "graphql" => {
                out.insert("workflow://graphql-service".to_string());
            }
            _ => {}
        }
    }
    let top_ext = structure_surface
        .get("top_extensions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for row in top_ext {
        let ext = row.get("extension").and_then(Value::as_str).unwrap_or("");
        match ext {
            "rs" => {
                out.insert("stack://rust".to_string());
            }
            "py" => {
                out.insert("stack://python".to_string());
            }
            "ts" | "tsx" => {
                out.insert("stack://typescript".to_string());
            }
            "go" => {
                out.insert("stack://go".to_string());
            }
            _ => {}
        }
    }
    out.into_iter().collect::<Vec<_>>()
}

fn recon_index_for_target(
    normalized_target: &str,
    route: Option<&Route>,
    target_class: &str,
    ts_iso: &str,
) -> (
    Value,
    Vec<String>,
    Vec<Value>,
    Value,
    Vec<Value>,
    Vec<Value>,
    Value,
    Value,
) {
    let route_obj = route.map(|v| {
        json!({
            "domain": v.domain,
            "args": v.args
        })
    });
    let target_root = infer_target_root(normalized_target);
    let root = target_root
        .as_ref()
        .cloned()
        .unwrap_or_else(repo_root_from_env_or_cwd);
    let root_exists = target_root.as_ref().is_some();
    let manifest_inventory = if root_exists {
        parse_manifest_inventory(&root)
    } else {
        Vec::new()
    };
    let (dependency_hints, dependency_closure, dependency_graph_summary) =
        build_dependency_closure(&manifest_inventory);
    let license_surface = if root_exists {
        parse_license_surface(&root)
    } else {
        Vec::new()
    };
    let test_surface = if root_exists {
        parse_test_surface(&root)
    } else {
        json!({"directory_hints":[],"test_file_count":0,"sample_files":[],"scanned_entries":0})
    };
    let api_surface = if root_exists {
        parse_api_surface(&root)
    } else {
        Vec::new()
    };
    let structure_surface = if root_exists {
        parse_structure_surface(&root)
    } else {
        json!({"total_files":0,"scanned_entries":0,"top_extensions":[]})
    };
    let recon_index = json!({
        "recon_id": build_receipt_hash(&format!("recon:{normalized_target}"), ts_iso),
        "route": route_obj,
        "probe_set": [
            "shape_scan",
            "dependency_scan",
            "integration_scan",
            "license_surface_scan",
            "test_surface_scan",
            "api_surface_scan",
            "structure_surface_scan"
        ],
        "target_root": if root_exists { Value::String(normalized_path_text(&root)) } else { Value::Null },
        "target_class": target_class,
        "manifest_inventory": manifest_inventory,
        "dependency_graph_summary": dependency_graph_summary,
        "license_surface": license_surface,
        "test_surface": test_surface,
        "api_surface": api_surface,
        "structure_surface": structure_surface
    });
    (
        recon_index,
        dependency_hints,
        dependency_closure,
        test_surface,
        license_surface,
        api_surface,
        structure_surface,
        dependency_graph_summary,
    )
}

pub fn canonical_assimilation_plan(
    target: &str,
    route: Option<&Route>,
    ts_iso: &str,
    requested_admission_verdict: &str,
    hard_selector: &str,
    selector_bypass: bool,
) -> Value {
    let normalized_target = normalize_target(target);
    let normalized_selector = normalize_target(hard_selector);
    let hard_selector_present = !normalized_selector.is_empty();
    let target_class =
        if normalized_target.starts_with("http://") || normalized_target.starts_with("https://") {
            "url"
        } else if normalized_target.contains("://") {
            "named_target"
        } else if normalized_target.contains('/') || normalized_target.contains('\\') {
            "path"
        } else {
            "named_target"
        };
    let route_domain = route
        .map(|v| normalize_target(&v.domain))
        .unwrap_or_default();
    let selector_matches_target = !hard_selector_present
        || normalized_selector == normalized_target
        || (!route_domain.is_empty() && normalized_selector == route_domain);
    let closure_controls_satisfied = route.is_some() && selector_matches_target && !selector_bypass;
    let intent_spec = json!({
        "intent_id": build_receipt_hash(&format!("intent:{normalized_target}"), ts_iso),
        "target": normalized_target.clone(),
        "target_class": target_class,
        "requested_at": ts_iso
    });
    let (
        recon_index,
        dependency_hints,
        dependency_closure,
        test_surface,
        license_surface,
        api_surface,
        structure_surface,
        dependency_graph_summary,
    ) = recon_index_for_target(&normalized_target, route, target_class, ts_iso);
    let framework_candidates =
        framework_targets_from_surfaces(&dependency_hints, &api_surface, &structure_surface);
    let mut candidate_target_set = BTreeSet::<String>::new();
    candidate_target_set.insert(normalized_target.clone());
    if !route_domain.is_empty() {
        candidate_target_set.insert(route_domain.clone());
    }
    for candidate in framework_candidates {
        candidate_target_set.insert(candidate);
    }
    let candidate_targets = candidate_target_set.into_iter().collect::<Vec<_>>();
    let manifest_count = recon_index
        .get("manifest_inventory")
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    let test_file_count = test_surface
        .get("test_file_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let license_count = license_surface.len();
    let api_surface_count = api_surface.len();
    let structure_file_count = structure_surface
        .get("total_files")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let dependency_edge_count = dependency_graph_summary
        .get("edge_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let recon_surface_complete = if target_class == "path" {
        manifest_count > 0 && structure_file_count > 0 && dependency_edge_count > 0
    } else {
        true
    };
    let closure_complete =
        closure_controls_satisfied && !candidate_targets.is_empty() && recon_surface_complete;
    let candidate_set = json!({
        "candidate_set_id": build_receipt_hash(&format!("cset:{normalized_target}"), ts_iso),
        "targets": candidate_targets,
        "dependency_hints": dependency_hints,
        "selector_mode": if hard_selector_present { "hard" } else { "auto" },
        "hard_selector": if hard_selector_present {
            Value::String(normalized_selector.clone())
        } else {
            Value::Null
        },
        "admissible_count": if closure_complete { 1 } else { 0 }
    });
    let candidate_closure = json!({
        "closure_id": build_receipt_hash(&format!("closure:{normalized_target}"), ts_iso),
        "resolved_targets": [normalized_target.clone()],
        "dependencies": dependency_closure,
        "closure_complete": closure_complete,
        "closure_stats": {
            "manifest_count": manifest_count,
            "test_file_count": test_file_count,
            "license_count": license_count,
            "api_surface_count": api_surface_count,
            "structure_file_count": structure_file_count,
            "dependency_graph_summary": dependency_graph_summary
        },
        "selected_candidate": if closure_complete {
            json!({
                "target": normalized_target.clone(),
                "route_domain": route_domain,
            })
        } else {
            Value::Null
        }
    });
    let mut gaps = Vec::<Value>::new();
    if selector_bypass {
        gaps.push(json!({
            "gap_id": "assimilation_selector_bypass_rejected",
            "severity": "blocker",
            "detail": "selector bypass is prohibited in the canonical assimilation protocol"
        }));
    }
    if hard_selector_present && !selector_matches_target {
        gaps.push(json!({
            "gap_id": "assimilation_hard_selector_closure_reject",
            "severity": "blocker",
            "detail": format!("hard selector `{}` did not resolve to the target or routed domain", normalized_selector)
        }));
    }
    if !closure_complete {
        gaps.push(json!({
            "gap_id": "assimilation_candidate_closure_incomplete",
            "severity": "blocker",
            "detail": "candidate closure is incomplete; no admissible closure candidate is available"
        }));
    }
    if target_class == "path" && manifest_count == 0 {
        gaps.push(json!({
            "gap_id": "assimilation_manifest_surface_missing",
            "severity": "blocker",
            "detail": "recon scan found no dependency manifests for a path target; assimilation cannot derive dependency closure safely"
        }));
    }
    if target_class == "path" && structure_file_count == 0 {
        gaps.push(json!({
            "gap_id": "assimilation_structure_surface_empty",
            "severity": "blocker",
            "detail": "recon scan found no source files for target path; target may be invalid or inaccessible"
        }));
    }
    if target_class == "path" && license_count == 0 {
        gaps.push(json!({
            "gap_id": "assimilation_license_surface_missing",
            "severity": "warning",
            "detail": "no license/security artifacts were discovered; legal/compliance review may be required"
        }));
    }
    if target_class == "path" && api_surface_count == 0 {
        gaps.push(json!({
            "gap_id": "assimilation_api_surface_missing",
            "severity": "warning",
            "detail": "no API/protocol surface discovered; integration blast radius may be under-modeled"
        }));
    }
    if target_class == "path" && test_file_count == 0 {
        gaps.push(json!({
            "gap_id": "assimilation_test_surface_missing",
            "severity": "warning",
            "detail": "no test surface discovered; integration confidence may be reduced"
        }));
    }
    let has_blocker_gap = gaps
        .iter()
        .any(|gap| gap.get("severity").and_then(Value::as_str) == Some("blocker"));
    let admitted =
        requested_admission_verdict == "admitted" && closure_complete && !has_blocker_gap;
    let blocker_count = gaps
        .iter()
        .filter(|gap| gap.get("severity").and_then(Value::as_str) == Some("blocker"))
        .count();
    let warning_count = gaps
        .iter()
        .filter(|gap| gap.get("severity").and_then(Value::as_str) == Some("warning"))
        .count();
    let provisional_gap_report = json!({
        "gap_report_id": build_receipt_hash(&format!("gap:{normalized_target}"), ts_iso),
        "gaps": gaps,
        "risk_level": if admitted { "normal" } else { "elevated" },
        "blocker_count": blocker_count,
        "warning_count": warning_count
    });
    let admission = json!({
        "admission_id": build_receipt_hash(&format!("admission:{normalized_target}"), ts_iso),
        "verdict": if admitted { "admitted" } else { "unadmitted" },
        "policy_gate": "assimilate_admission_v2",
        "requested_verdict": requested_admission_verdict
    });
    let admitted_plan = json!({
        "plan_id": build_receipt_hash(&format!("plan:{normalized_target}"), ts_iso),
        "steps": [
            "intent_spec",
            "recon_index",
            "candidate_set",
            "candidate_closure",
            "gap_analysis",
            "bridge_execution",
            "receipt_commit"
        ],
        "target_root": recon_index.get("target_root").cloned().unwrap_or(Value::Null),
        "rollback": {
            "strategy": "append_only_receipt_reversal",
            "enabled": true
        },
        "status": if admitted { "ready" } else { "blocked" }
    });
    let protocol_step_receipt = json!({
        "receipt_id": build_receipt_hash(&format!("protocol:{normalized_target}"), ts_iso),
        "status": if admitted { "ready" } else { "blocked" },
        "ts": ts_iso
    });
    json!({
        "intent_spec": intent_spec,
        "recon_index": recon_index,
        "candidate_set": candidate_set,
        "candidate_closure": candidate_closure,
        "provisional_gap_report": provisional_gap_report,
        "admission_verdict": admission,
        "admitted_assimilation_plan": admitted_plan,
        "protocol_step_receipt": protocol_step_receipt
    })
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::PathBuf;

    fn temp_recon_root(prefix: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "{prefix}_{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|v| v.as_nanos())
                .unwrap_or(0)
        ));
        let _ = fs::create_dir_all(root.join("src"));
        let _ = fs::create_dir_all(root.join("tests"));
        root
    }

    #[test]
    fn canonical_plan_blocks_when_hard_selector_does_not_match_route_or_target() {
        let route = Route {
            domain: "runtime-systems".to_string(),
            args: vec!["run".to_string()],
        };
        let plan = canonical_assimilation_plan(
            "workflow://langgraph",
            Some(&route),
            "2026-04-08T00:00:00Z",
            "admitted",
            "workflow://other-target",
            false,
        );
        let admitted = plan
            .get("admission_verdict")
            .and_then(|row| row.get("verdict"))
            .and_then(Value::as_str);
        assert_eq!(admitted, Some("unadmitted"));
        let closure_complete = plan
            .get("candidate_closure")
            .and_then(|row| row.get("closure_complete"))
            .and_then(Value::as_bool);
        assert_eq!(closure_complete, Some(false));
    }

    #[test]
    fn canonical_plan_blocks_when_selector_bypass_requested() {
        let route = Route {
            domain: "runtime-systems".to_string(),
            args: vec!["run".to_string()],
        };
        let plan = canonical_assimilation_plan(
            "workflow://langgraph",
            Some(&route),
            "2026-04-08T00:00:00Z",
            "admitted",
            "",
            true,
        );
        let admitted = plan
            .get("admission_verdict")
            .and_then(|row| row.get("verdict"))
            .and_then(Value::as_str);
        assert_eq!(admitted, Some("unadmitted"));
    }

    #[test]
    fn canonical_plan_admits_when_route_present_and_controls_satisfied() {
        let route = Route {
            domain: "runtime-systems".to_string(),
            args: vec!["run".to_string()],
        };
        let plan = canonical_assimilation_plan(
            "workflow://langgraph",
            Some(&route),
            "2026-04-08T00:00:00Z",
            "admitted",
            "runtime-systems",
            false,
        );
        let admitted = plan
            .get("admission_verdict")
            .and_then(|row| row.get("verdict"))
            .and_then(Value::as_str);
        assert_eq!(admitted, Some("admitted"));
        let closure_complete = plan
            .get("candidate_closure")
            .and_then(|row| row.get("closure_complete"))
            .and_then(Value::as_bool);
        assert_eq!(closure_complete, Some(true));
    }

    #[test]
    fn parse_args_accepts_selector_controls() {
        let parsed = parse_args(&[
            "workflow://langgraph".to_string(),
            "--hard-selector=runtime-systems".to_string(),
            "--selector-bypass=1".to_string(),
        ]);
        assert_eq!(parsed.target, "workflow://langgraph");
        assert_eq!(parsed.hard_selector, "runtime-systems");
        assert!(parsed.selector_bypass);
    }

    #[test]
    fn canonical_plan_recon_scans_path_targets_into_surfaces() {
        let root = temp_recon_root("infring_assimilation_recon");
        let _ = fs::write(
            root.join("Cargo.toml"),
            "[package]\nname=\"demo\"\nversion=\"0.1.0\"\n[dependencies]\nserde = \"1\"\n",
        );
        let _ = fs::write(root.join("src/main.rs"), "fn main() {}\n");
        let _ = fs::write(root.join("LICENSE"), "Apache-2.0\n");
        let _ = fs::write(root.join("tests/demo_test.rs"), "#[test] fn ok() {}\n");
        let _ = fs::write(root.join("openapi.yaml"), "openapi: 3.0.0\n");

        let route = Route {
            domain: "runtime-systems".to_string(),
            args: vec!["ops-bridge".to_string()],
        };
        let plan = canonical_assimilation_plan(
            root.to_string_lossy().as_ref(),
            Some(&route),
            "2026-04-09T00:00:00Z",
            "admitted",
            "",
            false,
        );

        let manifest_count = plan
            .get("recon_index")
            .and_then(|row| row.get("manifest_inventory"))
            .and_then(Value::as_array)
            .map(|rows| rows.len())
            .unwrap_or(0);
        assert!(manifest_count >= 1);

        let dependency_count = plan
            .get("candidate_closure")
            .and_then(|row| row.get("dependencies"))
            .and_then(Value::as_array)
            .map(|rows| rows.len())
            .unwrap_or(0);
        assert!(dependency_count >= 1);
        let external_edges = plan
            .get("candidate_closure")
            .and_then(|row| row.get("closure_stats"))
            .and_then(|row| row.get("dependency_graph_summary"))
            .and_then(|row| row.get("external_edge_count"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        assert!(external_edges >= 1);
        let has_openapi_candidate = plan
            .get("candidate_set")
            .and_then(|row| row.get("targets"))
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter().any(|row| {
                    row.as_str() == Some("workflow://openapi-service")
                })
            })
            .unwrap_or(false);
        assert!(has_openapi_candidate);

        let verdict = plan
            .get("admission_verdict")
            .and_then(|row| row.get("verdict"))
            .and_then(Value::as_str);
        assert_eq!(verdict, Some("admitted"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn canonical_plan_dependency_graph_detects_internal_manifest_edges() {
        let root = temp_recon_root("infring_assimilation_internal_graph");
        let _ = fs::create_dir_all(root.join("crates/child/src"));
        let _ = fs::write(
            root.join("Cargo.toml"),
            "[package]\nname=\"root-demo\"\nversion=\"0.1.0\"\n[dependencies]\nchild = { path = \"crates/child\" }\n",
        );
        let _ = fs::write(root.join("src/main.rs"), "fn main() {}\n");
        let _ = fs::write(
            root.join("crates/child/Cargo.toml"),
            "[package]\nname=\"child\"\nversion=\"0.1.0\"\n",
        );
        let _ = fs::write(root.join("crates/child/src/lib.rs"), "pub fn child() {}\n");

        let route = Route {
            domain: "runtime-systems".to_string(),
            args: vec!["ops-bridge".to_string()],
        };
        let plan = canonical_assimilation_plan(
            root.to_string_lossy().as_ref(),
            Some(&route),
            "2026-04-09T00:00:00Z",
            "admitted",
            "",
            false,
        );

        let internal_edges = plan
            .get("candidate_closure")
            .and_then(|row| row.get("closure_stats"))
            .and_then(|row| row.get("dependency_graph_summary"))
            .and_then(|row| row.get("internal_edge_count"))
            .and_then(Value::as_u64)
            .unwrap_or(0);
        assert!(internal_edges >= 1);

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn canonical_plan_emits_manifest_blocker_for_path_target_without_manifests() {
        let root = temp_recon_root("infring_assimilation_recon_blocker");
        let _ = fs::write(root.join("src/main.rs"), "fn main() {}\n");

        let route = Route {
            domain: "runtime-systems".to_string(),
            args: vec!["ops-bridge".to_string()],
        };
        let plan = canonical_assimilation_plan(
            root.to_string_lossy().as_ref(),
            Some(&route),
            "2026-04-09T00:00:00Z",
            "admitted",
            "",
            false,
        );
        let verdict = plan
            .get("admission_verdict")
            .and_then(|row| row.get("verdict"))
            .and_then(Value::as_str);
        assert_eq!(verdict, Some("unadmitted"));

        let has_manifest_blocker = plan
            .get("provisional_gap_report")
            .and_then(|row| row.get("gaps"))
            .and_then(Value::as_array)
            .map(|rows| {
                rows.iter().any(|row| {
                    row.get("gap_id").and_then(Value::as_str)
                        == Some("assimilation_manifest_surface_missing")
                        && row.get("severity").and_then(Value::as_str) == Some("blocker")
                })
            })
            .unwrap_or(false);
        assert!(has_manifest_blocker);

        let _ = fs::remove_dir_all(root);
    }
}
