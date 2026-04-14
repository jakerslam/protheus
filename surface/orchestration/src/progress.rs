// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{
    DegradationState, ExecutionCorrelation, ExecutionState, OrchestrationPlan, PlanCandidate,
    PlanStatus, StepState, StepStatus, TypedOrchestrationRequest,
};

pub fn execution_state_for(
    request: &TypedOrchestrationRequest,
    plan: &PlanCandidate,
    needs_clarification: bool,
) -> ExecutionState {
    let correlation = correlation_for(request, plan);
    let step_statuses = observed_step_statuses(request);
    if !plan.degradation.is_empty() {
        let alternate_path = plan
            .steps
            .iter()
            .map(|row| row.target_contract.clone())
            .collect::<Vec<_>>();
        return ExecutionState {
            plan_status: if needs_clarification || plan.requires_clarification {
                PlanStatus::ClarificationRequired
            } else {
                PlanStatus::Degraded
            },
            steps: plan
                .steps
                .iter()
                .map(|row| StepState {
                    step_id: row.step_id.clone(),
                    status: observed_step_status_for(&step_statuses, &row.step_id)
                        .or_else(|| observed_step_status(request))
                        .unwrap_or(StepStatus::Degraded),
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

    let plan_status = if let Some(observed) = observed_plan_status(request) {
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
                } else if let Some(observed) = observed_step_status(request) {
                    observed
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

pub fn progress_message(plan: &OrchestrationPlan) -> String {
    let posture = format!("{:?}", plan.posture).to_lowercase();
    let status = format!("{:?}", plan.execution_state.plan_status).to_lowercase();
    format!(
        "orchestration posture={} status={} steps={} clarification={} confidence={:.2}",
        posture,
        status,
        plan.selected_plan.steps.len(),
        plan.needs_clarification,
        plan.selected_plan.confidence
    )
}

fn observed_plan_status(request: &TypedOrchestrationRequest) -> Option<PlanStatus> {
    request
        .core_execution_observation
        .as_ref()
        .and_then(|row| row.plan_status.clone())
}

fn observed_step_status(request: &TypedOrchestrationRequest) -> Option<StepStatus> {
    match observed_plan_status(request) {
        Some(PlanStatus::Running) => Some(StepStatus::Running),
        Some(PlanStatus::Completed) => Some(StepStatus::Succeeded),
        Some(PlanStatus::Failed) => Some(StepStatus::Failed),
        _ => None,
    }
}

fn observed_step_statuses(request: &TypedOrchestrationRequest) -> Vec<(String, StepStatus)> {
    request
        .core_execution_observation
        .as_ref()
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
    plan: &PlanCandidate,
) -> ExecutionCorrelation {
    let observation = request.core_execution_observation.as_ref();
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
        observed_core_receipt_ids: observation
            .map(|row| row.receipt_ids.clone())
            .unwrap_or_default(),
        observed_core_outcome_refs: observation
            .map(|row| row.outcome_refs.clone())
            .unwrap_or_default(),
    }
}
