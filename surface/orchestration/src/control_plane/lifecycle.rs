// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{
    ClosureState, ControlPlaneClosureState, ControlPlaneLifecycleState, CoreContractCall,
    OrchestrationFallbackAction, OrchestrationPlan, PlanStatus, RecoveryDecision, RecoveryState,
    RequestClassification, RequestClass, RequestKind, ResourceKind, TypedOrchestrationRequest,
    WorkflowStage, WorkflowStageState, WorkflowStageStatus, WorkflowTemplate,
};

const CONTROL_PLANE_OWNER: &str = "surface_orchestration_control_plane";

pub fn workflow_owner() -> &'static str {
    CONTROL_PLANE_OWNER
}

pub fn select_workflow_template(
    request: &TypedOrchestrationRequest,
    classification: &RequestClassification,
    plan_status: PlanStatus,
    needs_clarification: bool,
    recovery: Option<&RecoveryState>,
) -> WorkflowTemplate {
    if needs_clarification {
        return WorkflowTemplate::ClarifyThenCoordinate;
    }
    if matches!(
        plan_status,
        PlanStatus::Failed | PlanStatus::Blocked | PlanStatus::ClarificationRequired
    ) || recovery
        .map(|row| {
            row.retryable || !matches!(row.decision, RecoveryDecision::None)
        })
        .unwrap_or(false)
    {
        return WorkflowTemplate::DiagnoseRetryEscalate;
    }
    if classification.request_class == RequestClass::ToolCall
        && (request.request_kind == RequestKind::Comparative
            || request.resource_kind == ResourceKind::Mixed)
    {
        return WorkflowTemplate::ResearchSynthesizeVerify;
    }
    if matches!(
        classification.request_class,
        RequestClass::Mutation | RequestClass::TaskProposal | RequestClass::Assimilation
    ) {
        return WorkflowTemplate::PlanExecuteReview;
    }
    WorkflowTemplate::ResearchSynthesizeVerify
}

pub fn build_lifecycle_state(
    template: WorkflowTemplate,
    plan: &OrchestrationPlan,
    fallback_actions: &[OrchestrationFallbackAction],
) -> ControlPlaneLifecycleState {
    let closure = closure_state(plan);
    let stages = vec![
        WorkflowStageState {
            stage: WorkflowStage::IntakeNormalization,
            status: WorkflowStageStatus::Completed,
            owner: workflow_owner().to_string(),
            note: "request was normalized into typed orchestration semantics".to_string(),
        },
        WorkflowStageState {
            stage: WorkflowStage::DecompositionPlanning,
            status: decomposition_stage_status(plan),
            owner: workflow_owner().to_string(),
            note: if plan.selected_plan.steps.is_empty() {
                "no executable steps emitted from planner candidate graph".to_string()
            } else {
                format!(
                    "selected {} steps across {} total candidates",
                    plan.selected_plan.steps.len(),
                    1 + plan.alternative_plans.len()
                )
            },
        },
        WorkflowStageState {
            stage: WorkflowStage::CoordinationSequencing,
            status: sequencing_stage_status(plan.execution_state.plan_status.clone()),
            owner: workflow_owner().to_string(),
            note: format!(
                "plan_status={:?}, selected_variant={:?}",
                plan.execution_state.plan_status, plan.selected_plan.variant
            )
            .to_lowercase(),
        },
        WorkflowStageState {
            stage: WorkflowStage::RecoveryEscalation,
            status: recovery_stage_status(plan),
            owner: workflow_owner().to_string(),
            note: recovery_note(plan, fallback_actions),
        },
        WorkflowStageState {
            stage: WorkflowStage::ResultPackaging,
            status: WorkflowStageStatus::Completed,
            owner: workflow_owner().to_string(),
            note: "control-plane summary, progress projection, and fallback actions packaged"
                .to_string(),
        },
        WorkflowStageState {
            stage: WorkflowStage::VerificationClosure,
            status: verification_stage_status(&closure),
            owner: workflow_owner().to_string(),
            note: format!(
                "verification={:?}, receipts={:?}, memory_packaging={:?}",
                closure.verification, closure.receipt_correlation, closure.memory_packaging
            )
            .to_lowercase(),
        },
    ];

    ControlPlaneLifecycleState {
        owner: workflow_owner().to_string(),
        template,
        active_stage: active_stage(plan, &closure),
        stages,
        next_actions: lifecycle_next_actions(plan, fallback_actions, &closure),
        closure,
    }
}

