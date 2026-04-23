#[derive(Clone, Copy)]
struct WorkflowDefinition {
    name: &'static str,
    workflow_type: &'static str,
    default_workflow: bool,
    description: &'static str,
    stages: &'static [&'static str],
    final_response_policy: &'static str,
    gate_contract: &'static str,
}

const COMPLEX_PROMPT_CHAIN_V1_STAGES: &[&str] = &[
    "gate_1_need_tool_access",
    "gate_2_task_decomposition_or_info_check",
    "gate_3_minimal_tool_selection",
    "gate_4_execute_and_wait_if_needed",
    "gate_5_result_collection_and_synthesis",
    "gate_6_previous_turn_coherence_check",
    "gate_7_final_output_or_grounded_failure",
];

const SIMPLE_CONVERSATION_V1_STAGES: &[&str] = &[
    "gate_1_need_tool_access",
    "gate_2_info_analysis",
    "gate_7_final_output_or_grounded_failure",
];

const CONVERSATION_BYPASS_MAX_TURNS: u64 = 3;

const WORKFLOW_LIBRARY: &[WorkflowDefinition] = &[
    WorkflowDefinition {
        name: "complex_prompt_chain_v1",
        workflow_type: "hard_agent_workflow",
        default_workflow: true,
        description: "Default workflow with deterministic gate checks: classify info vs task, decide whether tool calls are truly needed, execute only selected tools, synthesize evidence, and run coherence validation before final output.",
        stages: COMPLEX_PROMPT_CHAIN_V1_STAGES,
        final_response_policy: "llm_authored_when_online",
        gate_contract: "workflow_gate_v3",
    },
    WorkflowDefinition {
        name: "simple_conversation_v1",
        workflow_type: "hard_agent_workflow",
        default_workflow: false,
        description: "Reserved lightweight workflow slot for direct conversation. It still passes through workflow gate checks so turn control remains centralized.",
        stages: SIMPLE_CONVERSATION_V1_STAGES,
        final_response_policy: "llm_authored_when_online",
        gate_contract: "workflow_gate_v1",
    },
    WorkflowDefinition {
        name: "conversation_bypass_v1",
        workflow_type: "hard_agent_workflow",
        default_workflow: false,
        description: "Explicit direct-conversation override workflow. It remains workflow-gated, but prioritizes direct response continuity over additional orchestration hops.",
        stages: SIMPLE_CONVERSATION_V1_STAGES,
        final_response_policy: "llm_authored_when_online",
        gate_contract: "workflow_gate_bypass_v1",
    },
];

fn workflow_definition_to_json(definition: WorkflowDefinition) -> Value {
    json!({
        "name": definition.name,
        "workflow_type": definition.workflow_type,
        "default": definition.default_workflow,
        "description": definition.description,
        "stages": definition.stages,
        "final_response_policy": definition.final_response_policy,
        "gate_contract": definition.gate_contract
    })
}

fn workflow_definition_by_name(name: &str) -> Option<WorkflowDefinition> {
    let cleaned = clean_text(name, 80);
    if cleaned.is_empty() {
        return None;
    }
    WORKFLOW_LIBRARY
        .iter()
        .copied()
        .find(|row| row.name.eq_ignore_ascii_case(&cleaned))
}

fn default_workflow_definition() -> WorkflowDefinition {
    WORKFLOW_LIBRARY
        .iter()
        .copied()
        .find(|row| row.default_workflow)
        .unwrap_or(WORKFLOW_LIBRARY[0])
}

fn turn_workflow_library_catalog() -> Vec<Value> {
    WORKFLOW_LIBRARY
        .iter()
        .copied()
        .map(workflow_definition_to_json)
        .collect::<Vec<_>>()
}

fn default_turn_workflow_name() -> &'static str {
    default_workflow_definition().name
}

fn workflow_name_hint_from_mode(workflow_mode: &str) -> String {
    let cleaned = clean_text(workflow_mode, 120);
    if cleaned.is_empty() {
        return String::new();
    }
    let lowered = cleaned.to_ascii_lowercase();
    for marker in ["workflow=", "workflow:", "workflow/"] {
        if let Some(idx) = lowered.find(marker) {
            let start = idx + marker.len();
            if start >= cleaned.len() {
                continue;
            }
            let tail = clean_text(&cleaned[start..], 80);
            if tail.is_empty() {
                continue;
            }
            let token = tail
                .split(|ch: char| ch.is_whitespace() || ch == ',' || ch == ';' || ch == '|')
                .next()
                .unwrap_or("")
                .to_string();
            if !token.is_empty() {
                return token;
            }
        }
    }
    String::new()
}

