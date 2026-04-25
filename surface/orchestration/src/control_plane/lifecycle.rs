// Layer ownership: surface/orchestration (non-canonical orchestration coordination only).
use crate::contracts::{
    Capability, ClosureState, ControlPlaneClosureState, ControlPlaneHandoff,
    ControlPlaneLifecycleState, CoreContractCall, OrchestrationFallbackAction, OrchestrationPlan,
    PlanStatus, RecoveryDecision, RecoveryReason, RecoveryState, RequestClass,
    RequestClassification, RequestKind, ResourceKind, TypedOrchestrationRequest, WorkflowStage,
    WorkflowStageState, WorkflowStageStatus, WorkflowTemplate,
};
use crate::control_plane::templates::{workflow_template_definition, WorkflowSubtemplate};

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
    if matches!(classification.request_class, RequestClass::Assimilation) {
        if is_forgecode_assimilation_request(request) {
            if is_forgecode_raw_capability_request(request) {
                return WorkflowTemplate::ForgeCodeRawCapabilityAssimilation;
            }
            return WorkflowTemplate::ForgeCodeAgentComposition;
        }
        return WorkflowTemplate::CodexToolingSynthesis;
    }
    if matches!(
        classification.request_class,
        RequestClass::Mutation | RequestClass::TaskProposal
    ) {
        return WorkflowTemplate::PlanExecuteReview;
    }
    WorkflowTemplate::ResearchSynthesizeVerify
}

fn is_forgecode_assimilation_request(request: &TypedOrchestrationRequest) -> bool {
    let has_forgecode_marker = |value: &str| {
        let lower = value.to_ascii_lowercase();
        lower.contains("forgecode")
            || lower.contains("forge code")
            || lower.contains("antinomyhq/forgecode")
            || lower.contains("tailcallhq/forgecode")
    };

    if has_forgecode_marker(&request.legacy_intent) {
        return true;
    }
    if request
        .target_refs
        .iter()
        .any(|target| has_forgecode_marker(target))
    {
        return true;
    }
    has_forgecode_marker(&request.payload.to_string())
}

fn is_forgecode_raw_capability_request(request: &TypedOrchestrationRequest) -> bool {
    let has_raw_marker = |value: &str| {
        let lower = value.to_ascii_lowercase();
        lower.contains("raw capability")
            || lower.contains("raw capabilities")
            || lower.contains("mechanics only")
            || lower.contains("mechanics-only")
            || lower.contains("raw system capability")
            || lower.contains("workflow disabled")
            || lower.contains("no workflow wrapper")
    };
    if has_raw_marker(&request.legacy_intent) {
        return true;
    }
    if request
        .target_refs
        .iter()
        .any(|target| has_raw_marker(target))
    {
        return true;
    }
    has_raw_marker(&request.payload.to_string())
}

