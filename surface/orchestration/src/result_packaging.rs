// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{
    ControlPlaneDecisionTrace, ControlPlaneLifecycleState, OrchestrationFallbackAction,
    OrchestrationPlan, OrchestrationResultPackage, PlanStatus, PlanVariant, RequestClass,
    RuntimeQualitySignals, WorkflowTemplate,
};

pub fn package_result(
    plan: &OrchestrationPlan,
    progress_message: String,
    recovery_applied: bool,
    fallback_actions: Vec<OrchestrationFallbackAction>,
    workflow_template: WorkflowTemplate,
    control_plane_lifecycle: ControlPlaneLifecycleState,
) -> OrchestrationResultPackage {
    shape_result_package(
        plan,
        progress_message,
        recovery_applied,
        fallback_actions,
        workflow_template,
        control_plane_lifecycle,
    )
}

pub fn shape_result_package(
    plan: &OrchestrationPlan,
    progress_message: String,
    recovery_applied: bool,
    fallback_actions: Vec<OrchestrationFallbackAction>,
    workflow_template: WorkflowTemplate,
    control_plane_lifecycle: ControlPlaneLifecycleState,
) -> OrchestrationResultPackage {
    let requires_core_promotion = matches!(
        plan.request_class,
        RequestClass::Mutation | RequestClass::TaskProposal | RequestClass::Assimilation
    );
    let summary = summary_for_plan(plan);
    let runtime_quality = runtime_quality_signals(plan, fallback_actions.len() as u32);
    let decision_trace = decision_trace(plan);
    let mut execution_state = plan.execution_state.clone();
    execution_state.correlation.receipt_metadata.decision_trace = decision_trace.clone();

    OrchestrationResultPackage {
        summary,
        progress_message,
        execution_state,
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
        decision_trace,
        workflow_template,
        control_plane_lifecycle,
    }
}

fn summary_for_plan(plan: &OrchestrationPlan) -> String {
    if plan.needs_clarification {
        return "orchestration requires clarification".to_string();
    }
    match plan.execution_state.plan_status {
        PlanStatus::Running => "orchestration is tracking in-flight core execution".to_string(),
        PlanStatus::Completed => {
            "orchestration completed with correlated core execution".to_string()
        }
        PlanStatus::Failed => "orchestration observed a failed core execution outcome".to_string(),
        PlanStatus::Degraded => {
            "orchestration prepared degraded plan for core contract execution".to_string()
        }
        _ => "orchestration ready for core contract execution".to_string(),
    }
}

fn runtime_quality_signals(
    plan: &OrchestrationPlan,
    fallback_action_count: u32,
) -> RuntimeQualitySignals {
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
    let typed_probe_contract_gap_count = plan
        .classification
        .reasons
        .iter()
        .filter(|reason| reason.starts_with("typed_probe_contract_missing"))
        .count() as u32;
    let decision_rationale_count = if !plan.selected_plan.reasons.is_empty() {
        plan.selected_plan.reasons.len() as u32
    } else {
        plan.selected_plan
            .steps
            .iter()
            .flat_map(|step| step.rationale.iter())
            .count() as u32
    };

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
        typed_probe_contract_gap_count,
        decision_rationale_count,
        fallback_action_count,
    }
}

fn decision_trace(plan: &OrchestrationPlan) -> ControlPlaneDecisionTrace {
    let rationale = if !plan.selected_plan.reasons.is_empty() {
        plan.selected_plan.reasons.clone()
    } else {
        plan.selected_plan
            .steps
            .iter()
            .flat_map(|step| step.rationale.iter().cloned())
            .collect::<Vec<_>>()
    };
    ControlPlaneDecisionTrace {
        chosen: plan.selected_plan.plan_id.clone(),
        alternatives_rejected: plan
            .alternative_plans
            .iter()
            .map(|candidate| candidate.plan_id.clone())
            .collect(),
        confidence: plan.selected_plan.confidence,
        rationale,
    }
}
