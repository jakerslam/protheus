// Layer ownership: surface/orchestration (control-plane trace normalization only).
use crate::contracts::{
    ControlPlaneDecisionTrace, OrchestrationResultPackage, WorkflowQualitySignals, WorkflowStage,
    WorkflowStageStatus, WorkflowTemplate,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const WORKFLOW_PHASE_TRACE_SCHEMA_VERSION: u32 = 1;
pub const WORKFLOW_PHASE_TRACE_TYPE: &str = "orchestration_workflow_phase_trace";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowPhaseTraceCollector {
    pub collector_id: String,
    pub role: String,
    pub path_hint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowPhaseTracePhase {
    pub phase: WorkflowStage,
    pub status: WorkflowStageStatus,
    pub owner: String,
    pub note: String,
    pub eval_visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct WorkflowPhaseTraceIssueSignal {
    pub signal_id: String,
    pub severity_hint: String,
    pub phase: Option<WorkflowStage>,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WorkflowPhaseTrace {
    #[serde(rename = "type")]
    pub trace_type: String,
    pub schema_version: u32,
    pub generated_at_ms: u64,
    pub owner: String,
    pub trace_id: String,
    pub user_intent: String,
    pub selected_workflow: String,
    pub selected_model: Option<String>,
    pub tool_decision: String,
    pub tool_family: String,
    pub tool_result_summary: String,
    pub finalization_status: String,
    pub fallback_path: String,
    pub latency_ms: Option<u64>,
    pub workflow_template: WorkflowTemplate,
    pub active_stage: WorkflowStage,
    pub phases: Vec<WorkflowPhaseTracePhase>,
    pub collectors: Vec<WorkflowPhaseTraceCollector>,
    pub decision_trace: ControlPlaneDecisionTrace,
    pub observed_kernel_receipt_ids: Vec<String>,
    pub observed_kernel_outcome_refs: Vec<String>,
    pub expected_kernel_contract_ids: Vec<String>,
    pub normalized_failure_codes: Vec<String>,
    pub issue_signals: Vec<WorkflowPhaseTraceIssueSignal>,
    pub receipt_hash: String,
}

pub fn default_workflow_phase_trace_collectors() -> Vec<WorkflowPhaseTraceCollector> {
    vec![
        WorkflowPhaseTraceCollector {
            collector_id: "kernel_runtime_receipts".to_string(),
            role: "authoritative_runtime_facts".to_string(),
            path_hint: "core/local/artifacts/** and local/state/attention/receipts.jsonl"
                .to_string(),
        },
        WorkflowPhaseTraceCollector {
            collector_id: "dashboard_troubleshooting_snapshot".to_string(),
            role: "shell_display_snapshot".to_string(),
            path_hint: "client/runtime/local/state/ui/infring_dashboard/troubleshooting/**"
                .to_string(),
        },
        WorkflowPhaseTraceCollector {
            collector_id: "attention_passive_memory".to_string(),
            role: "chat_turn_event_stream".to_string(),
            path_hint: "local/state/attention/queue.jsonl".to_string(),
        },
    ]
}

pub fn build_workflow_phase_trace(
    package: &OrchestrationResultPackage,
    generated_at_ms: u64,
) -> WorkflowPhaseTrace {
    let correlation = &package.execution_state.correlation;
    let mut trace = WorkflowPhaseTrace {
        trace_type: WORKFLOW_PHASE_TRACE_TYPE.to_string(),
        schema_version: WORKFLOW_PHASE_TRACE_SCHEMA_VERSION,
        generated_at_ms,
        owner: package.control_plane_lifecycle.owner.clone(),
        trace_id: correlation.orchestration_trace_id.clone(),
        user_intent: user_intent_for_package(package),
        selected_workflow: format!("{:?}", package.workflow_template).to_ascii_lowercase(),
        selected_model: None,
        tool_decision: tool_decision_for_package(package),
        tool_family: tool_family_for_package(package),
        tool_result_summary: tool_result_summary_for_package(package),
        finalization_status: format!("{:?}", package.execution_state.plan_status)
            .to_ascii_lowercase(),
        fallback_path: fallback_path_for_package(package),
        latency_ms: None,
        workflow_template: package.workflow_template.clone(),
        active_stage: package.control_plane_lifecycle.active_stage.clone(),
        phases: package
            .control_plane_lifecycle
            .stages
            .iter()
            .map(|stage| WorkflowPhaseTracePhase {
                phase: stage.stage.clone(),
                status: stage.status.clone(),
                owner: stage.owner.clone(),
                note: stage.note.clone(),
                eval_visible: true,
            })
            .collect(),
        collectors: default_workflow_phase_trace_collectors(),
        decision_trace: package.decision_trace.clone(),
        observed_kernel_receipt_ids: correlation.observed_core_receipt_ids.clone(),
        observed_kernel_outcome_refs: correlation.observed_core_outcome_refs.clone(),
        expected_kernel_contract_ids: correlation.expected_core_contract_ids.clone(),
        normalized_failure_codes: normalized_failure_codes(package),
        issue_signals: issue_signals(package),
        receipt_hash: String::new(),
    };
    trace.receipt_hash = trace_hash(&trace);
    trace
}

fn user_intent_for_package(package: &OrchestrationResultPackage) -> String {
    if package.progress_message.trim().is_empty() {
        "unavailable:result_package_lacks_original_user_intent".to_string()
    } else {
        package.progress_message.clone()
    }
}

fn tool_decision_for_package(package: &OrchestrationResultPackage) -> String {
    if package.core_contract_calls.is_empty() {
        "no_kernel_contract_call_selected".to_string()
    } else {
        format!(
            "kernel_contract_calls_selected:{}",
            package.core_contract_calls.len()
        )
    }
}

fn tool_family_for_package(package: &OrchestrationResultPackage) -> String {
    let forgecode_quality = package
        .workflow_quality
        .as_ref()
        .map(|WorkflowQualitySignals::ForgeCode(signals)| signals);
    if forgecode_quality
        .map(|signals| {
            signals.known_path_direct_read_required || signals.exact_pattern_search_required
        })
        .unwrap_or(false)
    {
        "workspace".to_string()
    } else if forgecode_quality
        .map(|signals| signals.semantic_discovery_route_required)
        .unwrap_or(false)
    {
        "web_or_semantic_discovery".to_string()
    } else if forgecode_quality
        .map(|signals| signals.shell_terminal_only_usage_required)
        .unwrap_or(false)
    {
        "shell_terminal".to_string()
    } else if forgecode_quality
        .map(|signals| signals.specialized_tool_usage_required)
        .unwrap_or(false)
        || !package.core_contract_calls.is_empty()
    {
        "tool_route".to_string()
    } else {
        "none".to_string()
    }
}

fn tool_result_summary_for_package(package: &OrchestrationResultPackage) -> String {
    format!(
        "contracts={};fallback_actions={};observed_receipts={};observed_outcomes={}",
        package.core_contract_calls.len(),
        package.fallback_actions.len(),
        package
            .execution_state
            .correlation
            .observed_core_receipt_ids
            .len(),
        package
            .execution_state
            .correlation
            .observed_core_outcome_refs
            .len()
    )
}

fn fallback_path_for_package(package: &OrchestrationResultPackage) -> String {
    if package.fallback_actions.is_empty() && !package.recovery_applied {
        return "none".to_string();
    }
    let action_labels = package
        .fallback_actions
        .iter()
        .map(|action| action.label.as_str())
        .collect::<Vec<_>>()
        .join(",");
    if action_labels.is_empty() {
        "recovery_applied".to_string()
    } else {
        format!("recovery_applied:{action_labels}")
    }
}

fn normalized_failure_codes(package: &OrchestrationResultPackage) -> Vec<String> {
    let mut codes = Vec::new();
    if package.progress_message.trim().is_empty() {
        codes.push("intent_unavailable_in_result_package".to_string());
    }
    if package.runtime_quality.zero_executable_candidates {
        codes.push("zero_executable_candidates".to_string());
    }
    if package.runtime_quality.tool_failure_budget_exceeded {
        codes.push("tool_failure_budget_exceeded".to_string());
    }
    if package.runtime_quality.typed_probe_contract_gap_count > 0 {
        codes.push("typed_probe_contract_gap".to_string());
    }
    if package.recovery_applied {
        codes.push("recovery_applied".to_string());
    }
    for stage in &package.control_plane_lifecycle.stages {
        if stage.status == WorkflowStageStatus::Blocked {
            codes.push(format!("phase_blocked:{:?}", stage.stage).to_ascii_lowercase());
        }
    }
    codes.sort();
    codes.dedup();
    codes
}

fn issue_signals(package: &OrchestrationResultPackage) -> Vec<WorkflowPhaseTraceIssueSignal> {
    let mut signals = Vec::new();
    for stage in &package.control_plane_lifecycle.stages {
        if stage.status == WorkflowStageStatus::Blocked {
            signals.push(WorkflowPhaseTraceIssueSignal {
                signal_id: "workflow_phase_blocked".to_string(),
                severity_hint: "high".to_string(),
                phase: Some(stage.stage.clone()),
                summary: stage.note.clone(),
            });
        }
    }
    if package.runtime_quality.zero_executable_candidates {
        signals.push(WorkflowPhaseTraceIssueSignal {
            signal_id: "zero_executable_candidates".to_string(),
            severity_hint: "medium".to_string(),
            phase: Some(WorkflowStage::DecompositionPlanning),
            summary: "planner produced no executable candidate".to_string(),
        });
    }
    if package.recovery_applied {
        signals.push(WorkflowPhaseTraceIssueSignal {
            signal_id: "recovery_applied".to_string(),
            severity_hint: "info".to_string(),
            phase: Some(WorkflowStage::RecoveryEscalation),
            summary: "control plane applied a recovery or fallback path".to_string(),
        });
    }
    signals
}

fn trace_hash(trace: &WorkflowPhaseTrace) -> String {
    let mut canonical = trace.clone();
    canonical.receipt_hash.clear();
    let payload = serde_json::to_vec(&canonical).unwrap_or_default();
    format!("{:x}", Sha256::digest(payload))
}
