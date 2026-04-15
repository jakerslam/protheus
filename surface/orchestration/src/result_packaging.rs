// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{
    OrchestrationFallbackAction, OrchestrationPlan, OrchestrationResultPackage, PlanStatus,
    PlanVariant, RequestClass, RuntimeQualitySignals,
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
    let runtime_quality = runtime_quality_signals(plan);

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
        runtime_quality,
    }
}

fn runtime_quality_signals(plan: &OrchestrationPlan) -> RuntimeQualitySignals {
    let candidate_count = 1u32.saturating_add(plan.alternative_plans.len() as u32);
    let selected_plan_degraded = matches!(
        plan.selected_plan.variant,
        PlanVariant::DegradedFallback
    ) || !plan.selected_plan.degradation.is_empty()
        || matches!(plan.execution_state.plan_status, PlanStatus::Degraded);
    let selected_plan_requires_clarification =
        plan.needs_clarification || plan.selected_plan.requires_clarification;
    let used_heuristic_probe = plan.selected_plan.capability_probes.iter().any(|probe| {
        probe
            .probe_sources
            .iter()
            .any(|source| source.starts_with("heuristic."))
    });

    RuntimeQualitySignals {
        candidate_count,
        selected_variant: plan.selected_plan.variant.clone(),
        selected_plan_degraded,
        selected_plan_requires_clarification,
        used_heuristic_probe,
        surface_adapter_fallback: plan.classification.surface_adapter_fallback,
    }
}
