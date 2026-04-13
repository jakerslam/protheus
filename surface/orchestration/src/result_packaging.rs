// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{
    OrchestrationFallbackAction, OrchestrationPlan, OrchestrationResultPackage, PlanStatus,
    RequestClass,
};

pub fn package_result(
    plan: &OrchestrationPlan,
    progress_message: String,
    recovery_applied: bool,
    fallback_actions: Vec<OrchestrationFallbackAction>,
) -> OrchestrationResultPackage {
    let requires_core_promotion = matches!(
        plan.request_class,
        RequestClass::Mutation | RequestClass::TaskProposal | RequestClass::Assimilation
    );
    let summary = if plan.needs_clarification {
        "orchestration requires clarification".to_string()
    } else if matches!(plan.execution_state.plan_status, PlanStatus::Running) {
        "orchestration is tracking in-flight core execution".to_string()
    } else if matches!(plan.execution_state.plan_status, PlanStatus::Completed) {
        "orchestration completed with correlated core execution".to_string()
    } else if matches!(plan.execution_state.plan_status, PlanStatus::Failed) {
        "orchestration observed a failed core execution outcome".to_string()
    } else if matches!(plan.execution_state.plan_status, PlanStatus::Degraded) {
        "orchestration prepared degraded plan for core contract execution".to_string()
    } else {
        "orchestration ready for core contract execution".to_string()
    };

    OrchestrationResultPackage {
        summary,
        progress_message,
        execution_state: plan.execution_state.clone(),
        recovery_applied,
        fallback_actions,
        core_contract_calls: plan
            .selected_plan
            .steps
            .iter()
            .map(|row| row.target_contract.clone())
            .collect(),
        requires_core_promotion,
        classification: plan.classification.clone(),
        selected_plan: plan.selected_plan.clone(),
        alternative_plans: plan.alternative_plans.clone(),
    }
}
