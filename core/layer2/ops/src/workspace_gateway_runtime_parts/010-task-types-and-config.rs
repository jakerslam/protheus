// Layer ownership: core/layer2/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use crate::ops_lane_runtime::{run_lane, LaneSpec};
use crate::{deterministic_receipt_hash, now_epoch_ms};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::thread;
use std::time::Duration;
use uuid::Uuid;

const LEGACY_USAGE: &[&str] = &[
    "Usage:",
    "  infring-ops workspace-gateway-runtime run|status|bootstrap|gateway|dmscope|heartbeat [--policy=<path>] [--state-path=<path>] [--strict=1|0]",
];

const TASK_USAGE: &[&str] = &[
    "Usage:",
    "  infring-ops workspace-gateway-runtime task submit [--kind=<id>] [--estimated-seconds=<n>] [--steps=<n>] [--bus=<auto|nats|local>]",
    "  infring-ops workspace-gateway-runtime task status <ticket_id>",
    "  infring-ops workspace-gateway-runtime task list [--limit=<n>]",
    "  infring-ops workspace-gateway-runtime task cancel <ticket_id>",
    "  infring-ops workspace-gateway-runtime task worker [--max-tasks=<n>] [--wait-ms=<n>] [--idle-hibernate-ms=<n>] [--service=1|0] [--bus=<auto|nats|local>]",
    "  infring-ops workspace-gateway-runtime task slow-test [--seconds=<n>] [--progress-interval-seconds=<n>] [--bus=<auto|nats|local>]",
];

const TASK_STATE_ROOT_ENV: &str = "INFRING_TASK_RUNTIME_STATE_ROOT";
const TASK_BUS_ENV: &str = "INFRING_TASK_BUS";
const TASK_NATS_URL_ENV: &str = "INFRING_TASK_NATS_URL";
const TASK_NATS_STREAM_ENV: &str = "INFRING_TASK_NATS_STREAM";
const TASK_NATS_SUBJECT_ENV: &str = "INFRING_TASK_NATS_SUBJECT";
const TASK_NATS_CANCEL_SUBJECT_ENV: &str = "INFRING_TASK_NATS_CANCEL_SUBJECT";
const TASK_NATS_DURABLE_ENV: &str = "INFRING_TASK_NATS_DURABLE";