fn active_stage(plan: &OrchestrationPlan, closure: &ControlPlaneClosureState) -> WorkflowStage {
    if plan.needs_clarification {
        return WorkflowStage::RecoveryEscalation;
    }
    if matches!(
        plan.execution_state.plan_status,
        PlanStatus::Ready | PlanStatus::Planned | PlanStatus::Running | PlanStatus::Degraded
    ) {
        return WorkflowStage::CoordinationSequencing;
    }
    if matches!(plan.execution_state.plan_status, PlanStatus::Failed | PlanStatus::Blocked) {
        return WorkflowStage::RecoveryEscalation;
    }
    if closure.verification != ClosureState::Complete
        || closure.receipt_correlation != ClosureState::Complete
    {
        return WorkflowStage::VerificationClosure;
    }
    WorkflowStage::ResultPackaging
}

fn decomposition_stage_status(plan: &OrchestrationPlan) -> WorkflowStageStatus {
    if plan.needs_clarification {
        return WorkflowStageStatus::Completed;
    }
    if plan.selected_plan.steps.is_empty() {
        return WorkflowStageStatus::Blocked;
    }
    WorkflowStageStatus::Completed
}

fn sequencing_stage_status(plan_status: PlanStatus) -> WorkflowStageStatus {
    match plan_status {
        PlanStatus::Planned | PlanStatus::Ready => WorkflowStageStatus::Ready,
        PlanStatus::Running => WorkflowStageStatus::Running,
        PlanStatus::Completed | PlanStatus::Degraded => WorkflowStageStatus::Completed,
        PlanStatus::ClarificationRequired | PlanStatus::Blocked | PlanStatus::Failed => {
            WorkflowStageStatus::Blocked
        }
    }
}

fn recovery_stage_status(plan: &OrchestrationPlan) -> WorkflowStageStatus {
    if let Some(recovery) = &plan.execution_state.recovery {
        if matches!(recovery.decision, RecoveryDecision::None) {
            return WorkflowStageStatus::Skipped;
        }
        if matches!(plan.execution_state.plan_status, PlanStatus::Running) {
            return WorkflowStageStatus::Running;
        }
        return WorkflowStageStatus::Completed;
    }
    if plan.needs_clarification
        || matches!(
            plan.execution_state.plan_status,
            PlanStatus::Blocked | PlanStatus::ClarificationRequired | PlanStatus::Failed
        )
    {
        return WorkflowStageStatus::Ready;
    }
    WorkflowStageStatus::Skipped
}

fn recovery_note(
    plan: &OrchestrationPlan,
    fallback_actions: &[OrchestrationFallbackAction],
) -> String {
    if let Some(recovery) = &plan.execution_state.recovery {
        return format!(
            "decision={:?};reason={:?};retryable={};fallback_count={}",
            recovery.decision,
            recovery.reason,
            recovery.retryable,
            fallback_actions.len()
        )
        .to_lowercase();
    }
    if plan.needs_clarification {
        return "clarification requested before execution".to_string();
    }
    "no recovery intervention required".to_string()
}

fn verification_stage_status(closure: &ControlPlaneClosureState) -> WorkflowStageStatus {
    if closure.verification == ClosureState::Blocked
        || closure.receipt_correlation == ClosureState::Blocked
        || closure.memory_packaging == ClosureState::Blocked
    {
        return WorkflowStageStatus::Blocked;
    }
    if closure.verification == ClosureState::Complete
        && closure.receipt_correlation == ClosureState::Complete
        && closure.memory_packaging == ClosureState::Complete
    {
        return WorkflowStageStatus::Completed;
    }
    if closure.verification == ClosureState::Ready
        || closure.receipt_correlation == ClosureState::Ready
        || closure.memory_packaging == ClosureState::Ready
    {
        return WorkflowStageStatus::Ready;
    }
    WorkflowStageStatus::Pending
}

