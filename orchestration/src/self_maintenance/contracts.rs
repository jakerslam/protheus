use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupervisorMode {
    ObserveOnly,
    ProposeOnly,
    ApplySafe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceSourceKind {
    ArchitectureAudit,
    DependencyViolation,
    TaskFabricSignal,
    CiReport,
    HealthMetric,
    MemoryPressure,
    OrphanedObject,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimType {
    Drift,
    Violation,
    Inefficiency,
    DeadCode,
    Health,
    Unknown,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimStatus {
    Supported,
    Partial,
    Unsupported,
    Conflicting,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum WorkerTaskStatus {
    Queued,
    Running,
    Completed,
    Blocked,
    Failed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RemediationClass {
    DocsDriftFix,
    PathCorrection,
    CleanupTask,
    BacklogHygiene,
    Unsafe,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SupervisorReceiptStage {
    Observation,
    Claim,
    TaskCreation,
    Execution,
    Outcome,
    Escalation,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ConfidenceVector {
    pub relevance: f64,
    pub reliability: f64,
    pub freshness: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EvidenceCard {
    pub evidence_id: String,
    pub source_kind: EvidenceSourceKind,
    pub source_ref: String,
    pub summary: String,
    pub details: Value,
    pub tags: Vec<String>,
    pub confidence_vector: ConfidenceVector,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Claim {
    pub claim_id: String,
    pub claim_type: ClaimType,
    pub text: String,
    pub evidence_ids: Vec<String>,
    pub status: ClaimStatus,
    pub confidence_vector: ConfidenceVector,
    pub conflict_refs: Vec<String>,
    pub remediation_class: RemediationClass,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ClaimBundle {
    pub claim_bundle_id: String,
    pub task_id: String,
    pub claims: Vec<Claim>,
    pub unresolved_questions: Vec<String>,
    pub conflicts: Vec<String>,
    pub coverage_score: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerBudgetUsed {
    pub tool_calls: usize,
    pub input_tokens: usize,
    pub output_tokens: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkerOutput {
    pub task_id: String,
    pub status: WorkerTaskStatus,
    pub produced_evidence_ids: Vec<String>,
    pub open_questions: Vec<String>,
    pub recommended_next_actions: Vec<String>,
    pub blockers: Vec<String>,
    pub budget_used: WorkerBudgetUsed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SupervisorReceipt {
    pub receipt_id: String,
    pub stage: SupervisorReceiptStage,
    pub detail: String,
    pub task_id: Option<String>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArchitectureAuditInput {
    pub audit_id: String,
    pub summary: String,
    pub severity: String,
    pub source_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DependencyViolationInput {
    pub violation_id: String,
    pub summary: String,
    pub source_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TaskFabricSignalInput {
    pub stale_tasks: Vec<String>,
    pub blocked_tasks: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CiReportInput {
    pub report_id: String,
    pub status: String,
    pub summary: String,
    pub source_ref: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HealthMetricInput {
    pub metric_name: String,
    pub observed: f64,
    pub threshold: f64,
    pub source_ref: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MemoryPressureInput {
    pub scope: String,
    pub used_bytes: u64,
    pub limit_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OrphanedObjectInput {
    pub object_id: String,
    pub summary: String,
    pub source_ref: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ObservationInputs {
    pub architecture_audits: Vec<ArchitectureAuditInput>,
    pub dependency_violations: Vec<DependencyViolationInput>,
    pub task_fabric_signals: TaskFabricSignalInput,
    pub ci_reports: Vec<CiReportInput>,
    pub health_metrics: Vec<HealthMetricInput>,
    pub memory_pressure: Vec<MemoryPressureInput>,
    pub orphaned_objects: Vec<OrphanedObjectInput>,
}

impl ObservationInputs {
    pub fn empty() -> Self {
        Self {
            architecture_audits: Vec::new(),
            dependency_violations: Vec::new(),
            task_fabric_signals: TaskFabricSignalInput {
                stale_tasks: Vec::new(),
                blocked_tasks: Vec::new(),
            },
            ci_reports: Vec::new(),
            health_metrics: Vec::new(),
            memory_pressure: Vec::new(),
            orphaned_objects: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct EscalationRequest {
    pub escalation_id: String,
    pub claim_bundle: ClaimBundle,
    pub proposed_diff: String,
    pub impact_analysis: String,
    pub rollback_plan: String,
    pub reason_codes: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SupervisorRunResult {
    pub mode: SupervisorMode,
    pub evidence: Vec<EvidenceCard>,
    pub claim_bundle: ClaimBundle,
    pub generated_task_ids: Vec<String>,
    pub worker_outputs: Vec<WorkerOutput>,
    pub escalation_requests: Vec<EscalationRequest>,
    pub receipts: Vec<SupervisorReceipt>,
}

pub fn confidence_average(vector: &ConfidenceVector) -> f64 {
    ((vector.relevance + vector.reliability + vector.freshness) / 3.0).clamp(0.0, 1.0)
}
