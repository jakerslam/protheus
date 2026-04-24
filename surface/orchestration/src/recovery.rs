// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{
    ClarificationReason, DegradationReason, ExecutionPosture, OrchestrationPlan, PlanStatus,
    Precondition, RecoveryDecision, RecoveryReason, RecoveryState, StepStatus,
    TypedOrchestrationRequest,
};

pub fn coordinate_recovery_escalation(
    request: &TypedOrchestrationRequest,
    mut plan: OrchestrationPlan,
) -> (OrchestrationPlan, bool) {
    const DEFAULT_MAX_TOOL_FAILURE_PER_TURN: usize = 3;

    fn read_tool_failure_budget_limit(request: &TypedOrchestrationRequest) -> usize {
        fn parse_limit(value: Option<&serde_json::Value>) -> Option<usize> {
            value
                .and_then(|row| row.as_u64())
                .and_then(|row| usize::try_from(row).ok())
                .filter(|row| *row > 0)
        }
        parse_limit(request.payload.get("max_tool_failure_per_turn"))
            .or_else(|| {
                request
                    .payload
                    .get("runtime")
                    .and_then(|row| row.get("max_tool_failure_per_turn"))
                    .and_then(|row| row.as_u64())
                    .and_then(|row| usize::try_from(row).ok())
                    .filter(|row| *row > 0)
            })
            .or_else(|| {
                request
                    .payload
                    .get("orchestration")
                    .and_then(|row| row.get("max_tool_failure_per_turn"))
                    .and_then(|row| row.as_u64())
                    .and_then(|row| usize::try_from(row).ok())
                    .filter(|row| *row > 0)
            })
            .unwrap_or(DEFAULT_MAX_TOOL_FAILURE_PER_TURN)
    }

    fn failed_step_count(plan: &OrchestrationPlan) -> usize {
        plan.execution_state
            .steps
            .iter()
            .filter(|row| row.status == StepStatus::Failed)
            .count()
    }

    fn apply_tool_failure_budget_recovery(
        request: &TypedOrchestrationRequest,
        plan: &mut OrchestrationPlan,
    ) -> bool {
        let failed_steps = failed_step_count(plan);
        if failed_steps == 0 {
            return false;
        }
        let limit = read_tool_failure_budget_limit(request);
        if failed_steps < limit {
            return false;
        }
        let reason_marker = format!("recovery:tool_failure_budget_exceeded:{failed_steps}:{limit}");
        if !plan.classification.reasons.contains(&reason_marker) {
            plan.classification.reasons.push(reason_marker);
        }
        plan.posture = ExecutionPosture::Ask;
        plan.needs_clarification = true;
        plan.classification.needs_clarification = true;
        plan.clarification_prompt = Some(
            "tool failure budget was exceeded for this turn; narrow scope or adjust route before retry"
                .to_string(),
        );
        plan.execution_state.plan_status = PlanStatus::ClarificationRequired;
        plan.execution_state.recovery = Some(RecoveryState {
            decision: RecoveryDecision::Clarify,
            reason: Some(RecoveryReason::ToolFailureBudgetExceeded),
            retryable: true,
            note: format!(
                "tool failure budget reached ({failed_steps}/{limit}); switched to clarification-first recovery"
            ),
        });
        true
    }

    if matches!(
        plan.execution_state.plan_status,
        PlanStatus::Running | PlanStatus::Completed
    ) {
        return (plan, false);
    }

    if apply_tool_failure_budget_recovery(request, &mut plan) {
        return (plan, true);
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
        let typed_probe_gap = plan
            .classification
            .reasons
            .iter()
            .any(|reason| reason.starts_with("typed_probe_contract_missing"));
        plan.clarification_prompt = Some(if typed_probe_gap {
            "typed tool routing contract is incomplete; refresh probe envelope and retry with explicit route"
                .to_string()
        } else {
            "no executable plan steps were generated; provide narrower scope or an explicit tool/workspace target"
                .to_string()
        });
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
        plan.execution_state.plan_status = PlanStatus::ClarificationRequired;
        plan.execution_state.recovery = Some(RecoveryState {
            decision: RecoveryDecision::Clarify,
            reason: Some(if typed_probe_gap {
                RecoveryReason::TransportFailure
            } else {
                RecoveryReason::PlannerContradiction
            }),
            retryable: true,
            note: if typed_probe_gap {
                "planner emitted no executable steps because typed probe contract is incomplete"
                    .to_string()
            } else {
                "planner emitted no executable steps".to_string()
            },
        });
        return (plan, true);
    }

    if matches!(plan.execution_state.plan_status, PlanStatus::Failed) {
        plan.posture = ExecutionPosture::Ask;
        plan.needs_clarification = true;
        plan.clarification_prompt = Some(
            "core execution failed; retry with a narrower route or provide direct workspace/web evidence"
                .to_string(),
        );
        plan.classification.needs_clarification = true;
        plan.classification
            .reasons
            .push("recovery:failed_execution_requires_clarification".to_string());
        plan.execution_state.plan_status = PlanStatus::ClarificationRequired;
        plan.execution_state.recovery = Some(RecoveryState {
            decision: RecoveryDecision::Clarify,
            reason: Some(RecoveryReason::TransportFailure),
            retryable: true,
            note: "core execution failed; converted to clarification-first recovery to prevent repetitive fallback loops".to_string(),
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
