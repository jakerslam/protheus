// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{
    ClosureState, ControlPlaneClosureState, ControlPlaneHandoff, ControlPlaneLifecycleState,
    CoreContractCall, OrchestrationFallbackAction, OrchestrationPlan, PlanStatus, RecoveryDecision,
    RecoveryState, RequestClass, RequestClassification, RequestKind, ResourceKind,
    TypedOrchestrationRequest, WorkflowStage, WorkflowStageState, WorkflowStageStatus,
    WorkflowTemplate,
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
        .map(|row| row.retryable || !matches!(row.decision, RecoveryDecision::None))
        .unwrap_or(false)
    {
        return WorkflowTemplate::DiagnoseRetryEscalate;
    }
    if classification.request_class == RequestClass::ToolCall
        && (matches!(
            request.request_kind,
            RequestKind::Comparative | RequestKind::Workflow
        ) || matches!(
            request.resource_kind,
            ResourceKind::Mixed | ResourceKind::Workspace | ResourceKind::Tooling
        ))
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
    let handoff_chain = build_handoff_chain(plan, &closure);
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
        handoff_chain,
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
    if matches!(
        plan.execution_state.plan_status,
        PlanStatus::Failed | PlanStatus::Blocked
    ) {
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

    let expected_receipts = plan
        .execution_state
        .correlation
        .expected_core_contract_ids
        .len();
    let observed_receipts = plan
        .execution_state
        .correlation
        .observed_core_receipt_ids
        .len()
        + plan
            .execution_state
            .correlation
            .observed_core_outcome_refs
            .len();
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
                actions.push(format!("retry_via_fallback:{}", first_fallback.kind).to_lowercase());
            } else {
                actions.push("retry_selected_plan".to_string());
            }
        } else if !matches!(recovery.decision, RecoveryDecision::None) {
            actions.push("escalate_to_kernel_authority".to_string());
        }
    }
    if matches!(
        plan.request_classification.request_class,
        RequestClass::ToolCall
    ) {
        actions.push("emit_tool_route_recommendation_envelope".to_string());
    }
    if matches!(
        plan.request_classification.resource_kind,
        ResourceKind::Workspace | ResourceKind::Tooling | ResourceKind::Mixed
    ) {
        actions.push("prefer_workspace_first_probe_contract_for_tool_routing".to_string());
    }
    if matches!(
        plan.request_classification.resource_kind,
        ResourceKind::Tooling
    ) {
        actions.push("prefer_tree_sitter_query_route_for_code_workspace_intent".to_string());
    }
    if matches!(
        plan.request_classification.request_kind,
        RequestKind::Comparative | RequestKind::Workflow
    ) {
        actions.push("emit_synthesis_profile_and_context_mentions_projection".to_string());
    }
    if matches!(
        plan.request_classification.request_kind,
        RequestKind::Comparative
    ) && matches!(
        plan.request_classification.resource_kind,
        ResourceKind::Tooling | ResourceKind::Mixed
    ) {
        actions.push("require_multi_language_tooling_synthesis_evidence".to_string());
        actions.push("require_multi_provider_tooling_synthesis_evidence".to_string());
        actions.push("require_provider_compatibility_surface_crosscheck".to_string());
        actions.push("require_multi_provider_family_synthesis_evidence".to_string());
        actions.push("require_context_surface_alignment_for_tooling_synthesis".to_string());
        actions.push("require_shared_contract_schema_crosscheck".to_string());
        actions.push("require_shared_cline_schema_crosscheck".to_string());
        actions.push("require_shared_message_schema_crosscheck".to_string());
        actions.push("require_shared_service_schema_crosscheck".to_string());
        actions.push("require_chat_hook_schema_crosscheck".to_string());
        actions.push("require_chat_util_schema_crosscheck".to_string());
        actions.push("require_chat_message_schema_crosscheck".to_string());
        actions.push("require_chat_shared_schema_crosscheck".to_string());
        actions.push("require_chat_type_schema_crosscheck".to_string());
        actions.push("require_chat_view_schema_crosscheck".to_string());
        actions.push("require_chat_layout_schema_crosscheck".to_string());
        actions.push("require_chat_root_schema_crosscheck".to_string());
        actions.push("require_chat_output_schema_crosscheck".to_string());
        actions.push("require_chat_error_schema_crosscheck".to_string());
        actions.push("require_chat_preview_schema_crosscheck".to_string());
        actions.push("require_chat_interaction_schema_crosscheck".to_string());
        actions.push("require_chat_component_schema_crosscheck".to_string());
        actions.push("require_chat_task_header_schema_crosscheck".to_string());
        actions.push("require_chat_task_header_button_schema_crosscheck".to_string());
        actions.push("require_chat_auto_approve_schema_crosscheck".to_string());
        actions.push("require_cline_rules_schema_crosscheck".to_string());
        actions.push("require_common_component_schema_crosscheck".to_string());
        actions.push("require_common_content_schema_crosscheck".to_string());
        actions.push("require_common_ui_schema_crosscheck".to_string());
        actions.push("require_history_schema_crosscheck".to_string());
        actions.push("require_menu_schema_crosscheck".to_string());
        actions.push("require_onboarding_schema_crosscheck".to_string());
        actions.push("require_browser_schema_crosscheck".to_string());
        actions.push("require_settings_utils_schema_crosscheck".to_string());
        actions.push("require_hooks_schema_crosscheck".to_string());
        actions.push("require_root_provider_schema_crosscheck".to_string());
        actions.push("require_shell_config_schema_crosscheck".to_string());
        actions.push("require_shell_hook_schema_crosscheck".to_string());
        actions.push("require_shell_service_schema_crosscheck".to_string());
        actions.push("require_service_temp_schema_crosscheck".to_string());
        actions.push("require_service_test_schema_crosscheck".to_string());
        actions.push("require_service_uri_schema_crosscheck".to_string());
        actions.push("require_shell_lib_schema_crosscheck".to_string());
        actions.push("require_shell_util_schema_crosscheck".to_string());
        actions.push("require_settings_component_schema_crosscheck".to_string());
        actions.push("require_settings_section_schema_crosscheck".to_string());
        actions.push("require_settings_model_picker_schema_crosscheck".to_string());
        actions.push("require_settings_common_schema_crosscheck".to_string());
        actions.push("require_settings_test_schema_crosscheck".to_string());
        actions.push("require_settings_control_surface_crosscheck".to_string());
    }
    if matches!(
        plan.request_classification.request_class,
        RequestClass::ReadOnly
    ) {
        actions.push("prefer_workspace_context_synthesis_projection".to_string());
        actions.push("emit_shell_context_projection".to_string());
        actions.push("emit_chat_hook_projection".to_string());
        actions.push("emit_chat_util_projection".to_string());
        actions.push("emit_chat_message_projection".to_string());
        actions.push("emit_chat_shared_projection".to_string());
        actions.push("emit_chat_type_projection".to_string());
        actions.push("emit_chat_view_projection".to_string());
        actions.push("emit_chat_layout_projection".to_string());
        actions.push("emit_chat_root_projection".to_string());
        actions.push("emit_chat_output_projection".to_string());
        actions.push("emit_chat_error_projection".to_string());
        actions.push("emit_chat_preview_projection".to_string());
        actions.push("emit_chat_interaction_projection".to_string());
        actions.push("emit_chat_component_projection".to_string());
        actions.push("emit_shell_config_projection".to_string());
        actions.push("emit_shell_hook_projection".to_string());
        actions.push("emit_shell_service_projection".to_string());
        actions.push("emit_service_temp_projection".to_string());
        actions.push("emit_service_test_projection".to_string());
        actions.push("emit_service_uri_projection".to_string());
        actions.push("emit_shell_lib_projection".to_string());
        actions.push("emit_shell_util_projection".to_string());
        actions.push("emit_chat_task_header_projection".to_string());
        actions.push("emit_chat_task_header_button_projection".to_string());
        actions.push("emit_chat_auto_approve_projection".to_string());
        actions.push("emit_cline_rules_projection".to_string());
        actions.push("emit_common_component_projection".to_string());
        actions.push("emit_common_content_projection".to_string());
        actions.push("emit_common_ui_projection".to_string());
        actions.push("emit_history_projection".to_string());
        actions.push("emit_menu_projection".to_string());
        actions.push("emit_onboarding_projection".to_string());
        actions.push("emit_browser_projection".to_string());
        actions.push("emit_settings_utils_projection".to_string());
        actions.push("emit_hooks_projection".to_string());
        actions.push("emit_root_provider_projection".to_string());
        actions.push("emit_settings_section_projection".to_string());
        actions.push("emit_settings_test_projection".to_string());
    }
    if matches!(
        plan.request_classification.resource_kind,
        ResourceKind::Tooling
    ) && matches!(
        plan.request_classification.request_class,
        RequestClass::ReadOnly | RequestClass::ToolCall
    ) {
        actions.push("emit_mcp_render_surface_projection".to_string());
    }
    if matches!(
        plan.request_classification.resource_kind,
        ResourceKind::Tooling
    ) && matches!(
        plan.request_classification.request_class,
        RequestClass::Mutation | RequestClass::TaskProposal
    ) {
        actions.push("emit_mcp_configuration_delta_projection".to_string());
    }
    if matches!(
        plan.request_classification.resource_kind,
        ResourceKind::Tooling
    ) && matches!(
        plan.request_classification.request_class,
        RequestClass::ToolCall | RequestClass::ReadOnly
    ) {
        actions.push("prefer_mcp_marketplace_surface_synthesis".to_string());
        actions.push("emit_provider_catalog_synthesis_projection".to_string());
        actions.push("emit_provider_selection_matrix_projection".to_string());
        actions.push("emit_provider_model_picker_projection".to_string());
        actions.push("emit_provider_family_projection".to_string());
        actions.push("emit_provider_utils_projection".to_string());
        actions.push("emit_shared_contract_projection".to_string());
        actions.push("emit_shared_cline_projection".to_string());
        actions.push("emit_shared_messages_projection".to_string());
        actions.push("emit_shared_internal_projection".to_string());
        actions.push("emit_shared_services_projection".to_string());
        actions.push("emit_shared_multi_root_projection".to_string());
        actions.push("emit_chat_message_projection".to_string());
        actions.push("emit_chat_shared_projection".to_string());
        actions.push("emit_chat_type_projection".to_string());
        actions.push("emit_chat_view_projection".to_string());
        actions.push("emit_chat_layout_projection".to_string());
        actions.push("emit_chat_root_projection".to_string());
        actions.push("emit_chat_output_projection".to_string());
        actions.push("emit_chat_error_projection".to_string());
        actions.push("emit_chat_preview_projection".to_string());
        actions.push("emit_chat_interaction_projection".to_string());
        actions.push("emit_chat_component_projection".to_string());
        actions.push("emit_chat_task_header_projection".to_string());
        actions.push("emit_chat_task_header_button_projection".to_string());
        actions.push("emit_chat_auto_approve_projection".to_string());
        actions.push("emit_cline_rules_projection".to_string());
        actions.push("emit_common_component_projection".to_string());
        actions.push("emit_common_content_projection".to_string());
        actions.push("emit_common_ui_projection".to_string());
        actions.push("emit_history_projection".to_string());
        actions.push("emit_menu_projection".to_string());
        actions.push("emit_onboarding_projection".to_string());
        actions.push("emit_browser_projection".to_string());
        actions.push("emit_settings_utils_projection".to_string());
        actions.push("emit_hooks_projection".to_string());
        actions.push("emit_root_provider_projection".to_string());
        actions.push("emit_settings_component_projection".to_string());
        actions.push("emit_settings_section_projection".to_string());
        actions.push("emit_settings_model_picker_projection".to_string());
        actions.push("emit_settings_common_projection".to_string());
        actions.push("emit_settings_control_projection".to_string());
    }
    if matches!(
        plan.request_classification.resource_kind,
        ResourceKind::Tooling
    ) && matches!(
        plan.request_classification.request_class,
        RequestClass::Mutation | RequestClass::TaskProposal
    ) {
        actions.push("require_mcp_server_row_delta_validation_before_emit".to_string());
        actions.push("require_provider_config_delta_validation_before_emit".to_string());
        actions.push("require_provider_credentials_shape_validation_before_emit".to_string());
    }
    if matches!(
        plan.request_classification.resource_kind,
        ResourceKind::Tooling | ResourceKind::Mixed
    ) && matches!(
        plan.request_classification.request_kind,
        RequestKind::Direct
    ) {
        actions.push("prefer_proto_conversion_consistency_for_tooling_payloads".to_string());
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
    if matches!(
        plan.execution_state.plan_status,
        PlanStatus::Ready | PlanStatus::Planned
    ) && plan.selected_plan.steps.is_empty()
        && !plan.needs_clarification
    {
        actions.push("emit_tool_route_recommendation_envelope".to_string());
        actions.push("refresh_workspace_tooling_probe_and_rebuild_plan".to_string());
    }
    if matches!(plan.execution_state.plan_status, PlanStatus::Degraded)
        && fallback_actions.is_empty()
    {
        actions.push("emit_synthesis_gap_summary_for_shell".to_string());
    }
    if matches!(
        plan.execution_state.plan_status,
        PlanStatus::Blocked | PlanStatus::Failed
    ) && !plan.needs_clarification
    {
        actions.push("emit_workflow_gate_diagnostic_with_single_retry_path".to_string());
    }
    if closure.memory_packaging != ClosureState::Complete
        && plan
            .selected_plan
            .steps
            .iter()
            .any(|step| matches!(step.target_contract, CoreContractCall::ContextAtomAppend))
    {
        actions.push("complete_memory_packaging_handoff".to_string());
    }
    actions.sort();
    actions.dedup();
    if actions.is_empty() {
        actions.push("no_additional_control_plane_action".to_string());
    }
    actions
}