fn closure_state(plan: &OrchestrationPlan) -> ControlPlaneClosureState {
    let verification = match plan.execution_state.plan_status {
        PlanStatus::Completed => ClosureState::Complete,
        PlanStatus::Ready | PlanStatus::Planned | PlanStatus::Running | PlanStatus::Degraded => {
            ClosureState::Ready
        }
        PlanStatus::ClarificationRequired => ClosureState::Pending,
        PlanStatus::Blocked | PlanStatus::Failed => ClosureState::Blocked,
    };

    let expected_receipts = plan.execution_state.correlation.expected_core_contract_ids.len();
    let observed_receipts = plan.execution_state.correlation.observed_core_receipt_ids.len()
        + plan.execution_state.correlation.observed_core_outcome_refs.len();
    let receipt_correlation = if expected_receipts == 0 {
        ClosureState::Ready
    } else if observed_receipts > 0 {
        ClosureState::Complete
    } else if matches!(
        plan.execution_state.plan_status,
        PlanStatus::Ready | PlanStatus::Planned | PlanStatus::Running
    ) {
        ClosureState::Pending
    } else if matches!(plan.execution_state.plan_status, PlanStatus::Degraded) {
        ClosureState::Ready
    } else {
        ClosureState::Blocked
    };

    let uses_memory_packaging = plan.selected_plan.steps.iter().any(|step| {
        matches!(
            step.target_contract,
            CoreContractCall::ContextAtomAppend
                | CoreContractCall::ContextTopologyMaterialize
                | CoreContractCall::ContextTopologyInspect
                | CoreContractCall::UnifiedMemoryRead
        )
    });
    let memory_packaging = if !uses_memory_packaging {
        ClosureState::Ready
    } else if matches!(
        plan.execution_state.plan_status,
        PlanStatus::Completed | PlanStatus::Running | PlanStatus::Degraded
    ) {
        ClosureState::Complete
    } else if plan.needs_clarification {
        ClosureState::Pending
    } else {
        ClosureState::Blocked
    };

    ControlPlaneClosureState {
        verification,
        receipt_correlation,
        memory_packaging,
    }
}

fn lifecycle_next_actions(
    plan: &OrchestrationPlan,
    fallback_actions: &[OrchestrationFallbackAction],
    closure: &ControlPlaneClosureState,
) -> Vec<String> {
    let mut actions = Vec::<String>::new();
    if plan.needs_clarification {
        if let Some(prompt) = &plan.clarification_prompt {
            actions.push(format!("request_clarification:{prompt}").to_lowercase());
        } else {
            actions.push("request_clarification".to_string());
        }
    }
    if let Some(recovery) = &plan.execution_state.recovery {
        if recovery.retryable {
            if let Some(first_fallback) = fallback_actions.first() {
                actions.push(
                    format!("retry_via_fallback:{}", first_fallback.kind).to_lowercase(),
                );
            } else {
                actions.push("retry_selected_plan".to_string());
            }
        } else if !matches!(recovery.decision, RecoveryDecision::None) {
            actions.push("escalate_to_kernel_authority".to_string());
        }
    }
    if closure.receipt_correlation != ClosureState::Complete
        && !plan
            .execution_state
            .correlation
            .expected_core_contract_ids
            .is_empty()
    {
        actions.push("await_or_request_core_receipt_correlation".to_string());
    }
    if closure.verification == ClosureState::Ready && !plan.needs_clarification {
        actions.push("run_verification_pass_before_final_emit".to_string());
    }
    if actions.is_empty() {
        actions.push("no_additional_control_plane_action".to_string());
    }
    actions
}
