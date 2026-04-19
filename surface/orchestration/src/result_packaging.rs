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
    let candidates = std::iter::once(&plan.selected_plan)
        .chain(plan.alternative_plans.iter())
        .collect::<Vec<_>>();
    let candidate_count = candidates.len() as u32;
    let selected_plan_degraded =
        matches!(plan.selected_plan.variant, PlanVariant::DegradedFallback)
            || !plan.selected_plan.degradation.is_empty()
            || matches!(plan.execution_state.plan_status, PlanStatus::Degraded);
    let selected_plan_requires_clarification =
        plan.needs_clarification || plan.selected_plan.requires_clarification;
    let heuristic_probe_source_count = plan
        .selected_plan
        .capability_probes
        .iter()
        .flat_map(|probe| probe.probe_sources.iter())
        .filter(|source| source.starts_with("heuristic."))
        .count() as u32;
    let used_heuristic_probe = heuristic_probe_source_count > 0;
    let blocked_precondition_count = plan.selected_plan.blocked_on.len() as u32;
    let executable_candidate_count = candidates
        .iter()
        .filter(|candidate| {
            !candidate.steps.is_empty()
                && candidate.blocked_on.is_empty()
                && !candidate.requires_clarification
        })
        .count() as u32;
    let degraded_candidate_count = candidates
        .iter()
        .filter(|candidate| {
            matches!(candidate.variant, PlanVariant::DegradedFallback)
                || !candidate.degradation.is_empty()
        })
        .count() as u32;
    let clarification_candidate_count = candidates
        .iter()
        .filter(|candidate| candidate.requires_clarification)
        .count() as u32;
    let zero_executable_candidates = executable_candidate_count == 0;
    let all_candidates_degraded =
        candidate_count > 0 && degraded_candidate_count == candidate_count;
    let all_candidates_require_clarification =
        candidate_count > 0 && clarification_candidate_count == candidate_count;

    RuntimeQualitySignals {
        candidate_count,
        selected_variant: plan.selected_plan.variant.clone(),
        selected_plan_degraded,
        selected_plan_requires_clarification,
        used_heuristic_probe,
        heuristic_probe_source_count,
        blocked_precondition_count,
        executable_candidate_count,
        degraded_candidate_count,
        clarification_candidate_count,
        zero_executable_candidates,
        all_candidates_degraded,
        all_candidates_require_clarification,
        surface_adapter_fallback: plan.classification.surface_adapter_fallback,
    }
}
