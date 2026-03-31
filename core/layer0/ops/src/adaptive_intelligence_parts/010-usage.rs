// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use crate::v8_kernel::{
    append_jsonl, build_plane_conduit_enforcement, conduit_bypass_requested, emit_plane_receipt,
    parse_bool, parse_f64, parse_u64, print_json, read_json, scoped_state_root, sha256_hex_str,
    split_csv_clean, write_json, ReceiptJsonExt,
};
use crate::{clean, client_state_root, now_iso, parse_args};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

const STATE_ENV: &str = "ADAPTIVE_INTELLIGENCE_STATE_ROOT";
const STATE_SCOPE: &str = "adaptive_intelligence";
const LOCAL_AI_BIN_ENV: &str = "PROTHEUS_LOCAL_AI_BIN";
const COMMAND_PATH: &str = "core://adaptive-intelligence";

#[derive(Debug, Clone, Serialize, Deserialize)]
struct AdaptivePolicy {
    schema_id: String,
    schema_version: String,
    seed_model: String,
    logical_model: String,
    creative_model: String,
    tiny_logical_model: String,
    resource_thresholds: ResourceThresholds,
    graduation_threshold_pct: f64,
    min_human_approvers: usize,
    nightly_cadence_hours: u64,
    local_only: bool,
    trainer_adapter: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResourceThresholds {
    dual_vram_gb: f64,
    dual_ram_gb: f64,
    dual_cpu_cores: u64,
    logical_vram_gb: f64,
    logical_ram_gb: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ModelProfile {
    role: String,
    seed_model: String,
    active_model: String,
    specialization_score_pct: f64,
    graduated: bool,
    last_trained_at: Option<String>,
    last_graduated_at: Option<String>,
    trainer_adapter: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TrainingState {
    cycles_completed: u64,
    last_job_id: Option<String>,
    last_context_digest: Option<String>,
    last_mode: Option<String>,
    nightly_due: bool,
    last_trained_at: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct RuntimeState {
    version: String,
    created_at: String,
    updated_at: String,
    active_mode: String,
    local_only: bool,
    logical: ModelProfile,
    creative: ModelProfile,
    training: TrainingState,
    last_proposal_digest: Option<String>,
    last_connector_digest: Option<String>,
    last_resource_mode: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResourceSnapshot {
    vram_gb: f64,
    ram_gb: f64,
    cpu_cores: u64,
    mode: String,
    degraded: bool,
}

#[derive(Debug, Clone)]
struct ContextBundle {
    conversation_samples: Vec<String>,
    dream_samples: Vec<String>,
    interaction_digest: String,
    persona: String,
    logical_bias: String,
    creative_bias: String,
}

fn usage() {
    println!("Usage:");
    println!("  protheus-ops adaptive-intelligence status");
    println!("  protheus-ops adaptive-intelligence propose --prompt=<text> [--persona=<id>] [--logical-bias=<text>] [--creative-bias=<text>] [--vram-gb=<n>] [--ram-gb=<n>] [--cpu-cores=<n>] [--strict=1|0]");
    println!("  protheus-ops adaptive-intelligence shadow-train [--cycles=<n>] [--persona=<id>] [--strict=1|0]");
    println!("  protheus-ops adaptive-intelligence prioritize [--vram-gb=<n>] [--ram-gb=<n>] [--cpu-cores=<n>] [--strict=1|0]");
    println!("  protheus-ops adaptive-intelligence graduate --model=<logical|creative> --human-only=1 --approvers=<csv> [--strict=1|0]");
}

fn policy_path(root: &Path) -> PathBuf {
    root.join("client/runtime/config/adaptive_intelligence_policy.json")
}

fn runtime_state_path(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE).join("runtime_state.json")
}

fn proposal_history_path(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE).join("proposal_history.jsonl")
}

fn connector_history_path(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE).join("connector_history.jsonl")
}

fn training_history_path(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE).join("training_history.jsonl")
}

fn graduation_history_path(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE).join("graduation_history.jsonl")
}

fn latest_path(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE).join("latest.json")
}

