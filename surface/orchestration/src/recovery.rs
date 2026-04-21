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
        BlockedPreconditionRecoverySpec {
            precondition: Precondition::TargetSupplied,
            clarification_prompt: "specify a target before execution",
            plan_status: PlanStatus::ClarificationRequired,
            decision: RecoveryDecision::Clarify,
            reason: RecoveryReason::MissingTarget,
            retryable: true,
            note: "planner blocked on missing target",
        },
    ) {
        return (plan, true);
    }

    if apply_blocked_precondition_recovery(
        &mut plan,
        BlockedPreconditionRecoverySpec {
            precondition: Precondition::TargetSyntacticallyValid,
            clarification_prompt: "specify a valid target format before execution",
            plan_status: PlanStatus::ClarificationRequired,
            decision: RecoveryDecision::Clarify,
            reason: RecoveryReason::TargetInvalid,
            retryable: true,
            note: "planner blocked on syntactically invalid target",
        },
    ) {
        return (plan, true);
    }

    if apply_blocked_precondition_recovery(
        &mut plan,
        BlockedPreconditionRecoverySpec {
            precondition: Precondition::TargetExists,
            clarification_prompt: "target could not be located for execution",
            plan_status: PlanStatus::ClarificationRequired,
            decision: RecoveryDecision::Clarify,
            reason: RecoveryReason::TargetNotFound,
            retryable: true,
            note: "planner blocked because the requested target was not found",
        },
    ) {
        return (plan, true);
    }

    if apply_blocked_precondition_recovery(
        &mut plan,
        BlockedPreconditionRecoverySpec {
            precondition: Precondition::AuthorizationValid,
            clarification_prompt: "authorization is required before execution",
            plan_status: PlanStatus::Blocked,
            decision: RecoveryDecision::Clarify,
            reason: RecoveryReason::AuthorizationFailure,
            retryable: true,
            note: "planner blocked on authorization",
        },
    ) {
        return (plan, true);
    }

    if apply_blocked_precondition_recovery(
        &mut plan,
        BlockedPreconditionRecoverySpec {
            precondition: Precondition::PolicyAllows,
            clarification_prompt: "policy scope must be narrowed before execution",
            plan_status: PlanStatus::Blocked,
            decision: RecoveryDecision::Halt,
            reason: RecoveryReason::PolicyDenied,
            retryable: false,
            note: "planner blocked on policy",
        },
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
    spec: BlockedPreconditionRecoverySpec,
) -> bool {
    if !plan.selected_plan.blocked_on.contains(&spec.precondition) {
        return false;
    }

    plan.posture = ExecutionPosture::Ask;
    plan.needs_clarification = true;
    plan.clarification_prompt = Some(spec.clarification_prompt.to_string());
    plan.execution_state.plan_status = spec.plan_status;
    plan.execution_state.recovery = Some(RecoveryState {
        decision: spec.decision,
        reason: Some(spec.reason),
        retryable: spec.retryable,
        note: spec.note.to_string(),
    });
    true
}

struct BlockedPreconditionRecoverySpec {
    precondition: Precondition,
    clarification_prompt: &'static str,
    plan_status: PlanStatus,
    decision: RecoveryDecision,
    reason: RecoveryReason,
    retryable: bool,
    note: &'static str,
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