pub fn build_lifecycle_state(
    template: WorkflowTemplate,
    plan: &OrchestrationPlan,
    fallback_actions: &[OrchestrationFallbackAction],
) -> ControlPlaneLifecycleState {
    let template_definition = workflow_template_definition(&template);
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
            note: format!(
                "template={} ({}) summary, progress projection, and fallback actions packaged",
                template_definition.id, template_definition.description
            ),
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
        next_actions: lifecycle_next_actions(
            plan,
            fallback_actions,
            &closure,
            template_definition.subtemplates,
        ),
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
    subtemplates: &[WorkflowSubtemplate],
) -> Vec<String> {
    let mut actions = Vec::<String>::new();
    let has_forgecode_lane = subtemplates
        .iter()
        .any(|row| row.id.starts_with("forgecode_"));
    let required_capabilities = &plan.classification.required_capabilities;
    let has_workspace_capability = required_capabilities.iter().any(|capability| {
        matches!(
            capability,
            Capability::WorkspaceRead | Capability::WorkspaceSearch
        )
    });
    let has_web_capability = required_capabilities
        .iter()
        .any(|capability| matches!(capability, Capability::WebSearch | Capability::WebFetch));
    let has_tool_route = required_capabilities
        .iter()
        .any(|capability| matches!(capability, Capability::ToolRoute | Capability::ExecuteTool));
    let has_tooling_or_mixed_route =
        has_tool_route && (has_workspace_capability || has_web_capability);

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
            if matches!(
                recovery.reason,
                Some(RecoveryReason::ToolFailureBudgetExceeded)
            ) {
                actions.push("require_scope_narrowing_or_budget_override_before_retry".to_string());
            }
        } else if !matches!(recovery.decision, RecoveryDecision::None) {
            actions.push("escalate_to_kernel_authority".to_string());
        }
    }
    if matches!(plan.classification.request_class, RequestClass::ToolCall) {
        actions.push("emit_tool_route_recommendation_envelope".to_string());
    }
    if has_workspace_capability || has_tool_route {
        actions.push("prefer_workspace_first_probe_contract_for_tool_routing".to_string());
    }
    if has_tool_route {
        actions.push("prefer_tree_sitter_query_route_for_code_workspace_intent".to_string());
    }
    if has_forgecode_lane {
        actions.push("require_tool_name_alias_normalization_before_route_probe".to_string());
        actions.push("require_retryable_error_backoff_contract_for_tool_calls".to_string());
        actions.push("require_mcp_transport_fallback_http_then_sse".to_string());
        actions.push("require_non_interactive_mcp_auth_reuse_before_prompt".to_string());
        actions.push("require_semantic_discovery_before_exact_symbol_search".to_string());
        actions.push("require_exact_pattern_search_for_precise_symbol_match".to_string());
        actions.push("require_known_path_direct_read_for_targeted_context".to_string());
        actions.push("require_parallel_tool_calls_for_independent_queries".to_string());
        actions.push("require_tool_grounded_verification_before_operator_emit".to_string());
        actions.push("require_step_checkpoint_state_update_after_each_completed_step".to_string());
        actions.push("require_completion_claim_only_after_execution_and_validation".to_string());
        actions.push("require_specialized_tool_selection_over_shell_for_file_ops".to_string());
        actions.push("require_shell_usage_reserved_for_terminal_operations".to_string());
        actions.push("require_simple_lookup_local_tooling_before_subagent_dispatch".to_string());
        actions.push("require_direct_read_or_search_before_subagent_launch".to_string());
        actions.push("require_subagent_brief_to_declare_work_mode".to_string());
        actions.push("require_subagent_brief_to_declare_output_contract".to_string());
        actions.push("require_subagent_brief_to_declare_context_scope".to_string());
        actions.push("require_subagent_result_synthesis_before_operator_emit".to_string());
        actions.push("require_task_role_selection_before_subagent_dispatch".to_string());
        actions.push("require_parallel_subagent_batch_for_independent_work".to_string());
        actions.push("require_disjoint_write_scope_for_parallel_subagent_workers".to_string());
        actions.push("require_subagent_result_integration_before_operator_emit".to_string());
        if let Some(summary) = forgecode_mcp_retry_diagnostic_summary(plan, fallback_actions) {
            actions.push("require_concise_mcp_retry_fallback_diagnostics_projection".to_string());
            actions.push(
                format!("emit_forgecode_mcp_retry_diagnostic_summary:{summary}").to_lowercase(),
            );
        }
    }
    if matches!(
        plan.classification.request_class,
        RequestClass::ReadOnly | RequestClass::ToolCall
    ) {
        actions.push("emit_synthesis_profile_and_context_mentions_projection".to_string());
    }
    if has_tooling_or_mixed_route
        && matches!(
            plan.classification.request_class,
            RequestClass::ReadOnly | RequestClass::ToolCall
        )
    {
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
    if matches!(plan.classification.request_class, RequestClass::ReadOnly) {
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
    if has_tool_route
        && matches!(
            plan.classification.request_class,
            RequestClass::ReadOnly | RequestClass::ToolCall
        )
    {
        actions.push("emit_mcp_render_surface_projection".to_string());
    }
    if has_tool_route
        && matches!(
            plan.classification.request_class,
            RequestClass::Mutation | RequestClass::TaskProposal
        )
    {
        actions.push("emit_mcp_configuration_delta_projection".to_string());
    }
    if has_tool_route
        && matches!(
            plan.classification.request_class,
            RequestClass::ToolCall | RequestClass::ReadOnly
        )
    {
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
    if has_tool_route
        && matches!(
            plan.classification.request_class,
            RequestClass::Mutation | RequestClass::TaskProposal
        )
    {
        actions.push("require_mcp_server_row_delta_validation_before_emit".to_string());
        actions.push("require_provider_config_delta_validation_before_emit".to_string());
        actions.push("require_provider_credentials_shape_validation_before_emit".to_string());
    }
    if has_tooling_or_mixed_route
        && matches!(
            plan.classification.request_class,
            RequestClass::ReadOnly | RequestClass::ToolCall
        )
    {
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
    let subtemplate_context = WorkflowSubtemplateContext {
        plan,
        fallback_actions,
        closure,
        has_workspace_capability,
        has_web_capability,
        has_tool_route,
    };
    append_workflow_subtemplate_actions(&mut actions, subtemplates, &subtemplate_context);
    actions.sort();
    actions.dedup();
    if actions.is_empty() {
        actions.push("no_additional_control_plane_action".to_string());
    }
    actions
}

fn forgecode_mcp_retry_diagnostic_summary(
    plan: &OrchestrationPlan,
    fallback_actions: &[OrchestrationFallbackAction],
) -> Option<String> {
    let retry_marker_count = plan
        .classification
        .reasons
        .iter()
        .filter(|reason| {
            let lower = reason.to_ascii_lowercase();
            lower.contains("mcp")
                || lower.contains("retry")
                || lower.contains("backoff")
                || lower.contains("transport_fallback")
        })
        .count();
    let transport_fallback_count = fallback_actions
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
        .count();
    let budget_exceeded = matches!(
        plan.execution_state
            .recovery
            .as_ref()
            .and_then(|row| row.reason.as_ref()),
        Some(RecoveryReason::ToolFailureBudgetExceeded)
    ) || plan
        .classification
        .reasons
        .iter()
        .any(|reason| reason.starts_with("recovery:tool_failure_budget_exceeded:"));
    if retry_marker_count == 0 && transport_fallback_count == 0 && !budget_exceeded {
        return None;
    }
    Some(format!(
        "retry_markers={retry_marker_count};transport_fallbacks={transport_fallback_count};fallbacks={};budget_exceeded={budget_exceeded}",
        fallback_actions.len()
    ))
}

struct WorkflowSubtemplateContext<'a> {
    plan: &'a OrchestrationPlan,
    fallback_actions: &'a [OrchestrationFallbackAction],
    closure: &'a ControlPlaneClosureState,
    has_workspace_capability: bool,
    has_web_capability: bool,
    has_tool_route: bool,
}

fn append_workflow_subtemplate_actions(
    actions: &mut Vec<String>,
    subtemplates: &[WorkflowSubtemplate],
    context: &WorkflowSubtemplateContext<'_>,
) {
    for subtemplate in subtemplates {
        if !workflow_subtemplate_active(subtemplate.id, context) {
            continue;
        }
        actions.push(format!("activate_subworkflow:{}", subtemplate.id).to_lowercase());
        for signal in subtemplate.required_signals {
            actions.push(format!("require_subworkflow_signal:{signal}").to_lowercase());
            if workflow_signal_runtime_actionable(signal) {
                actions.push((*signal).to_string());
            }
        }
        for gate in subtemplate.required_gates {
            actions.push(format!("run_subworkflow_gate:{gate}").to_lowercase());
        }
    }
}

fn workflow_subtemplate_active(
    subtemplate_id: &str,
    context: &WorkflowSubtemplateContext<'_>,
) -> bool {
    let plan = context.plan;
    let has_workspace_capability = context.has_workspace_capability;
    let has_web_capability = context.has_web_capability;
    let has_tool_route = context.has_tool_route;
    let has_tooling_or_mixed_route =
        has_tool_route && (has_workspace_capability || has_web_capability);
    let mentions_worktree = plan_reason_contains(plan, &["worktree", ".worktrees", "branch"]);
    let mentions_cleanup = plan_reason_contains(plan, &["clean", "cleanup", "prune", "remove"]);
    let mentions_conflict = plan_reason_contains(
        plan,
        &["conflict", "merge", "rebase", "lock file", "generated file"],
    );
    match subtemplate_id {
        "codex_wave_preflight" => matches!(
            plan.classification.request_class,
            RequestClass::Assimilation
        ),
        "codex_disjoint_wave_execution" => {
            matches!(
                plan.classification.request_class,
                RequestClass::Assimilation
            ) && !plan.selected_plan.steps.is_empty()
        }
        "codex_tool_route_misdirection" => has_workspace_capability || has_tool_route,
        "codex_multi_provider_synthesis_recovery" => has_tooling_or_mixed_route,
        "codex_route_probe_dry_run" => has_tool_route,
        "codex_environment_diagnose_parallel" => {
            !context.fallback_actions.is_empty()
                || plan.execution_state.recovery.is_some()
                || plan.needs_clarification
                || matches!(
                    plan.execution_state.plan_status,
                    PlanStatus::Blocked | PlanStatus::Failed | PlanStatus::Degraded
                )
                || context.closure.verification == ClosureState::Blocked
        }
        "codex_worktree_isolated_setup" => mentions_worktree && !mentions_cleanup,
        "codex_worktree_cleanup_safety_modes" => mentions_worktree && mentions_cleanup,
        "codex_review_fix_loop" => matches!(
            plan.classification.request_class,
            RequestClass::Assimilation | RequestClass::Mutation | RequestClass::TaskProposal
        ),
        "forgecode_sage_research_lane" => {
            matches!(
                plan.classification.request_class,
                RequestClass::Assimilation
            ) && (has_workspace_capability || has_web_capability || has_tool_route)
        }
        "forgecode_muse_planning_lane" => matches!(
            plan.classification.request_class,
            RequestClass::Assimilation
        ),
        "forgecode_forge_implementation_lane" => {
            matches!(
                plan.classification.request_class,
                RequestClass::Assimilation
            ) && !plan.needs_clarification
                && matches!(
                    plan.execution_state.plan_status,
                    PlanStatus::Planned
                        | PlanStatus::Ready
                        | PlanStatus::Running
                        | PlanStatus::Degraded
                        | PlanStatus::Completed
                )
        }
        "forgecode_task_dispatch_orchestration" => {
            matches!(
                plan.classification.request_class,
                RequestClass::Assimilation
            ) && has_tool_route
        }
        "forgecode_tool_selection_hierarchy" => {
            matches!(
                plan.classification.request_class,
                RequestClass::Assimilation
            ) && (has_workspace_capability || has_web_capability || has_tool_route)
        }
        "forgecode_grounded_response_hygiene" => matches!(
            plan.classification.request_class,
            RequestClass::Assimilation
        ),
        "forgecode_specialized_tool_usage_hygiene" => {
            matches!(
                plan.classification.request_class,
                RequestClass::Assimilation
            ) && (has_workspace_capability || has_web_capability || has_tool_route)
        }
        "forgecode_subagent_brief_hygiene" => {
            matches!(
                plan.classification.request_class,
                RequestClass::Assimilation
            ) && has_tool_route
        }
        "forgecode_plan_artifact_discipline" => matches!(
            plan.classification.request_class,
            RequestClass::Assimilation
        ),
        "forgecode_reasoning_provider_matrix" => {
            matches!(
                plan.classification.request_class,
                RequestClass::Assimilation
            ) && has_tooling_or_mixed_route
        }
        "forgecode_cli_debug_regression_loop" => {
            matches!(
                plan.classification.request_class,
                RequestClass::Assimilation
            ) && has_tool_route
        }
        "forgecode_conflict_resolution_plan_first" => {
            matches!(
                plan.classification.request_class,
                RequestClass::Assimilation
            ) && mentions_conflict
        }
        "forgecode_master_single_agent_workflow" => matches!(
            plan.classification.request_class,
            RequestClass::Assimilation
        ),
        _ => false,
    }
}

fn workflow_signal_runtime_actionable(signal: &str) -> bool {
    matches!(
        signal,
        "retry_selected_plan"
            | "feedback_retry_selected_plan"
            | "emit_workflow_gate_diagnostic_with_single_retry_path"
            | "require_multi_language_tooling_synthesis_evidence"
            | "require_multi_provider_tooling_synthesis_evidence"
            | "require_multi_provider_family_synthesis_evidence"
            | "require_context_surface_alignment_for_tooling_synthesis"
            | "structured_findings_emitted"
            | "checklist_plan_emitted"
            | "verification_criteria_defined"
            | "implementation_handoff_emitted"
            | "final_composed_synthesis_emitted"
            | "task_role_selected_before_dispatch"
            | "independent_subtasks_parallelized"
            | "disjoint_write_ownership_declared"
            | "subagent_results_integrated_before_emit"
            | "semantic_discovery_selected_before_exact_search"
            | "exact_pattern_search_selected_for_symbol_match"
            | "known_path_direct_read_selected"
            | "independent_tool_calls_parallelized"
            | "tool_grounded_verification_before_response"
            | "step_checkpoint_state_updated"
            | "completion_claim_after_execution"
            | "completion_claim_after_validation"
            | "specialized_tool_selected_over_shell_for_file_ops"
            | "shell_usage_reserved_for_terminal_operations"
            | "simple_lookup_not_dispatched_to_subagent"
            | "direct_read_or_search_before_subagent_launch"
            | "subagent_brief_declares_work_mode"
            | "subagent_brief_declares_output_contract"
            | "subagent_brief_declares_context_scope"
            | "subagent_result_synthesis_emitted"
    )
}

fn plan_reason_contains(plan: &OrchestrationPlan, markers: &[&str]) -> bool {
    let contains_marker = |value: &str| {
        let lowered = value.to_ascii_lowercase();
        markers.iter().any(|marker| lowered.contains(marker))
    };
    if plan
        .classification
        .reasons
        .iter()
        .any(|reason| contains_marker(reason))
    {
        return true;
    }
    if plan
        .selected_plan
        .reasons
        .iter()
        .any(|reason| contains_marker(reason))
    {
        return true;
    }
    if plan.alternative_plans.iter().any(|candidate| {
        candidate
            .reasons
            .iter()
            .any(|reason| contains_marker(reason))
    }) {
        return true;
    }
    if let Some(prompt) = &plan.clarification_prompt {
        return contains_marker(prompt);
    }
    false
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