const DEFAULT_NATS_URL: &str = "nats://127.0.0.1:4222";
const DEFAULT_NATS_STREAM: &str = "INFRING_TASKS";
const DEFAULT_NATS_SUBJECT: &str = "infring.tasks";
const DEFAULT_NATS_CANCEL_SUBJECT: &str = "infring.tasks.cancel";
const DEFAULT_NATS_DURABLE: &str = "infring-task-workers";
const DEFAULT_WORKER_IDLE_HIBERNATE_MS: u64 = 15_000;
const DEFAULT_WORKER_MIN_POLL_MS: u64 = 125;
const DEFAULT_WORKER_MAX_POLL_MS: u64 = 900;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TaskTicket {
    pub id: String,
    pub status: String,
    pub estimated_seconds: u64,
    pub bus_mode: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskPayload {
    id: String,
    kind: String,
    requested_by: String,
    estimated_seconds: u64,
    steps: u64,
    created_at_ms: u64,
    metadata: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskCancelEnvelope {
    task_id: String,
    ts_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProgressUpdate {
    id: String,
    progress_percent: u8,
    step: u64,
    total_steps: u64,
    message: String,
    ts_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskResult {
    id: String,
    status: String,
    summary: String,
    completed_at_ms: u64,
    duration_ms: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskRecord {
    id: String,
    kind: String,
    status: String,
    bus_mode: String,
    progress_percent: u8,
    estimated_seconds: u64,
    created_at_ms: u64,
    updated_at_ms: u64,
    cancelled: bool,
    result: Option<TaskResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct TaskRegistry {
    version: String,
    tasks: Vec<TaskRecord>,
}

#[derive(Debug, Clone)]
struct TaskPaths {
    root: PathBuf,
    registry_json: PathBuf,
    queue_jsonl: PathBuf,
    events_jsonl: PathBuf,
    conduit_jsonl: PathBuf,
    receipts_jsonl: PathBuf,
    cancelled_json: PathBuf,
    worker_state_json: PathBuf,
}

#[derive(Debug, Clone)]
struct ParsedCli {
    positional: Vec<String>,
    flags: BTreeMap<String, String>,
}

fn parse_cli(argv: &[String]) -> ParsedCli {
    let mut positional = Vec::<String>::new();
    let mut flags = BTreeMap::<String, String>::new();
    let mut idx = 0usize;
    while idx < argv.len() {
        let token = argv[idx].trim();
        if let Some(raw) = token.strip_prefix("--") {
            if let Some((k, v)) = raw.split_once('=') {
                flags.insert(k.trim().to_string(), v.trim().to_string());
            } else if let Some(next) = argv.get(idx + 1) {
                if !next.starts_with("--") {
                    flags.insert(raw.trim().to_string(), next.trim().to_string());
                    idx += 1;
                } else {
                    flags.insert(raw.trim().to_string(), "1".to_string());
                }
            } else {
                flags.insert(raw.trim().to_string(), "1".to_string());
            }
        } else {
            positional.push(argv[idx].clone());
        }
        idx += 1;
    }
    ParsedCli { positional, flags }
}

fn parse_u64_flag(flags: &BTreeMap<String, String>, key: &str, fallback: u64) -> u64 {
    flags
        .get(key)
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(fallback)
}

fn parse_non_empty(flags: &BTreeMap<String, String>, key: &str) -> Option<String> {
    flags.get(key).and_then(|raw| {
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn parse_bool_like(raw: &str) -> bool {
    matches!(
        raw.trim().to_ascii_lowercase().as_str(),
        "1" | "true" | "yes" | "on"
    )
}

fn parse_bool_flag(flags: &BTreeMap<String, String>, key: &str, fallback: bool) -> bool {
    flags.get(key).map(|raw| parse_bool_like(raw)).unwrap_or(fallback)
}

fn clean_id(raw: &str) -> String {
    let mut out = String::with_capacity(raw.len());
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
            out.push(ch.to_ascii_lowercase());
        }
    }
    out
}

fn env_non_empty(key: &str) -> Option<String> {
    std::env::var(key).ok().and_then(|value| {
        let trimmed = value.trim();
        if trimmed.is_empty() {
            None
        } else {
            Some(trimmed.to_string())
        }
    })
}

fn task_state_root(root: &Path) -> PathBuf {
    if let Some(override_root) = env_non_empty(TASK_STATE_ROOT_ENV) {
        return PathBuf::from(override_root);
    }
    root.join("local")
        .join("state")
        .join("runtime")
        .join("task_runtime")
}

fn task_paths(root: &Path) -> TaskPaths {
    let runtime_root = task_state_root(root);
    TaskPaths {
        registry_json: runtime_root.join("registry.json"),
        queue_jsonl: runtime_root.join("queue.jsonl"),
        events_jsonl: runtime_root.join("events.jsonl"),
        conduit_jsonl: runtime_root.join("conduit_messages.jsonl"),
        receipts_jsonl: runtime_root.join("verity_receipts.jsonl"),
        cancelled_json: runtime_root.join("cancelled.json"),
        worker_state_json: runtime_root.join("worker_state.json"),
        root: runtime_root,
    }
}

fn run_legacy_lane(root: &Path, argv: &[String]) -> i32 {
    run_lane(
        root,
        argv,
        &LaneSpec {
            lane_id: "workspace_gateway_runtime",
            lane_type: "workspace_gateway_runtime",
            replacement: "infring-ops workspace-gateway-runtime",
            usage: LEGACY_USAGE,
            passthrough_flags: &["strict", "policy", "state-path"],
        },
    )
}
