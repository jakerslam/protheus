use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fs;
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

static ID_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Pending,
    InProgress,
    Complete,
    Failed,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SwarmEnvelope {
    pub id: String,
    pub reply_to: Option<String>,
    pub route: String,
    pub payload: Value,
    pub priority: u8,
    pub status: TaskStatus,
    pub created_unix_ms: u64,
}

impl SwarmEnvelope {
    pub fn new_auto(role_prefix: &str, route: &str, payload: Value, priority: u8) -> Self {
        Self {
            id: auto_id(role_prefix),
            reply_to: None,
            route: route.to_string(),
            payload,
            priority,
            status: TaskStatus::Pending,
            created_unix_ms: now_unix_ms(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("id_required".to_string());
        }
        if !self.id.contains('-') {
            return Err("id_must_include_prefix_separator".to_string());
        }
        if self.route.trim().is_empty() {
            return Err("route_required".to_string());
        }
        if self.priority > 9 {
            return Err("priority_out_of_range".to_string());
        }
        Ok(())
    }
}

pub fn auto_id(role_prefix: &str) -> String {
    let prefix = sanitize_prefix(role_prefix);
    let seq = ID_COUNTER.fetch_add(1, Ordering::SeqCst) + 1;
    format!("{}-{:016x}", prefix, seq)
}

fn sanitize_prefix(input: &str) -> String {
    let mut out = String::new();
    for ch in input.chars() {
        if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' {
            out.push(ch.to_ascii_lowercase());
        }
    }
    if out.is_empty() {
        "swarm".to_string()
    } else {
        out
    }
}

fn now_unix_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct InFlightRecord {
    pub owner: String,
    pub status: TaskStatus,
    pub updated_unix_ms: u64,
}

#[derive(Debug, Default, Clone, Serialize, Deserialize, PartialEq)]
pub struct InFlightTracker {
    pub tasks: HashMap<String, InFlightRecord>,
}

impl InFlightTracker {
    pub fn dispatch(&mut self, envelope: &SwarmEnvelope, owner: &str) -> Result<(), String> {
        envelope.validate()?;
        if let Some(existing) = self.tasks.get(&envelope.id) {
            if existing.status == TaskStatus::Pending || existing.status == TaskStatus::InProgress {
                return Err("in_flight_conflict".to_string());
            }
        }
        self.tasks.insert(
            envelope.id.clone(),
            InFlightRecord {
                owner: owner.to_string(),
                status: TaskStatus::InProgress,
                updated_unix_ms: now_unix_ms(),
            },
        );
        Ok(())
    }