fn conversation_eye_path(root: &Path) -> PathBuf {
    let runtime_local = root.join("client/runtime/local/state/memory/conversation_eye/nodes.jsonl");
    if runtime_local.exists() {
        return runtime_local;
    }
    client_state_root(root).join("memory/conversation_eye/nodes.jsonl")
}

fn dream_log_path(root: &Path) -> PathBuf {
    crate::core_state_root(root)
        .join("ops")
        .join("organism_layer")
        .join("dream_log.jsonl")
}

fn default_policy() -> AdaptivePolicy {
    AdaptivePolicy {
        schema_id: "adaptive_intelligence_policy".to_string(),
        schema_version: "1.0".to_string(),
        seed_model: "ollama/llama3.2:latest".to_string(),
        logical_model: "ollama/llama3.2:latest".to_string(),
        creative_model: "ollama/qwen2.5:latest".to_string(),
        tiny_logical_model: "ollama/tinyllama:latest".to_string(),
        resource_thresholds: ResourceThresholds {
            dual_vram_gb: 12.0,
            dual_ram_gb: 16.0,
            dual_cpu_cores: 8,
            logical_vram_gb: 4.0,
            logical_ram_gb: 8.0,
        },
        graduation_threshold_pct: 85.0,
        min_human_approvers: 2,
        nightly_cadence_hours: 24,
        local_only: true,
        trainer_adapter: "qlora_shadow".to_string(),
    }
}

fn default_state(policy: &AdaptivePolicy) -> RuntimeState {
    let now = now_iso();
    RuntimeState {
        version: "1.0".to_string(),
        created_at: now.clone(),
        updated_at: now.clone(),
        active_mode: "logical_only".to_string(),
        local_only: policy.local_only,
        logical: ModelProfile {
            role: "logical".to_string(),
            seed_model: policy.seed_model.clone(),
            active_model: policy.logical_model.clone(),
            specialization_score_pct: 0.0,
            graduated: false,
            last_trained_at: None,
            last_graduated_at: None,
            trainer_adapter: policy.trainer_adapter.clone(),
        },
        creative: ModelProfile {
            role: "creative".to_string(),
            seed_model: policy.seed_model.clone(),
            active_model: policy.creative_model.clone(),
            specialization_score_pct: 0.0,
            graduated: false,
            last_trained_at: None,
            last_graduated_at: None,
            trainer_adapter: policy.trainer_adapter.clone(),
        },
        training: TrainingState {
            cycles_completed: 0,
            last_job_id: None,
            last_context_digest: None,
            last_mode: None,
            nightly_due: true,
            last_trained_at: None,
        },
        last_proposal_digest: None,
        last_connector_digest: None,
        last_resource_mode: None,
    }
}

fn load_policy(root: &Path) -> AdaptivePolicy {
    let path = policy_path(root);
    if !path.exists() {
        return default_policy();
    }
    let raw = match fs::read_to_string(&path) {
        Ok(raw) => raw,
        Err(_) => return default_policy(),
    };
    serde_json::from_str::<AdaptivePolicy>(&raw).unwrap_or_else(|_| default_policy())
}

fn load_state(root: &Path, policy: &AdaptivePolicy) -> RuntimeState {
    let path = runtime_state_path(root);
    let Some(value) = read_json(&path) else {
        return default_state(policy);
    };
    serde_json::from_value::<RuntimeState>(value).unwrap_or_else(|_| default_state(policy))
}

fn store_state(root: &Path, state: &RuntimeState) -> Result<(), String> {
    let path = runtime_state_path(root);
    let value =
        serde_json::to_value(state).map_err(|err| format!("adaptive_state_encode_failed:{err}"))?;
    write_json(&path, &value)
}

fn read_jsonl(path: &Path) -> Vec<Value> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
        .collect()
}

fn extract_text(value: &Value) -> String {
    for key in ["text", "content", "summary", "insight", "message", "note"] {
        if let Some(text) = value.get(key).and_then(Value::as_str) {
            let cleaned = clean(text, 240);
            if !cleaned.is_empty() {
                return cleaned;
            }
        }
    }
    clean(
        serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string()),
        240,
    )
}

