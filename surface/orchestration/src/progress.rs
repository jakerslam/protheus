// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{
    ControlPlaneDecisionTrace, CoreExecutionObservation, DegradationState, ExecutionCorrelation,
    ExecutionState, OrchestrationPlan, PlanCandidate, PlanStatus, ReceiptDebugMetadata,
    RecoveryReason, StepState, StepStatus, TypedOrchestrationRequest,
};

pub fn project_execution_state(
    request: &TypedOrchestrationRequest,
    execution_observation: Option<&CoreExecutionObservation>,
    plan: &PlanCandidate,
    needs_clarification: bool,
) -> ExecutionState {
    let correlation = correlation_for(request, execution_observation, plan);
    let step_statuses = observed_step_statuses(execution_observation);
    let observed_plan =
        observed_or_derived_plan_status(execution_observation, step_statuses.as_slice());
    let has_observation = execution_observation.is_some();
    if !plan.degradation.is_empty() {
        let alternate_path = plan
            .steps
            .iter()
            .map(|row| row.target_contract.clone())
            .collect::<Vec<_>>();
        return ExecutionState {
            plan_status: if let Some(observed) = observed_plan {
                observed
            } else if needs_clarification || plan.requires_clarification {
                PlanStatus::ClarificationRequired
            } else {
                PlanStatus::Degraded
            },
            steps: plan
                .steps
                .iter()
                .map(|row| StepState {
                    step_id: row.step_id.clone(),
                    status: if let Some(observed) =
                        observed_step_status_for(&step_statuses, &row.step_id)
                    {
                        observed
                    } else if has_observation {
                        StepStatus::Pending
                    } else {
                        StepStatus::Degraded
                    },
                    blocked_on: row.blocked_on.clone(),
                })
                .collect(),
            recovery: None,
            degradation: Some(DegradationState {
                reasons: plan.degradation.clone(),
                alternate_path,
                note: "planner selected degraded alternate path".to_string(),
            }),
            correlation,
        };
    }

    let plan_status = if let Some(observed) = observed_plan {
        observed
    } else if needs_clarification || plan.requires_clarification {
        PlanStatus::ClarificationRequired
    } else if !plan.blocked_on.is_empty() || plan.steps.is_empty() {
        PlanStatus::Blocked
    } else {
        PlanStatus::Ready
    };

    ExecutionState {
        plan_status,
        steps: plan
            .steps
            .iter()
            .map(|row| StepState {
                step_id: row.step_id.clone(),
                status: if let Some(observed) =
                    observed_step_status_for(&step_statuses, &row.step_id)
                {
                    observed
                } else if has_observation {
                    if row.blocked_on.is_empty() {
                        StepStatus::Pending
                    } else {
                        StepStatus::Blocked
                    }
                } else if row.blocked_on.is_empty() {
                    StepStatus::Ready
                } else {
                    StepStatus::Blocked
                },
                blocked_on: row.blocked_on.clone(),
            })
            .collect(),
        recovery: None,
        degradation: None,
        correlation,
    }
}

// Compatibility alias during control-plane naming transition.
pub fn execution_state_for(
    request: &TypedOrchestrationRequest,
    execution_observation: Option<&CoreExecutionObservation>,
    plan: &PlanCandidate,
    needs_clarification: bool,
) -> ExecutionState {
    project_execution_state(request, execution_observation, plan, needs_clarification)
}

pub fn build_progress_projection(plan: &OrchestrationPlan) -> String {
    let posture = format!("{:?}", plan.posture).to_lowercase();
    let status = format!("{:?}", plan.execution_state.plan_status).to_lowercase();
    let blocked = plan.selected_plan.blocked_on.len();
    let failed_steps = plan
        .execution_state
        .steps
        .iter()
        .filter(|row| row.status == StepStatus::Failed)
        .count();
    let probe_gaps = plan
        .classification
        .reasons
        .iter()
        .filter(|reason| reason.starts_with("typed_probe_contract_missing"))
        .count();
    let heuristic_probe_count = plan
        .selected_plan
        .capability_probes
        .iter()
        .flat_map(|probe| probe.probe_sources.iter())
        .filter(|source| source.starts_with("heuristic."))
        .count();
    let tool_failure_budget_exceeded = plan
        .execution_state
        .recovery
        .as_ref()
        .and_then(|row| row.reason.as_ref())
        .map(|reason| reason == &RecoveryReason::ToolFailureBudgetExceeded)
        .unwrap_or(false);
    format!(
        "orchestration posture={} status={} steps={} blocked={} failed_steps={} probe_gaps={} heuristic_probes={} tool_failure_budget_exceeded={} clarification={} confidence={:.2}",
        posture,
        status,
        plan.selected_plan.steps.len(),
        blocked,
        failed_steps,
        probe_gaps,
        heuristic_probe_count,
        tool_failure_budget_exceeded,
        plan.needs_clarification,
        plan.selected_plan.confidence
    )
}