fn selected_turn_workflow(workflow_mode: &str) -> Value {
    let hint = workflow_name_hint_from_mode(workflow_mode);
    let selected = if hint.is_empty() {
        workflow_definition_by_name(default_turn_workflow_name())
            .unwrap_or_else(default_workflow_definition)
    } else {
        workflow_definition_by_name(&hint).unwrap_or_else(default_workflow_definition)
    };
    let selection_reason = if hint.is_empty() {
        "default_library_workflow".to_string()
    } else if workflow_definition_by_name(&hint).is_some() {
        "mode_hint_workflow".to_string()
    } else {
        "mode_hint_unknown_fallback_default".to_string()
    };
    json!({
        "name": selected.name,
        "workflow_type": selected.workflow_type,
        "mode": clean_text(workflow_mode, 80),
        "selection_reason": selection_reason,
        "final_response_policy": selected.final_response_policy,
        "gate_contract": selected.gate_contract
    })
}

fn workflow_turn_contains_any(lowered: &str, markers: &[&str]) -> bool {
    markers.iter().any(|marker| lowered.contains(marker))
}

fn message_requests_conversation_bypass(message: &str) -> bool {
    let lowered = clean_text(message, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    workflow_turn_contains_any(
        &lowered,
        &[
            "break the workflow",
            "bypass the workflow",
            "workflow bypass",
            "respond directly",
            "direct mode",
            "talk freely",
            "no workflow",
            "skip workflow",
        ],
    )
}

fn message_requests_conversation_bypass_disable(message: &str) -> bool {
    let lowered = clean_text(message, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    workflow_turn_contains_any(
        &lowered,
        &[
            "resume workflow",
            "restore workflow",
            "turn workflow back on",
            "re-enable workflow",
            "enable workflow",
            "use normal workflow",
        ],
    )
}

fn message_requests_high_risk_external_action(message: &str) -> bool {
    let lowered = clean_text(message, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    workflow_turn_contains_any(
        &lowered,
        &[
            "send email",
            "send an email",
            "tweet",
            "post publicly",
            "publish",
            "deploy to production",
            "drop database",
            "delete production",
            "exfiltrate",
            "leak secrets",
        ],
    )
}

fn value_as_u64_like(value: Option<&Value>) -> u64 {
    value
        .and_then(|row| row.as_u64().or_else(|| row.as_i64().map(|v| v.max(0) as u64)))
        .unwrap_or(0)
}

fn latest_assistant_conversation_bypass_remaining_turns(active_messages: &[Value]) -> u64 {
    for row in active_messages.iter().rev() {
        let role = clean_text(row.get("role").and_then(Value::as_str).unwrap_or(""), 24)
            .to_ascii_lowercase();
        if role != "assistant" && role != "agent" {
            continue;
        }
        let from_finalization = value_as_u64_like(
            row.pointer("/response_finalization/workflow_control/conversation_bypass/remaining_turns_after"),
        );
        if from_finalization > 0 {
            return from_finalization;
        }
        let from_workflow = value_as_u64_like(
            row.pointer("/response_workflow/workflow_control/conversation_bypass/remaining_turns_after"),
        );
        if from_workflow > 0 {
            return from_workflow;
        }
    }
    0
}

fn workflow_conversation_bypass_control_from_events(workflow_events: &[Value]) -> Value {
    for row in workflow_events.iter().rev() {
        let kind = clean_text(row.get("kind").and_then(Value::as_str).unwrap_or(""), 80);
        if kind != "conversation_bypass_control" {
            continue;
        }
        if let Some(detail) = row.get("detail").filter(|detail| detail.is_object()) {
            return detail.clone();
        }
    }
    json!({
        "enabled": false,
        "source": "none",
        "reason": "not_requested",
        "remaining_turns_before": 0,
        "remaining_turns_after": 0,
        "requested_ttl_turns": CONVERSATION_BYPASS_MAX_TURNS
    })
}

fn workflow_conversation_bypass_control_from_workflow(workflow: &Value) -> Value {
    if let Some(control) = workflow
        .pointer("/workflow_control/conversation_bypass")
        .filter(|control| control.is_object())
    {
        return control.clone();
    }
    if let Some(events) = workflow.get("system_events").and_then(Value::as_array) {
        return workflow_conversation_bypass_control_from_events(events);
    }
    json!({
        "enabled": false,
        "source": "none",
        "reason": "not_requested",
        "remaining_turns_before": 0,
        "remaining_turns_after": 0,
        "requested_ttl_turns": CONVERSATION_BYPASS_MAX_TURNS
    })
}

fn workflow_conversation_bypass_control_for_turn(
    message: &str,
    active_messages: &[Value],
    gate_should_call_tools: bool,
    inline_tools_allowed: bool,
) -> Value {
    let requested_enable = message_requests_conversation_bypass(message);
    let requested_disable = message_requests_conversation_bypass_disable(message);
    let previous_remaining = latest_assistant_conversation_bypass_remaining_turns(active_messages);
    let sticky_requested = previous_remaining > 0;
    let explicit_tool_request = inline_tool_calls_allowed_for_user_message(message)
        && !message_explicitly_disallows_tool_calls(message);
    let high_risk_external_action = message_requests_high_risk_external_action(message);
    let mut enabled = false;
    let mut source = "none";
    let mut reason = "not_requested";
    let mut blocked = false;
    let mut block_reason = "";
    let mut remaining_before = previous_remaining;
    let mut remaining_after = 0u64;

    if requested_disable {
        source = "user_disable";
        reason = "disabled_by_user";
        remaining_before = previous_remaining;
        remaining_after = 0;
    } else if requested_enable || sticky_requested {
        source = if requested_enable {
            "user_override"
        } else {
            "sticky"
        };
        if high_risk_external_action {
            blocked = true;
            reason = "blocked_by_safety_gate";
            block_reason = "high_risk_external_action";
        } else if gate_should_call_tools || inline_tools_allowed || explicit_tool_request {
            blocked = true;
            reason = "blocked_by_tooling_requirement";
            block_reason = "tooling_required_or_explicit";
        } else {
            enabled = true;
            reason = if requested_enable {
                "enabled_by_user_override"
            } else {
                "continued_from_sticky_state"
            };
            let ttl_seed = if requested_enable {
                CONVERSATION_BYPASS_MAX_TURNS
            } else {
                previous_remaining.max(1)
            };
            remaining_before = ttl_seed;
            remaining_after = ttl_seed.saturating_sub(1);
        }
    }

    let workflow_mode_override = if enabled {
        "workflow=conversation_bypass_v1".to_string()
    } else {
        String::new()
    };
    let should_emit_event =
        requested_enable || requested_disable || sticky_requested || enabled || blocked;

    json!({
        "enabled": enabled,
        "source": source,
        "reason": reason,
        "blocked": blocked,
        "block_reason": block_reason,
        "requested_enable": requested_enable,
        "requested_disable": requested_disable,
        "sticky_requested": sticky_requested,
        "explicit_tool_request": explicit_tool_request,
        "gate_should_call_tools": gate_should_call_tools,
        "inline_tools_allowed": inline_tools_allowed,
        "high_risk_external_action": high_risk_external_action,
        "requested_ttl_turns": CONVERSATION_BYPASS_MAX_TURNS,
        "remaining_turns_before": remaining_before,
        "remaining_turns_after": remaining_after,
        "workflow_mode_override": workflow_mode_override,
        "should_emit_event": should_emit_event
    })
}

fn workflow_turn_is_meta_control_message(message: &str) -> bool {
    let lowered = clean_text(message, 1_200).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    workflow_turn_contains_any(
        &lowered,
        &[
            "that was just a test",
            "just a test",
            "just testing",
            "test only",
            "ignore that",
            "never mind",
            "nm",
            "thanks",
            "thank you",
            "cool",
            "sounds good",
            "did you try it",
            "did you do it",
            "what happened",
        ],
    ) && !workflow_turn_contains_any(
        &lowered,
        &[
            "search",
            "web",
            "online",
            "internet",
            "file",
            "patch",
            "edit",
            "update",
            "create",
            "read",
            "memory",
            "repo",
            "codebase",
        ],
    )
}

fn workflow_turn_task_decomposition(
    requires_file_mutation: bool,
    requires_local_lookup: bool,
    requires_live_web: bool,
) -> Vec<&'static str> {
    if requires_file_mutation {
        return vec![
            "confirm mutation target and acceptance goal",
            "apply minimal file changes for the requested outcome",
            "summarize what changed and why",
        ];
    }
    if requires_live_web {
        return vec![
            "run targeted live-web retrieval for missing facts",
            "extract actionable findings from tool receipts",
            "deliver concise source-backed answer",
        ];
    }
    if requires_local_lookup {
        return vec![
            "inspect local memory/workspace evidence",
            "collect only relevant facts for the request",
            "return grounded answer without unnecessary tooling",
        ];
    }
    vec!["answer directly from present context"]
}

fn workflow_turn_tool_decision_tree(message: &str) -> Value {
    let canonical_gate = crate::app_plane::chat_ui_turn_tool_decision_tree(message);
    let requires_file_mutation = canonical_gate
        .get("requires_file_mutation")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let requires_local_lookup = canonical_gate
        .get("requires_local_lookup")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let requires_live_web = canonical_gate
        .get("requires_live_web")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let explicit_web_intent = canonical_gate
        .get("explicit_web_intent")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let has_sufficient_information = canonical_gate
        .get("has_sufficient_information")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let should_call_tools = canonical_gate
        .get("should_call_tools")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let workflow_route = clean_text(
        canonical_gate
            .get("workflow_route")
            .and_then(Value::as_str)
            .unwrap_or(if should_call_tools { "task" } else { "info" }),
        24,
    );
    let reason_code = clean_text(
        canonical_gate
            .get("reason_code")
            .and_then(Value::as_str)
            .unwrap_or("direct_answer_default"),
        80,
    );
    let info_source = clean_text(
        canonical_gate
            .get("info_source")
            .and_then(Value::as_str)
            .unwrap_or("none"),
        24,
    );
    let recommended_tool_family = clean_text(
        canonical_gate
            .get("recommended_tool_family")
            .and_then(Value::as_str)
            .unwrap_or("none"),
        40,
    );
    let selected_tool_family = clean_text(
        canonical_gate
            .get("selected_tool_family")
            .and_then(Value::as_str)
            .unwrap_or(&recommended_tool_family),
        40,
    );
    let meta_control = canonical_gate
        .get("meta_control_message")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let status_check = canonical_gate
        .get("status_check_message")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let meta_diagnostic_request = canonical_gate
        .get("meta_diagnostic_request")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let llm_should_answer_directly = canonical_gate
        .get("llm_should_answer_directly")
        .and_then(Value::as_bool)
        .unwrap_or(!should_call_tools);
    let automatic_tool_calls_allowed = canonical_gate
        .get("automatic_tool_calls_allowed")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let tool_selection_authority = clean_text(
        canonical_gate
            .get("tool_selection_authority")
            .and_then(Value::as_str)
            .unwrap_or("llm_selected"),
        32,
    );
    let workflow_retry_limit = canonical_gate
        .get("workflow_retry_limit")
        .and_then(Value::as_i64)
        .unwrap_or(1);
    let needs_tool_access = canonical_gate
        .get("needs_tool_access")
        .and_then(Value::as_bool)
        .unwrap_or(should_call_tools);
    let gate_prompt = clean_text(
        canonical_gate
            .get("gate_prompt")
            .and_then(Value::as_str)
            .unwrap_or("Need tool access for this query?"),
        120,
    );
    let tool_family_menu = canonical_gate
        .get("tool_family_menu")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let tool_menu = canonical_gate
        .get("tool_menu")
        .cloned()
        .unwrap_or_else(|| json!([]));
    let manual_tool_selection = canonical_gate
        .get("manual_tool_selection")
        .and_then(Value::as_bool)
        .unwrap_or(needs_tool_access);
    let decomposition_steps = workflow_turn_task_decomposition(
        requires_file_mutation,
        requires_local_lookup,
        requires_live_web,
    );
    json!({
        "contract": "tool_decision_tree_v3",
        "workflow_gate_contract": "workflow_gate_v3",
        "route_classification": workflow_route,
        "workflow_route": workflow_route,
        "reason_code": reason_code,
        "requires_file_mutation": requires_file_mutation,
        "requires_local_lookup": requires_local_lookup,
        "requires_live_web": requires_live_web,
        "explicit_web_intent": explicit_web_intent,
        "has_sufficient_information": has_sufficient_information,
        "llm_should_answer_directly": llm_should_answer_directly,
        "should_call_tools": should_call_tools,
        "needs_tool_access": needs_tool_access,
        "gate_prompt": gate_prompt,
        "info_source": info_source,
        "recommended_tool_family": recommended_tool_family,
        "selected_tool_family": selected_tool_family,
        "tool_family_menu": tool_family_menu,
        "tool_menu": tool_menu,
        "manual_tool_selection": manual_tool_selection,
        "meta_control_message": meta_control,
        "status_check_message": status_check,
        "meta_diagnostic_request": meta_diagnostic_request,
        "automatic_tool_calls_allowed": automatic_tool_calls_allowed,
        "tool_selection_authority": tool_selection_authority,
        "workflow_retry_limit": workflow_retry_limit,
        "gates": {
            "gate_1": {
                "name": "needs_tool_access",
                "question": gate_prompt,
                "required": needs_tool_access,
                "route": workflow_route,
                "reason_code": reason_code
            },
            "gate_2": {
                "name": "analysis",
                "analysis_type": if workflow_route == "task" {
                    "task_decomposition"
                } else {
                    "info_sufficiency"
                },
                "task_steps": decomposition_steps,
                "requires_more_information": !has_sufficient_information
            },
            "gate_3": {
                "name": "tool_selection",
                "tooling_default": "disabled",
                "selected_family": recommended_tool_family,
                "selected_minimal": should_call_tools
            },
            "gate_4": {
                "name": "tool_execution_wait",
                "wait_for_tools": should_call_tools,
                "skip_when_no_tools": !should_call_tools
            },
            "gate_5": {
                "name": "result_synthesis",
                "source_contract": "current_request_plus_recorded_tool_receipts"
            },
            "gate_6": {
                "name": "coherence_check",
                "recent_messages_window": 2,
                "retry_limit": workflow_retry_limit,
                "failure_mode": "retry_once_then_grounded_failure"
            },
            "gate_7": {
                "name": "final_output",
                "output_contract": "final_answer_or_explicit_failure"
            }
        }
    })
}

fn workflow_library_prompt_context(message: &str, latent_tool_candidates: &[Value]) -> String {
    let broker = crate::infring_tooling_core_v1_bridge::ToolBroker::default();
    let grouped_catalog = broker.grouped_capability_catalog();
    let tool_gate = workflow_turn_tool_decision_tree(message);
    let requires_file_mutation = tool_gate
        .get("requires_file_mutation")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let has_sufficient_information = tool_gate
        .get("has_sufficient_information")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let info_source = clean_text(
        tool_gate
            .get("info_source")
            .and_then(Value::as_str)
            .unwrap_or("none"),
        40,
    );
    let should_call_tools = tool_gate
        .get("should_call_tools")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let workflow_route = clean_text(
        tool_gate
            .get("workflow_route")
            .and_then(Value::as_str)
            .unwrap_or("info"),
        20,
    );
    let reason_code = clean_text(
        tool_gate
            .get("reason_code")
            .and_then(Value::as_str)
            .unwrap_or("direct_answer_default"),
        80,
    );
    let meta_diagnostic_request = tool_gate
        .get("meta_diagnostic_request")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let explicit_web_intent = tool_gate
        .get("explicit_web_intent")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let recommended_tool_family = clean_text(
        tool_gate
            .get("recommended_tool_family")
            .and_then(Value::as_str)
            .unwrap_or("none"),
        80,
    );
    let mut lines = vec![
        format!(
            "Workflow library gate: every chat turn must pass through `{}`. The default selected workflow is `{}`.",
            "agent_workflow_library_v1",
            default_turn_workflow_name()
        ),
        "Default workflow contract: read the user request, decide whether tools are needed, run only the minimal selected tool family when required, wait for tool/system results, and write the final answer using recorded evidence.".to_string(),
        "Never emit raw `<function=...>` markup in user-facing output.".to_string(),
        "Chat operator syntax such as `tool::...` or slash tool requests are workflow hints, not pre-executed results. You still must decide whether to call the hinted tool.".to_string(),
        format!(
            "Deterministic tool gate for this turn: route={}, reason_code={}, requires_file_mutation={}, has_sufficient_information={}, info_source={}, explicit_web_intent={}, should_call_tools={}, recommended_tool_family={}, meta_diagnostic_request={}.",
            workflow_route,
            reason_code,
            requires_file_mutation,
            has_sufficient_information,
            info_source,
            explicit_web_intent,
            should_call_tools,
            recommended_tool_family
            ,
            meta_diagnostic_request
        ),
        "Decision tree v3: (1) classify route as `task` or `info`; (2) analyze sufficiency/decompose task; (3) select minimal tool family only when required; (4) wait for tool receipts if selected; (5) synthesize from recorded evidence; (6) run coherence check against the latest 2 messages with one retry; (7) return final answer or explicit grounded failure.".to_string(),
        "Selection authority: `llm_selected`. Automatic backend tool firing is not allowed.".to_string(),
        "Tooling is never default. Do not call web/file/memory tools unless the gate explicitly requires them for this turn.".to_string(),
        "Meta/control turns (for example: `that was just a test`) are direct-answer turns. Do not call web tools for those turns.".to_string(),
        "Enforcement: if `should_call_tools` is false, answer directly from available context. If true, execute at least one tool call in the selected minimal family before final synthesis.".to_string(),
    ];
    if !grouped_catalog.is_empty() {
        lines.push("Modular tool catalog by domain:".to_string());
        for group in grouped_catalog.iter().take(6) {
            let domain = serde_json::to_value(group.domain)
                .ok()
                .and_then(|value| value.as_str().map(|row| row.to_string()))
                .unwrap_or_else(|| "unknown".to_string());
            let tool_names = group
                .tools
                .iter()
                .filter(|row| row.discoverable)
                .take(6)
                .map(|row| clean_text(&row.tool_name, 80))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>();
            if tool_names.is_empty() {
                lines.push(format!(
                    "- {}: {}",
                    clean_text(&domain, 40),
                    clean_text(&group.description, 180)
                ));
            } else {
                lines.push(format!(
                    "- {}: {} Available tools: {}.",
                    clean_text(&domain, 40),
                    clean_text(&group.description, 180),
                    clean_text(&tool_names.join(", "), 240)
                ));
            }
        }
    }
    if !latent_tool_candidates.is_empty() {
        lines.push("Strong workflow hints for this turn (not yet executed):".to_string());
        for row in latent_tool_candidates.iter().take(4) {
            let tool = clean_text(row.get("tool").and_then(Value::as_str).unwrap_or(""), 80);
            let reason = clean_text(row.get("reason").and_then(Value::as_str).unwrap_or(""), 220);
            let label = clean_text(row.get("label").and_then(Value::as_str).unwrap_or(""), 80);
            let workflow_only = row
                .get("workflow_only")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let detail = if workflow_only {
                let message = clean_text(
                    row.pointer("/proposed_input/message")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    220,
                );
                if message.is_empty() {
                    format!("- {}: {}.", tool, reason)
                } else {
                    format!("- {}: {} Guidance: {}.", tool, reason, message)
                }
            } else if label.is_empty() {
                format!("- {}: {}.", tool, reason)
            } else {
                format!("- {} ({}): {}.", tool, label, reason)
            };
            lines.push(clean_text(&detail, 360));
        }
    }
    clean_text(&lines.join("\n"), 12_000)
}

fn turn_workflow_requires_final_llm(
    response_tools: &[Value],
    workflow_events: &[Value],
    draft_response: &str,
) -> bool {
    if !response_tools.is_empty() || !workflow_events.is_empty() {
        return true;
    }
    let cleaned_draft = clean_text(draft_response, 4_000);
    if cleaned_draft.is_empty() {
        return true;
    }
    let (without_inline_calls, inline_calls) = extract_inline_tool_calls(&cleaned_draft, 6);
    if !inline_calls.is_empty()
        || without_inline_calls
            .to_ascii_lowercase()
            .contains("<function=")
    {
        return true;
    }
    if response_is_no_findings_placeholder(&cleaned_draft)
        || response_looks_like_tool_ack_without_findings(&cleaned_draft)
        || workflow_response_requests_more_tooling(&cleaned_draft)
    {
        return true;
    }
    false
}

fn turn_workflow_stage_rows(
    workflow_mode: &str,
    response_tools: &[Value],
    workflow_events: &[Value],
    draft_response: &str,
) -> Vec<Value> {
    let requires_final_llm =
        turn_workflow_requires_final_llm(response_tools, workflow_events, draft_response);
    let _ = workflow_mode;
    let cleaned_draft = clean_text(draft_response, 2_000);
    let final_stage_status = if requires_final_llm {
        "pending_final_llm"
    } else {
        "no_post_synthesis_required"
    };
    vec![
        json!({
            "stage": "workflow_gate",
            "status": "enforced"
        }),
        json!({
            "stage": "initial_model_interpretation",
            "status": if cleaned_draft.is_empty() {
                "completed_empty"
            } else {
                "completed"
            },
            "draft_response_state": if cleaned_draft.is_empty() {
                "empty"
            } else if response_is_no_findings_placeholder(&cleaned_draft) {
                "no_findings"
            } else if response_looks_like_tool_ack_without_findings(&cleaned_draft) {
                "ack_only"
            } else {
                "present"
            }
        }),
        json!({
            "stage": "tool_and_system_collection",
            "status": if response_tools.is_empty() && workflow_events.is_empty() {
                "no_external_events"
            } else {
                "collected"
            },
            "tool_count": response_tools.len(),
            "system_event_count": workflow_events.len()
        }),
        json!({
            "stage": "final_llm_response",
            "required": requires_final_llm,
            "status": final_stage_status
        }),
    ]
}

fn turn_workflow_metadata(
    workflow_mode: &str,
    response_tools: &[Value],
    workflow_events: &[Value],
    draft_response: &str,
    message: &str,
) -> Value {
    let cleaned_draft = clean_text(draft_response, 4_000);
    let draft_response_state = if cleaned_draft.is_empty() {
        "empty"
    } else if response_is_no_findings_placeholder(&cleaned_draft) {
        "no_findings"
    } else if response_looks_like_tool_ack_without_findings(&cleaned_draft) {
        "ack_only"
    } else {
        "present"
    };
    let requires_final_llm =
        turn_workflow_requires_final_llm(response_tools, workflow_events, draft_response);
    let tool_gate = workflow_turn_tool_decision_tree(message);
    let conversation_bypass_control =
        workflow_conversation_bypass_control_from_events(workflow_events);
    json!({
        "contract": "agent_workflow_library_v1",
        "workflow_gate": {
            "required": true,
            "status": "enforced"
        },
        "tool_gate": tool_gate,
        "library": {
            "default_workflow": default_turn_workflow_name(),
            "available_workflows": turn_workflow_library_catalog()
        },
        "selected_workflow": selected_turn_workflow(workflow_mode),
        "tool_count": response_tools.len(),
        "system_event_count": workflow_events.len(),
        "draft_response_state": draft_response_state,
        "findings_summary": clean_text(&response_tools_summary_for_user(response_tools, 4), 2_000),
        "failure_summary": clean_text(&response_tools_failure_reason_for_user(response_tools, 4), 2_000),
        "workflow_control": {
            "conversation_bypass": conversation_bypass_control
        },
        "system_events": workflow_events,
        "stage_statuses": turn_workflow_stage_rows(workflow_mode, response_tools, workflow_events, draft_response),
        "final_llm_response": {
            "required": requires_final_llm,
            "source": "workflow_post_synthesis"
        }
    })
}

fn set_turn_workflow_final_stage_status(workflow: &mut Value, status: &str) {
    if let Some(rows) = workflow.get_mut("stage_statuses").and_then(Value::as_array_mut) {
        for row in rows.iter_mut() {
            if row
                .get("stage")
                .and_then(Value::as_str)
                .map(|value| value == "final_llm_response")
                .unwrap_or(false)
            {
                row["status"] = Value::String(clean_text(status, 80));
            }
        }
    }
}

fn workflow_response_requests_more_tooling(response: &str) -> bool {
    let lowered = clean_text(response, 800).to_ascii_lowercase();
    !lowered.is_empty()
        && [
            "i'll get you an update",
            "i will get you an update",
            "let me get you an update",
            "i'll look into",
            "i will look into",
            "let me look into",
            "i'll check",
            "i will check",
            "let me check",
            "working on it",
            "one moment",
            "stand by",
            "i'll report back",
            "i will report back",
            "let me search",
            "i'll search",
            "i will search",
            "would you like me to search",
            "would you like me to fetch",
            "search for more",
            "rerun with",
            "retry with",
            "narrower query",
            "specific source url",
            "need to search",
            "need targeted web research",
            "need more specific",
            "let me try",
            "i'll try",
            "i will try",
            "if you'd like, i can search",
            "if you would like, i can search",
            "if you'd like, i can fetch",
            "if you would like, i can fetch",
            "if you'd like, i can look deeper",
            "if you would like, i can look deeper",
            "more targeted approach",
            "another search",
            "technical documentation",
            "architecture details to enable",
        ]
        .iter()
        .any(|marker| lowered.contains(marker))
}

fn strip_dangling_inline_tool_markup(text: &str) -> String {
    let mut cleaned = text.to_string();
    loop {
        let lowered = cleaned.to_ascii_lowercase();
        let Some(start) = lowered.find("<function=") else {
            break;
        };
        let tail = &cleaned[start..];
        let end_rel = tail
            .find("</function>")
            .map(|idx| idx + "</function>".len())
            .or_else(|| tail.find('\n'))
            .unwrap_or(tail.len());
        let end = start.saturating_add(end_rel).min(cleaned.len());
        if end <= start {
            break;
        }
        cleaned.replace_range(start..end, "");
    }
    cleaned.replace("</function>", "")
}

fn sanitize_workflow_final_response_candidate(response: &str) -> String {
    let (without_inline_calls, inline_calls) = extract_inline_tool_calls(response, 6);
    let candidate = if inline_calls.is_empty() {
        response
    } else {
        without_inline_calls.trim()
    };
    let mut cleaned = clean_chat_text(
        strip_dangling_inline_tool_markup(candidate).trim(),
        32_000,
    );
    let lowered = cleaned.to_ascii_lowercase();
    let cutoff = [
        "let me try",
        "i'll try",
        "i will try",
        "let me search",
        "i'll search",
        "i will search",
        "would you like me to search",
        "would you like me to fetch",
        "if you'd like, i can search",
        "if you would like, i can search",
        "if you'd like, i can fetch",
        "if you would like, i can fetch",
        "if you'd like, i can look deeper",
        "if you would like, i can look deeper",
    ]
        .iter()
        .filter_map(|marker| lowered.find(marker))
        .min();
    if let Some(idx) = cutoff {
        cleaned = cleaned[..idx].trim().trim_end_matches(&['\n', ' ', '-', ':'][..]).to_string();
    }
    clean_chat_text(cleaned.trim(), 32_000)
}

#[cfg(test)]
mod workflow_control_tests {
    use super::*;

    #[test]
    fn conversation_bypass_control_enables_for_direct_override_phrase() {
        let control = workflow_conversation_bypass_control_for_turn(
            "break the workflow and respond directly",
            &[],
            false,
            false,
        );
        assert_eq!(control.get("enabled").and_then(Value::as_bool), Some(true));
        assert_eq!(
            control.get("source").and_then(Value::as_str),
            Some("user_override")
        );
        assert_eq!(
            control.get("workflow_mode_override").and_then(Value::as_str),
            Some("workflow=conversation_bypass_v1")
        );
    }

    #[test]
    fn conversation_bypass_control_blocks_when_tooling_is_required() {
        let control = workflow_conversation_bypass_control_for_turn(
            "break the workflow and respond directly",
            &[],
            true,
            true,
        );
        assert_eq!(control.get("enabled").and_then(Value::as_bool), Some(false));
        assert_eq!(control.get("blocked").and_then(Value::as_bool), Some(true));
        assert_eq!(
            control.get("block_reason").and_then(Value::as_str),
            Some("tooling_required_or_explicit")
        );
    }

    #[test]
    fn conversation_bypass_control_continues_sticky_state() {
        let active_messages = vec![json!({
            "role": "assistant",
            "response_finalization": {
                "workflow_control": {
                    "conversation_bypass": {
                        "remaining_turns_after": 2
                    }
                }
            }
        })];
        let control =
            workflow_conversation_bypass_control_for_turn("status?", &active_messages, false, false);
        assert_eq!(control.get("enabled").and_then(Value::as_bool), Some(true));
        assert_eq!(control.get("source").and_then(Value::as_str), Some("sticky"));
        assert_eq!(
            control.get("remaining_turns_before").and_then(Value::as_u64),
            Some(2)
        );
        assert_eq!(
            control.get("remaining_turns_after").and_then(Value::as_u64),
            Some(1)
        );
    }

    #[test]
    fn conversation_bypass_control_disables_when_user_requests_resume() {
        let active_messages = vec![json!({
            "role": "assistant",
            "response_finalization": {
                "workflow_control": {
                    "conversation_bypass": {
                        "remaining_turns_after": 2
                    }
                }
            }
        })];
        let control = workflow_conversation_bypass_control_for_turn(
            "resume workflow now",
            &active_messages,
            false,
            false,
        );
        assert_eq!(control.get("enabled").and_then(Value::as_bool), Some(false));
        assert_eq!(
            control.get("source").and_then(Value::as_str),
            Some("user_disable")
        );
        assert_eq!(
            control.get("remaining_turns_after").and_then(Value::as_u64),
            Some(0)
        );
    }
}