fn collect_context_bundle(root: &Path, flags: &HashMap<String, String>) -> ContextBundle {
    let conversation_samples = read_jsonl(&conversation_eye_path(root))
        .into_iter()
        .rev()
        .take(12)
        .map(|row| extract_text(&row))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    let dream_samples = read_jsonl(&dream_log_path(root))
        .into_iter()
        .rev()
        .take(8)
        .map(|row| extract_text(&row))
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    let persona = clean(
        flags
            .get("persona")
            .cloned()
            .unwrap_or_else(|| "default".to_string()),
        80,
    );
    let logical_bias = clean(
        flags
            .get("logical-bias")
            .cloned()
            .unwrap_or_else(|| "precise planning".to_string()),
        160,
    );
    let creative_bias = clean(
        flags
            .get("creative-bias")
            .cloned()
            .unwrap_or_else(|| "divergent synthesis".to_string()),
        160,
    );
    let digest = sha256_hex_str(&format!(
        "{}|{}|{}|{}|{}",
        persona,
        logical_bias,
        creative_bias,
        conversation_samples.join("|"),
        dream_samples.join("|")
    ));
    ContextBundle {
        conversation_samples,
        dream_samples,
        interaction_digest: digest,
        persona,
        logical_bias,
        creative_bias,
    }
}

fn detect_ram_gb() -> f64 {
    #[cfg(target_os = "macos")]
    {
        if let Ok(output) = Command::new("sysctl").args(["-n", "hw.memsize"]).output() {
            if output.status.success() {
                if let Ok(text) = String::from_utf8(output.stdout) {
                    if let Ok(bytes) = text.trim().parse::<f64>() {
                        return bytes / 1024.0 / 1024.0 / 1024.0;
                    }
                }
            }
        }
    }
    #[cfg(target_os = "linux")]
    {
        if let Ok(raw) = fs::read_to_string("/proc/meminfo") {
            for line in raw.lines() {
                if let Some(rest) = line.strip_prefix("MemTotal:") {
                    let kb = rest
                        .split_whitespace()
                        .next()
                        .and_then(|v| v.parse::<f64>().ok())
                        .unwrap_or(0.0);
                    return kb / 1024.0 / 1024.0;
                }
            }
        }
    }
    8.0
}

fn resource_snapshot(flags: &HashMap<String, String>, policy: &AdaptivePolicy) -> ResourceSnapshot {
    let vram_gb = parse_f64(flags.get("vram-gb"), 0.0);
    let ram_gb = parse_f64(flags.get("ram-gb"), detect_ram_gb());
    let cpu_cores = parse_u64(
        flags.get("cpu-cores"),
        std::thread::available_parallelism()
            .map(|v| v.get() as u64)
            .unwrap_or(4),
    );
    let mode = if vram_gb >= policy.resource_thresholds.dual_vram_gb
        && ram_gb >= policy.resource_thresholds.dual_ram_gb
        && cpu_cores >= policy.resource_thresholds.dual_cpu_cores
    {
        "dual".to_string()
    } else if vram_gb >= policy.resource_thresholds.logical_vram_gb
        && ram_gb >= policy.resource_thresholds.logical_ram_gb
    {
        "logical_only".to_string()
    } else {
        "tiny_logical_only".to_string()
    };
    let degraded = mode != "dual";
    ResourceSnapshot {
        vram_gb,
        ram_gb,
        cpu_cores,
        mode,
        degraded,
    }
}

fn command_exists(name: &str) -> bool {
    Command::new("sh")
        .arg("-lc")
        .arg(format!("command -v {} >/dev/null 2>&1", clean(name, 120)))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn is_local_model(model: &str) -> bool {
    let model = model.trim();
    !model.is_empty() && (model.starts_with("ollama/") || model.starts_with("local/"))
}

fn ollama_model_name(model_id: &str) -> String {
    model_id
        .trim()
        .trim_start_matches("ollama/")
        .trim_start_matches("local/")
        .to_string()
}

