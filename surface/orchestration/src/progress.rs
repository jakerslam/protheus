use crate::contracts::{
    DegradationState, ExecutionState, OrchestrationPlan, PlanCandidate, PlanStatus, StepState,
    StepStatus,
};

pub fn execution_state_for(plan: &PlanCandidate, needs_clarification: bool) -> ExecutionState {
    if let Some(reason) = plan.degradation.clone() {
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
                    status: StepStatus::Degraded,
                    blocked_on: row.blocked_on.clone(),
                })
                .collect(),
            recovery: None,
            degradation: Some(DegradationState {
                reason,
                alternate_path,
                note: "planner selected degraded alternate path".to_string(),
            }),
        };
    }

    let plan_status = if needs_clarification || plan.requires_clarification {
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
                status: if row.blocked_on.is_empty() {
                    StepStatus::Ready
                } else {
                    StepStatus::Blocked
                },
                blocked_on: row.blocked_on.clone(),
            })
            .collect(),
        recovery: None,
        degradation: None,
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