// Compatibility alias during control-plane naming transition.
pub fn progress_message(plan: &OrchestrationPlan) -> String {
    build_progress_projection(plan)
}

fn observed_plan_status(
    execution_observation: Option<&CoreExecutionObservation>,
) -> Option<PlanStatus> {
    execution_observation.and_then(|row| row.plan_status.clone())
}

fn observed_or_derived_plan_status(
    execution_observation: Option<&CoreExecutionObservation>,
    observed_step_statuses: &[(String, StepStatus)],
) -> Option<PlanStatus> {
    if let Some(observed) = observed_plan_status(execution_observation) {
        return Some(observed);
    }
    let observation = execution_observation?;
    if observed_step_statuses
        .iter()
        .any(|(_, status)| matches!(status, StepStatus::Failed))
    {
        return Some(PlanStatus::Failed);
    }
    if observed_step_statuses
        .iter()
        .any(|(_, status)| matches!(status, StepStatus::Running))
    {
        return Some(PlanStatus::Running);
    }
    if observed_step_statuses
        .iter()
        .any(|(_, status)| matches!(status, StepStatus::Blocked))
    {
        return Some(PlanStatus::Blocked);
    }
    if observed_step_statuses
        .iter()
        .any(|(_, status)| matches!(status, StepStatus::Degraded))
    {
        return Some(PlanStatus::Degraded);
    }
    if !observed_step_statuses.is_empty()
        && observed_step_statuses
            .iter()
            .all(|(_, status)| matches!(status, StepStatus::Succeeded | StepStatus::Skipped))
    {
        return Some(PlanStatus::Completed);
    }
    if !observation.outcome_refs.is_empty() {
        return Some(PlanStatus::Completed);
    }
    if !observation.receipt_ids.is_empty() || !observation.step_statuses.is_empty() {
        return Some(PlanStatus::Running);
    }
    None
}

fn observed_step_statuses(
    execution_observation: Option<&CoreExecutionObservation>,
) -> Vec<(String, StepStatus)> {
    execution_observation
        .map(|row| {
            row.step_statuses
                .iter()
                .map(|step| (step.step_id.clone(), step.status.clone()))
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn observed_step_status_for(
    step_statuses: &[(String, StepStatus)],
    step_id: &str,
) -> Option<StepStatus> {
    step_statuses
        .iter()
        .find(|(candidate, _)| candidate == step_id)
        .map(|(_, status)| status.clone())
}

fn correlation_for(
    request: &TypedOrchestrationRequest,
    execution_observation: Option<&CoreExecutionObservation>,
    plan: &PlanCandidate,
) -> ExecutionCorrelation {
    ExecutionCorrelation {
        orchestration_trace_id: format!(
            "orch_{}_{}",
            request.session_id,
            plan.plan_id
                .replace(|ch: char| !ch.is_ascii_alphanumeric(), "")
        ),
        expected_core_contract_ids: {
            let mut ids = plan
                .steps
                .iter()
                .flat_map(|row| row.expected_contract_refs.iter().cloned())
                .collect::<Vec<_>>();
            ids.sort();
            ids.dedup();
            ids
        },
        observed_core_receipt_ids: execution_observation
            .map(|row| row.receipt_ids.clone())
            .unwrap_or_default(),
        observed_core_outcome_refs: execution_observation
            .map(|row| row.outcome_refs.clone())
            .unwrap_or_default(),
        receipt_metadata: ReceiptDebugMetadata {
            decision_trace: decision_trace_for_candidate(plan),
        },
    }
}

fn decision_trace_for_candidate(plan: &PlanCandidate) -> ControlPlaneDecisionTrace {
    let rationale = if !plan.reasons.is_empty() {
        plan.reasons.clone()
    } else {
        plan.steps
            .iter()
            .flat_map(|step| step.rationale.iter().cloned())
            .collect::<Vec<_>>()
    };
    ControlPlaneDecisionTrace {
        chosen: plan.plan_id.clone(),
        alternatives_rejected: Vec::new(),
        confidence: plan.confidence,
        rationale,
    }
}
