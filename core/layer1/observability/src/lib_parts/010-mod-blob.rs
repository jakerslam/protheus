// SPDX-License-Identifier: Apache-2.0
#[path = "../blob.rs"]
mod blob;

use infring_memory_core_v6::{
    load_embedded_observability_profile as load_embedded_profile_from_memory, EmbeddedChaosHook,
    EmbeddedObservabilityProfile,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::ffi::{CStr, CString};
use std::fmt::{Display, Formatter};
use std::os::raw::c_char;

pub use blob::{
    load_embedded_observability_runtime_envelope, BlobError, ObservabilityRuntimeEnvelope,
    OBS_RUNTIME_BLOB_ID,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct TraceEvent {
    pub trace_id: String,
    pub ts_millis: u64,
    pub source: String,
    pub operation: String,
    pub severity: String,
    pub tags: Vec<String>,
    pub payload_digest: String,
    pub signed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ChaosScenarioRequest {
    pub scenario_id: String,
    pub events: Vec<TraceEvent>,
    pub cycles: u32,
    pub inject_fault_every: u32,
    pub enforce_fail_closed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct TraceWindowReport {
    pub accepted_events: usize,
    pub dropped_events: usize,
    pub high_severity_events: usize,
    pub red_legion_channels_triggered: Vec<String>,
    pub event_digest: String,
    pub drift_score_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SovereigntyIndex {
    pub score_pct: f64,
    pub fail_closed: bool,
    pub status: String,
    pub reasons: Vec<String>,
    pub integrity_component_pct: f64,
    pub continuity_component_pct: f64,
    pub reliability_component_pct: f64,
    pub chaos_penalty_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChaosResilienceReport {
    pub profile_id: String,
    pub scenario_id: String,
    pub hooks_fired: Vec<String>,
    pub trace_report: TraceWindowReport,
    pub sovereignty: SovereigntyIndex,
    pub telemetry_overhead_ms: f64,
    pub chaos_battery_pct_24h: f64,
    pub resilient: bool,
}

#[derive(Debug, Clone)]
pub enum ObservabilityError {
    ProfileLoadFailed(String),
    InvalidRequest(String),
    EncodeFailed(String),
}

impl Display for ObservabilityError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            ObservabilityError::ProfileLoadFailed(msg) => write!(f, "profile_load_failed:{msg}"),
            ObservabilityError::InvalidRequest(msg) => write!(f, "invalid_request:{msg}"),
            ObservabilityError::EncodeFailed(msg) => write!(f, "encode_failed:{msg}"),
        }
    }
}

impl std::error::Error for ObservabilityError {}

fn normalize_text(input: &str, max: usize) -> String {
    input
        .trim()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max)
        .collect()
}

fn round3(value: f64) -> f64 {
    (value * 1000.0).round() / 1000.0
}

fn severity_weight(severity: &str) -> f64 {
    match severity.to_ascii_lowercase().as_str() {
        "critical" => 1.0,
        "high" => 0.7,
        "medium" => 0.35,
        "low" => 0.15,
        _ => 0.2,
    }
}

fn event_fingerprint(event: &TraceEvent) -> String {
    let mut parts: Vec<String> = vec![
        normalize_text(&event.trace_id, 160),
        event.ts_millis.to_string(),
        normalize_text(&event.source, 160),
        normalize_text(&event.operation, 160),
        normalize_text(&event.severity, 32),
        normalize_text(&event.payload_digest, 256),
        event.signed.to_string(),
    ];
    let mut tags = event.tags.clone();
    tags.sort();
    for tag in tags {
        parts.push(normalize_text(&tag, 80));
    }
    parts.join("|")
}

fn digest_lines(lines: &[String]) -> String {
    let mut hasher = Sha256::new();
    for (idx, line) in lines.iter().enumerate() {
        hasher.update(format!("{}:{}|", idx, line).as_bytes());
    }
    hex::encode(hasher.finalize())
}

fn capped_events(profile: &EmbeddedObservabilityProfile, events: &[TraceEvent]) -> Vec<TraceEvent> {
    events
        .iter()
        .take(profile.stream_policy.max_events_per_window.max(1) as usize)
        .cloned()
        .collect()
}

fn error_json(err: &ObservabilityError) -> String {
    serde_json::json!({ "ok": false, "error": err.to_string() }).to_string()
}

fn channel_triggered(channel: &str, events: &[TraceEvent]) -> bool {
    events.iter().any(|event| {
        event.tags.iter().any(|tag| {
            let t = tag.to_ascii_lowercase();
            let c = channel.to_ascii_lowercase();
            t == c || t.starts_with(&c)
        })
    })
}

fn hook_triggered(
    hook: &EmbeddedChaosHook,
    trace: &TraceWindowReport,
    events: &[TraceEvent],
) -> bool {
    if !hook.enabled {
        return false;
    }
    let cond = hook.condition.to_ascii_lowercase();

    if cond.contains("tamper") {
        return events.iter().any(|event| {
            let sev = event.severity.to_ascii_lowercase();
            sev == "critical"
                && event
                    .tags
                    .iter()
                    .any(|tag| tag.to_ascii_lowercase().contains("tamper"))
        });
    }
    if cond.contains("window.events") {
        return trace.dropped_events > 0;
    }
    if cond.contains("replay.drift") {
        return trace.drift_score_pct > 0.0;
    }
    false
}

fn continuity_component(events: &[TraceEvent], window_ms: u32) -> f64 {
    if events.len() <= 1 {
        return 100.0;
    }

    let mut monotonic_ok: usize = 0;
    let mut total_pairs: usize = 0;
    let mut gap_penalties: usize = 0;

    for pair in events.windows(2) {
        let left = &pair[0];
        let right = &pair[1];
        total_pairs += 1;
        if right.ts_millis >= left.ts_millis {
            monotonic_ok += 1;
        }
        if right.ts_millis.saturating_sub(left.ts_millis) > (window_ms as u64 * 2) {
            gap_penalties += 1;
        }
    }

    let monotonic_ratio = monotonic_ok as f64 / total_pairs as f64;
    let gap_penalty = gap_penalties as f64 / total_pairs as f64;
    let score = (monotonic_ratio * 100.0) - (gap_penalty * 35.0);
    score.clamp(0.0, 100.0)
}

fn reliability_component(events: &[TraceEvent], accepted_events: usize) -> f64 {
    if events.is_empty() {
        return 100.0;
    }
    let accepted_ratio = accepted_events as f64 / events.len() as f64;
    let severe_ratio = events
        .iter()
        .filter(|event| {
            let sev = event.severity.to_ascii_lowercase();
            sev == "critical" || sev == "high"
        })
        .count() as f64
        / events.len() as f64;

    let score = (accepted_ratio * 100.0) - (severe_ratio * 25.0);
    score.clamp(0.0, 100.0)
}

pub fn load_embedded_observability_profile(
) -> Result<EmbeddedObservabilityProfile, ObservabilityError> {
    load_embedded_profile_from_memory()
        .map_err(|err| ObservabilityError::ProfileLoadFailed(err.to_string()))
}