    pub fn transition(&mut self, id: &str, status: TaskStatus) -> Result<(), String> {
        if let Some(task) = self.tasks.get_mut(id) {
            task.status = status;
            task.updated_unix_ms = now_unix_ms();
            return Ok(());
        }
        Err("task_not_found".to_string())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct RecoveryPolicy {
    pub max_retries: u32,
    pub fixer_route: String,
}

impl Default for RecoveryPolicy {
    fn default() -> Self {
        Self {
            max_retries: 2,
            fixer_route: "swarm/fixer".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryDecision {
    Retry { attempt: u32, route: String },
    RerouteFixer { route: String },
}

pub fn recovery_decision(
    envelope: &SwarmEnvelope,
    attempts: u32,
    policy: &RecoveryPolicy,
) -> RecoveryDecision {
    if attempts < policy.max_retries {
        RecoveryDecision::Retry {
            attempt: attempts + 1,
            route: envelope.route.clone(),
        }
    } else {
        RecoveryDecision::RerouteFixer {
            route: policy.fixer_route.clone(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScalingPolicy {
    pub min_workers: usize,
    pub max_workers: usize,
    pub target_queue_per_worker: usize,
    pub scale_step: usize,
}

impl Default for ScalingPolicy {
    fn default() -> Self {
        Self {
            min_workers: 1,
            max_workers: 32,
            target_queue_per_worker: 4,
            scale_step: 1,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ScalingDecision {
    pub previous_workers: usize,
    pub recommended_workers: usize,
    pub action: String,
    pub reason: String,
}

pub fn plan_scaling(
    queue_depth: usize,
    active_workers: usize,
    policy: &ScalingPolicy,
) -> ScalingDecision {
    let safe_workers = active_workers
        .max(policy.min_workers)
        .min(policy.max_workers);
    let desired = ((queue_depth + policy.target_queue_per_worker.saturating_sub(1))
        / policy.target_queue_per_worker)
        .max(policy.min_workers)
        .min(policy.max_workers);

    if desired > safe_workers {
        let bumped = (safe_workers + policy.scale_step)
            .min(desired)
            .min(policy.max_workers);
        return ScalingDecision {
            previous_workers: safe_workers,
            recommended_workers: bumped,
            action: "scale_up".to_string(),
            reason: format!(
                "queue_depth={} exceeds target_per_worker={} for workers={}",
                queue_depth, policy.target_queue_per_worker, safe_workers
            ),
        };
    }

    if desired < safe_workers {
        let reduced = safe_workers
            .saturating_sub(policy.scale_step)
            .max(desired)
            .max(policy.min_workers);
        return ScalingDecision {
            previous_workers: safe_workers,
            recommended_workers: reduced,
            action: "scale_down".to_string(),
            reason: format!(
                "queue_depth={} below target_per_worker={} for workers={}",
                queue_depth, policy.target_queue_per_worker, safe_workers
            ),
        };
    }

    ScalingDecision {
        previous_workers: safe_workers,
        recommended_workers: safe_workers,
        action: "hold".to_string(),
        reason: "queue pressure balanced".to_string(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct QueueArtifact {
    pub schema_version: String,
    pub updated_unix_ms: u64,
    pub items: Vec<SwarmEnvelope>,
}

impl Default for QueueArtifact {
    fn default() -> Self {
        Self {
            schema_version: "swarm_queue_v1".to_string(),
            updated_unix_ms: now_unix_ms(),
            items: Vec::new(),
        }
    }
}

impl QueueArtifact {
    pub fn validate(&self) -> Result<(), String> {
        if self.schema_version != "swarm_queue_v1" {
            return Err("queue_schema_mismatch".to_string());
        }
        for item in &self.items {
            item.validate()?;
        }
        Ok(())
    }

    pub fn push(&mut self, mut item: SwarmEnvelope) -> Result<(), String> {
        item.status = TaskStatus::Pending;
        item.validate()?;
        self.items.push(item);
        self.sort_priority();
        self.updated_unix_ms = now_unix_ms();
        Ok(())
    }

    pub fn sort_priority(&mut self) {
        self.items
            .sort_by(|a, b| b.priority.cmp(&a.priority).then_with(|| a.id.cmp(&b.id)));
    }

    pub fn save<P: AsRef<Path>>(&self, path: P) -> Result<(), String> {
        self.validate()?;
        if let Some(parent) = path.as_ref().parent() {
            fs::create_dir_all(parent).map_err(|e| format!("queue_parent_create_failed:{e}"))?;
        }
        let text =
            serde_json::to_string_pretty(self).map_err(|e| format!("queue_encode_failed:{e}"))?;
        fs::write(path, text).map_err(|e| format!("queue_write_failed:{e}"))
    }

    pub fn load<P: AsRef<Path>>(path: P) -> Result<Self, String> {
        if !path.as_ref().exists() {
            return Ok(Self::default());
        }
        let raw = fs::read_to_string(path).map_err(|e| format!("queue_read_failed:{e}"))?;
        let mut queue: Self =
            serde_json::from_str(&raw).map_err(|e| format!("queue_decode_failed:{e}"))?;
        queue.validate()?;
        queue.sort_priority();
        Ok(queue)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SwarmMetrics {
    pub queue_depth: usize,
    pub fail_rate: f64,
    pub retry_count: u64,
    pub completion_rate: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SwarmReceipt {
    pub schema_id: String,
    pub ts_unix_ms: u64,
    pub action: String,
    pub envelope_id: Option<String>,
    pub route: Option<String>,
    pub details: Value,
    pub metrics: Option<SwarmMetrics>,
}

pub fn build_metrics(
    total: u64,
    failed: u64,
    retried: u64,
    completed: u64,
    queue_depth: usize,
) -> SwarmMetrics {
    let total_nonzero = if total == 0 { 1.0 } else { total as f64 };
    SwarmMetrics {
        queue_depth,
        fail_rate: failed as f64 / total_nonzero,
        retry_count: retried,
        completion_rate: completed as f64 / total_nonzero,
    }
}

pub fn build_receipt(
    action: &str,
    envelope: Option<&SwarmEnvelope>,
    details: Value,
    metrics: Option<SwarmMetrics>,
) -> SwarmReceipt {
    SwarmReceipt {
        schema_id: "swarm_router_receipt_v1".to_string(),
        ts_unix_ms: now_unix_ms(),
        action: action.to_string(),
        envelope_id: envelope.map(|e| e.id.clone()),
        route: envelope.map(|e| e.route.clone()),
        details,
        metrics,
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpgradePolicy {
    pub allow_upgrade: bool,
    pub allow_rollback: bool,
}

impl Default for UpgradePolicy {
    fn default() -> Self {
        Self {
            allow_upgrade: true,
            allow_rollback: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct UpgradeReceipt {
    pub ok: bool,
    pub action: String,
    pub from_version: String,
    pub to_version: String,
    pub rollback_available: bool,
    pub ts_unix_ms: u64,
}

fn version_transition_receipt(
    allowed: bool,
    success_action: &str,
    denied_action: &str,
    from_version: &str,
    to_version: &str,
    rollback_available: bool,
) -> UpgradeReceipt {
    UpgradeReceipt {
        ok: allowed,
        action: if allowed {
            success_action.to_string()
        } else {
            denied_action.to_string()
        },
        from_version: from_version.to_string(),
        to_version: to_version.to_string(),
        rollback_available,
        ts_unix_ms: now_unix_ms(),
    }
}

pub fn apply_upgrade(
    from_version: &str,
    to_version: &str,
    policy: &UpgradePolicy,
) -> UpgradeReceipt {
    version_transition_receipt(
        policy.allow_upgrade,
        "upgrade_applied",
        "upgrade_denied",
        from_version,
        to_version,
        policy.allow_upgrade && policy.allow_rollback,
    )
}

pub fn apply_rollback(
    from_version: &str,
    to_version: &str,
    policy: &UpgradePolicy,
) -> UpgradeReceipt {
    version_transition_receipt(
        policy.allow_rollback,
        "rollback_applied",
        "rollback_denied",
        from_version,
        to_version,
        false,
    )
}

pub fn status_payload(
    queue: &QueueArtifact,
    tracker: &InFlightTracker,
    metrics: Option<SwarmMetrics>,
) -> Value {
    json!({
        "ok": true,
        "type": "swarm_router_status",
        "schema_version": queue.schema_version,
        "queue_depth": queue.items.len(),
        "in_flight": tracker.tasks.len(),
        "metrics": metrics
    })
}

#[cfg(test)]
mod tests;
