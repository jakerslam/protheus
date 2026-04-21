// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{
    ClarificationReason, DegradationReason, ExecutionPosture, OrchestrationPlan, PlanStatus,
    Precondition, RecoveryDecision, RecoveryReason, RecoveryState, StepStatus,
    TypedOrchestrationRequest,
};

pub fn coordinate_recovery_escalation(
    _request: &TypedOrchestrationRequest,
    mut plan: OrchestrationPlan,
) -> (OrchestrationPlan, bool) {
    if matches!(
        plan.execution_state.plan_status,
        PlanStatus::Running | PlanStatus::Completed | PlanStatus::Failed
    )
    {
        return (plan, false);
    }

    if apply_blocked_precondition_recovery(
        &mut plan,
        Precondition::TargetSupplied,
        "specify a target before execution",
        PlanStatus::ClarificationRequired,
        RecoveryDecision::Clarify,
        RecoveryReason::MissingTarget,
        true,
        "planner blocked on missing target",
    ) {
        return (plan, true);
    }

    if apply_blocked_precondition_recovery(
        &mut plan,
        Precondition::TargetSyntacticallyValid,
        "specify a valid target format before execution",
        PlanStatus::ClarificationRequired,
        RecoveryDecision::Clarify,
        RecoveryReason::TargetInvalid,
        true,
        "planner blocked on syntactically invalid target",
    ) {
        return (plan, true);
    }

    if apply_blocked_precondition_recovery(
        &mut plan,
        Precondition::TargetExists,
        "target could not be located for execution",
        PlanStatus::ClarificationRequired,
        RecoveryDecision::Clarify,
        RecoveryReason::TargetNotFound,
        true,
        "planner blocked because the requested target was not found",
    ) {
        return (plan, true);
    }

    if apply_blocked_precondition_recovery(
        &mut plan,
        Precondition::AuthorizationValid,
        "authorization is required before execution",
        PlanStatus::Blocked,
        RecoveryDecision::Clarify,
        RecoveryReason::AuthorizationFailure,
        true,
        "planner blocked on authorization",
    ) {
        return (plan, true);
    }

    if apply_blocked_precondition_recovery(
        &mut plan,
        Precondition::PolicyAllows,
        "policy scope must be narrowed before execution",
        PlanStatus::Blocked,
        RecoveryDecision::Halt,
        RecoveryReason::PolicyDenied,
        false,
        "planner blocked on policy",
    ) {
        return (plan, true);
    }

    if apply_transport_degradation_recovery(&mut plan) {
        return (plan, true);
    }

    if plan.selected_plan.steps.is_empty() {
        plan.posture = ExecutionPosture::Ask;
        plan.needs_clarification = true;
        plan.clarification_prompt = Some("no executable plan steps were generated".to_string());
        if !plan
            .classification
            .clarification_reasons
            .contains(&ClarificationReason::PlannerGap)
        {
            plan.classification
                .clarification_reasons
                .push(ClarificationReason::PlannerGap);
        }
        plan.classification.needs_clarification = true;
        plan.classification.confidence = plan.classification.confidence.min(0.40);
        plan.classification
            .reasons
            .push("planner:no_executable_steps".to_string());
        plan.execution_state.plan_status = PlanStatus::Blocked;
        plan.execution_state.recovery = Some(RecoveryState {
            decision: RecoveryDecision::Halt,
            reason: Some(RecoveryReason::PlannerContradiction),
            retryable: false,
            note: "planner emitted no executable steps".to_string(),
        });
        return (plan, true);
    }

    if !plan.selected_plan.degradation.is_empty() {
        let reason = recovery_reason_for_degradation(plan.selected_plan.degradation.first());
        let retryable = matches!(
            reason,
            RecoveryReason::ToolUnavailable | RecoveryReason::TransportFailure
        );
        plan.execution_state.plan_status = PlanStatus::Degraded;
        plan.execution_state.recovery = Some(RecoveryState {
            decision: RecoveryDecision::Degrade,
            reason: Some(reason),
            retryable,
            note: "planner selected degraded alternate path".to_string(),
        });
        return (plan, true);
    }

    (plan, false)
}

// Compatibility alias during control-plane naming transition.
pub fn apply_recovery_policy(
    request: &TypedOrchestrationRequest,
    plan: OrchestrationPlan,
) -> (OrchestrationPlan, bool) {
    coordinate_recovery_escalation(request, plan)
}

fn apply_blocked_precondition_recovery(
    plan: &mut OrchestrationPlan,
    blocked_precondition: Precondition,
    clarification_prompt: &str,
    plan_status: PlanStatus,
    decision: RecoveryDecision,
    reason: RecoveryReason,
    retryable: bool,
    note: &str,
) -> bool {
    if !plan.selected_plan.blocked_on.contains(&blocked_precondition) {
        return false;
    }

    plan.posture = ExecutionPosture::Ask;
    plan.needs_clarification = true;
    plan.clarification_prompt = Some(clarification_prompt.to_string());
    plan.execution_state.plan_status = plan_status;
    plan.execution_state.recovery = Some(RecoveryState {
        decision,
        reason: Some(reason),
        retryable,
        note: note.to_string(),
    });
    true
}

fn apply_transport_degradation_recovery(plan: &mut OrchestrationPlan) -> bool {
    if !plan
        .selected_plan
        .blocked_on
        .contains(&Precondition::TransportAvailable)
    {
        return false;
    }
    plan.execution_state.plan_status = PlanStatus::Degraded;
    plan.execution_state.recovery = Some(RecoveryState {
        decision: RecoveryDecision::Degrade,
        reason: Some(RecoveryReason::TransportFailure),
        retryable: true,
        note: "planner selected degraded path because transport is unavailable".to_string(),
    });
    for step in &mut plan.execution_state.steps {
        if step.status == StepStatus::Ready {
            step.status = StepStatus::Degraded;
        }
    }
    true
}

fn recovery_reason_for_degradation(reason: Option<&DegradationReason>) -> RecoveryReason {
    match reason {
        Some(DegradationReason::ToolUnavailable) => RecoveryReason::ToolUnavailable,
        Some(DegradationReason::TransportFailure) => RecoveryReason::TransportFailure,
        Some(DegradationReason::MissingTarget) => RecoveryReason::MissingTarget,
        Some(DegradationReason::TargetInvalid) => RecoveryReason::TargetInvalid,
        Some(DegradationReason::TargetNotFound) => RecoveryReason::TargetNotFound,
        Some(DegradationReason::AuthFailure) => RecoveryReason::AuthorizationFailure,
        Some(DegradationReason::PolicyDenied) => RecoveryReason::PolicyDenied,
        None => RecoveryReason::PlannerContradiction,
    }
}
