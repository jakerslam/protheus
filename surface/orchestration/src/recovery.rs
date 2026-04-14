// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{
    ClarificationReason, DegradationReason, ExecutionPosture, OrchestrationPlan, PlanStatus,
    Precondition, RecoveryDecision, RecoveryReason, RecoveryState, StepStatus,
    TypedOrchestrationRequest,
};

pub fn apply_recovery_policy(
    _request: &TypedOrchestrationRequest,
    mut plan: OrchestrationPlan,
) -> (OrchestrationPlan, bool) {
    if plan.execution_state.plan_status.eq(&PlanStatus::Running)
        || plan.execution_state.plan_status.eq(&PlanStatus::Completed)
        || plan.execution_state.plan_status.eq(&PlanStatus::Failed)
    {
        return (plan, false);
    }

    if plan
        .selected_plan
        .blocked_on
        .contains(&Precondition::TargetSupplied)
    {
        plan.posture = ExecutionPosture::Ask;
        plan.needs_clarification = true;
        plan.clarification_prompt = Some("specify a target before execution".to_string());
        plan.execution_state.plan_status = PlanStatus::ClarificationRequired;
        plan.execution_state.recovery = Some(RecoveryState {
            decision: RecoveryDecision::Clarify,
            reason: Some(RecoveryReason::MissingTarget),
            retryable: true,
            note: "planner blocked on missing target".to_string(),
        });
        return (plan, true);
    }

    if plan
        .selected_plan
        .blocked_on
        .contains(&Precondition::TargetSyntacticallyValid)
    {
        plan.posture = ExecutionPosture::Ask;
        plan.needs_clarification = true;
        plan.clarification_prompt =
            Some("specify a valid target format before execution".to_string());
        plan.execution_state.plan_status = PlanStatus::ClarificationRequired;
        plan.execution_state.recovery = Some(RecoveryState {
            decision: RecoveryDecision::Clarify,
            reason: Some(RecoveryReason::TargetInvalid),
            retryable: true,
            note: "planner blocked on syntactically invalid target".to_string(),
        });
        return (plan, true);
    }

    if plan
        .selected_plan
        .blocked_on
        .contains(&Precondition::TargetExists)
    {
        plan.posture = ExecutionPosture::Ask;
        plan.needs_clarification = true;
        plan.clarification_prompt = Some("target could not be located for execution".to_string());
        plan.execution_state.plan_status = PlanStatus::ClarificationRequired;
        plan.execution_state.recovery = Some(RecoveryState {
            decision: RecoveryDecision::Clarify,
            reason: Some(RecoveryReason::TargetNotFound),
            retryable: true,
            note: "planner blocked because the requested target was not found".to_string(),
        });
        return (plan, true);
    }

    if plan
        .selected_plan
        .blocked_on
        .contains(&Precondition::AuthorizationValid)
    {
        plan.posture = ExecutionPosture::Ask;
        plan.needs_clarification = true;
        plan.clarification_prompt = Some("authorization is required before execution".to_string());
        plan.execution_state.plan_status = PlanStatus::Blocked;
        plan.execution_state.recovery = Some(RecoveryState {
            decision: RecoveryDecision::Clarify,
            reason: Some(RecoveryReason::AuthorizationFailure),
            retryable: true,
            note: "planner blocked on authorization".to_string(),
        });
        return (plan, true);
    }

    if plan
        .selected_plan
        .blocked_on
        .contains(&Precondition::PolicyAllows)
    {
        plan.posture = ExecutionPosture::Ask;
        plan.needs_clarification = true;
        plan.clarification_prompt =
            Some("policy scope must be narrowed before execution".to_string());
        plan.execution_state.plan_status = PlanStatus::Blocked;
        plan.execution_state.recovery = Some(RecoveryState {
            decision: RecoveryDecision::Halt,
            reason: Some(RecoveryReason::PolicyDenied),
            retryable: false,
            note: "planner blocked on policy".to_string(),
        });
        return (plan, true);
    }

    if plan
        .selected_plan
        .blocked_on
        .contains(&Precondition::TransportAvailable)
    {
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
        let reason = match plan.selected_plan.degradation.first() {
            Some(DegradationReason::ToolUnavailable) => RecoveryReason::ToolUnavailable,
            Some(DegradationReason::TransportFailure) => RecoveryReason::TransportFailure,
            Some(DegradationReason::MissingTarget) => RecoveryReason::MissingTarget,
            Some(DegradationReason::TargetInvalid) => RecoveryReason::TargetInvalid,
            Some(DegradationReason::TargetNotFound) => RecoveryReason::TargetNotFound,
            Some(DegradationReason::AuthFailure) => RecoveryReason::AuthorizationFailure,
            Some(DegradationReason::PolicyDenied) => RecoveryReason::PolicyDenied,
            None => RecoveryReason::PlannerContradiction,
        };
        plan.execution_state.plan_status = PlanStatus::Degraded;
        plan.execution_state.recovery = Some(RecoveryState {
            decision: RecoveryDecision::Degrade,
            reason: Some(reason.clone()),
            retryable: matches!(
                reason,
                RecoveryReason::ToolUnavailable | RecoveryReason::TransportFailure
            ),
            note: "planner selected degraded alternate path".to_string(),
        });
        return (plan, true);
    }

    (plan, false)
}
