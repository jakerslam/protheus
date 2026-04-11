use crate::contracts::{
    OrchestrationFallbackAction, OrchestrationPlan, OrchestrationResultPackage, RequestClass,
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
    } else {
        "orchestration ready for core contract execution".to_string()
    };

    OrchestrationResultPackage {
        summary,
        progress_message,
        recovery_applied,
        fallback_actions,
        core_contract_calls: plan
            .steps
            .iter()
            .map(|row| row.target_contract.clone())
            .collect(),
        requires_core_promotion,
    }
}