fn build_handoff_chain(
    plan: &OrchestrationPlan,
    closure: &ControlPlaneClosureState,
) -> Vec<ControlPlaneHandoff> {
    let sequencing_status = sequencing_stage_status(plan.execution_state.plan_status.clone());
    let verification_status = verification_stage_status(closure);
    let memory_status = match closure.memory_packaging {
        ClosureState::Complete => WorkflowStageStatus::Completed,
        ClosureState::Ready => WorkflowStageStatus::Ready,
        ClosureState::Pending => WorkflowStageStatus::Pending,
        ClosureState::Blocked => WorkflowStageStatus::Blocked,
    };
    vec![
        ControlPlaneHandoff {
            handoff_id: "handoff_user_request_to_decomposition".to_string(),
            from: "user_request_ingress".to_string(),
            to: "decomposition_planning".to_string(),
            owner: workflow_owner().to_string(),
            artifact: "typed_request_snapshot".to_string(),
            status: WorkflowStageStatus::Completed,
        },
        ControlPlaneHandoff {
            handoff_id: "handoff_decomposition_to_coordination".to_string(),
            from: "decomposition_planning".to_string(),
            to: "coordination_sequencing".to_string(),
            owner: workflow_owner().to_string(),
            artifact: "selected_plan_recommendation".to_string(),
            status: decomposition_stage_status(plan),
        },
        ControlPlaneHandoff {
            handoff_id: "handoff_coordination_to_core_execution".to_string(),
            from: "coordination_sequencing".to_string(),
            to: "core_contract_execution".to_string(),
            owner: workflow_owner().to_string(),
            artifact: "core_contract_call_envelope".to_string(),
            status: sequencing_status,
        },
        ControlPlaneHandoff {
            handoff_id: "handoff_core_execution_to_verification".to_string(),
            from: "core_contract_execution".to_string(),
            to: "verification_closure".to_string(),
            owner: workflow_owner().to_string(),
            artifact: "execution_observation_snapshot".to_string(),
            status: verification_status,
        },
        ControlPlaneHandoff {
            handoff_id: "handoff_verification_to_memory_packaging".to_string(),
            from: "verification_closure".to_string(),
            to: "memory_packaging_projection".to_string(),
            owner: workflow_owner().to_string(),
            artifact: "result_package_projection".to_string(),
            status: memory_status,
        },
    ]
}
