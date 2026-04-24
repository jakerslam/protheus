// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{
    ControlPlaneDecisionTrace, ControlPlaneLifecycleState, OrchestrationFallbackAction,
    OrchestrationPlan, OrchestrationResultPackage, PlanStatus, PlanVariant, RecoveryReason,
    RequestClass, RuntimeQualitySignals, StepStatus, WorkflowTemplate,
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
    let runtime_quality = runtime_quality_signals(plan, &fallback_actions, &workflow_template);
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
    fallback_actions: &[OrchestrationFallbackAction],
    workflow_template: &WorkflowTemplate,
) -> RuntimeQualitySignals {
    let is_forgecode_workflow = matches!(
        workflow_template,
        WorkflowTemplate::ForgeCodeAgentComposition
            | WorkflowTemplate::ForgeCodeRawCapabilityAssimilation
    );
    let fallback_action_count = fallback_actions.len() as u32;
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
    let marker_budget = tool_failure_budget_marker(plan);
    let failed_step_count = plan
        .execution_state
        .steps
        .iter()
        .filter(|row| row.status == StepStatus::Failed)
        .count() as u32;
    let tool_failure_budget_failed_step_count =
        marker_budget.map(|row| row.0).unwrap_or(failed_step_count);
    let tool_failure_budget_limit = marker_budget.map(|row| row.1).unwrap_or(0);
    let tool_failure_budget_exceeded = marker_budget.is_some()
        || matches!(
            plan.execution_state
                .recovery
                .as_ref()
                .and_then(|row| row.reason.as_ref()),
            Some(RecoveryReason::ToolFailureBudgetExceeded)
        );
    let mcp_retry_reason_count = mcp_retry_reason_count(plan);
    let mcp_transport_fallback_action_count = mcp_transport_fallback_action_count(fallback_actions);
    let mcp_retry_recovery_active = is_forgecode_workflow
        && (mcp_retry_reason_count > 0
            || mcp_transport_fallback_action_count > 0
            || tool_failure_budget_exceeded);
    let mcp_diagnostic_summary = build_mcp_diagnostic_summary(
        mcp_retry_reason_count,
        mcp_transport_fallback_action_count,
        tool_failure_budget_exceeded,
        fallback_action_count,
        workflow_template,
    );

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
        mcp_alias_route_required: is_forgecode_workflow,
        retry_backoff_contract_required: is_forgecode_workflow,
        mcp_transport_fallback_required: is_forgecode_workflow,
        semantic_discovery_route_required: is_forgecode_workflow,
        exact_pattern_search_required: is_forgecode_workflow,
        known_path_direct_read_required: is_forgecode_workflow,
        parallel_independent_tool_calls_required: is_forgecode_workflow,
        grounded_verification_required: is_forgecode_workflow,
        step_checkpointing_required: is_forgecode_workflow,
        completion_hygiene_required: is_forgecode_workflow,
        specialized_tool_usage_required: is_forgecode_workflow,
        shell_terminal_only_usage_required: is_forgecode_workflow,
        simple_lookup_locality_hygiene_required: is_forgecode_workflow,
        subagent_brief_contract_required: is_forgecode_workflow,
        subagent_output_contract_required: is_forgecode_workflow,
        subagent_result_synthesis_required: is_forgecode_workflow,
        mcp_retry_reason_count,
        mcp_transport_fallback_action_count,
        mcp_retry_recovery_active,
        mcp_diagnostic_summary,
        tool_failure_budget_failed_step_count,
        tool_failure_budget_limit,
        tool_failure_budget_exceeded,
    }
}

fn mcp_retry_reason_count(plan: &OrchestrationPlan) -> u32 {
    plan.classification
        .reasons
        .iter()
        .filter(|reason| {
            let lower = reason.to_ascii_lowercase();
            lower.contains("mcp")
                || lower.contains("retry")
                || lower.contains("backoff")
                || lower.contains("transport_fallback")
        })
        .count() as u32
}

fn mcp_transport_fallback_action_count(fallback_actions: &[OrchestrationFallbackAction]) -> u32 {
    fallback_actions
        .iter()
        .filter(|action| {
            let kind = action.kind.to_ascii_lowercase();
            let reason = action.reason.to_ascii_lowercase();
            kind.contains("mcp")
                || kind.contains("transport")
                || reason.contains("mcp")
                || reason.contains("transport")
                || reason.contains("http")
                || reason.contains("sse")
        })
        .count() as u32
}

fn build_mcp_diagnostic_summary(
    mcp_retry_reason_count: u32,
    mcp_transport_fallback_action_count: u32,
    tool_failure_budget_exceeded: bool,
    fallback_action_count: u32,
    workflow_template: &WorkflowTemplate,
) -> String {
    if !matches!(
        workflow_template,
        WorkflowTemplate::ForgeCodeAgentComposition
            | WorkflowTemplate::ForgeCodeRawCapabilityAssimilation
    ) {
        return "mcp_diag:not_applicable".to_string();
    }
    format!(
        "mcp_diag:retry_markers={mcp_retry_reason_count};transport_fallbacks={mcp_transport_fallback_action_count};fallbacks={fallback_action_count};budget_exceeded={tool_failure_budget_exceeded}"
    )
}

fn tool_failure_budget_marker(plan: &OrchestrationPlan) -> Option<(u32, u32)> {
    plan.classification.reasons.iter().find_map(|reason| {
        let payload = reason.strip_prefix("recovery:tool_failure_budget_exceeded:")?;
        let mut segments = payload.split(':');
        let failed = segments.next()?.parse::<u32>().ok()?;
        let limit = segments.next()?.parse::<u32>().ok()?;
        Some((failed, limit))
    })
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
