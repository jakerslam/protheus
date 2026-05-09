enum ManualToolboxPrivateGateOutcome {
    Continue,
    Finalize,
}

#[allow(clippy::too_many_arguments)]
fn handle_manual_toolbox_private_gate_turn(
    workflow: &mut Value,
    message: &str,
    response_tools: &[Value],
    attempt: u64,
    attempt_provider: &str,
    attempt_model: &str,
    retried: &Value,
    retried_text: &str,
    active_manual_toolbox_category_turn: bool,
    active_manual_toolbox_family_turn: bool,
    active_manual_toolbox_tool_turn: bool,
    active_manual_toolbox_payload_turn: bool,
    manual_toolbox_no_selected: &mut bool,
    manual_toolbox_selected_category_key: &mut String,
    manual_toolbox_selected_category_label: &mut String,
    manual_toolbox_selected_family_key: &mut String,
    manual_toolbox_selected_family_label: &mut String,
    manual_toolbox_selected_tool_key: &mut String,
    manual_toolbox_selected_tool_label: &mut String,
    last_invalid_excerpt: &mut String,
    last_reject_reason: &mut String,
) -> Option<ManualToolboxPrivateGateOutcome> {
    if response_tools.is_empty() {
        if let Some(pending_request) =
            manual_toolbox_pending_request_from_tool_invocation_markup(retried_text, message)
        {
            record_manual_toolbox_pending_request_value(workflow, pending_request);
            mark_workflow_pending_gate_without_final_synthesis(
                workflow,
                "skipped_pending_tool_confirmation",
                "manual_toolbox_gate_submission",
                attempt,
            );
            return Some(ManualToolboxPrivateGateOutcome::Finalize);
        }
    }
    if active_manual_toolbox_category_turn
        && response_tools.is_empty()
        && response_is_exact_no_tool_gate_submission(retried_text)
    {
        let structured_final_answer = workflow_structured_gate_final_answer(retried_text);
        if let Some(final_answer) = structured_final_answer {
            let response_provider = clean_text(
                retried
                    .get("provider")
                    .and_then(Value::as_str)
                    .unwrap_or(attempt_provider),
                80,
            );
            let response_model = clean_text(
                retried
                    .get("runtime_model")
                    .or_else(|| retried.get("model"))
                    .and_then(Value::as_str)
                    .unwrap_or(attempt_model),
                240,
            );
            workflow["response"] = Value::String(final_answer);
            mark_workflow_direct_llm_no_tool_answer(workflow);
            workflow["final_llm_response"]["used"] = Value::Bool(true);
            workflow["final_llm_response"]["status"] = Value::String("synthesized".to_string());
            workflow["final_llm_response"]["source"] =
                Value::String("structured_gate_final_answer".to_string());
            workflow["final_llm_response"]["provider"] = Value::String(response_provider.clone());
            workflow["final_llm_response"]["model"] = Value::String(response_model.clone());
            workflow["final_llm_response"]["runtime_model"] = Value::String(response_model.clone());
            workflow["provider"] = Value::String(response_provider);
            workflow["model"] = Value::String(response_model.clone());
            workflow["runtime_model"] = Value::String(response_model);
            set_turn_workflow_final_stage_status(workflow, "synthesized");
            return Some(ManualToolboxPrivateGateOutcome::Finalize);
        }
        *manual_toolbox_no_selected = true;
        workflow["workflow_control"]["direct_response_path"] =
            Value::String("first_gate_no_tool_category".to_string());
        set_turn_workflow_final_stage_status(workflow, "first_gate_no_pending_final_output");
        return Some(ManualToolboxPrivateGateOutcome::Continue);
    }
    if active_manual_toolbox_category_turn
        && response_is_tool_bearing_category_gate_submission(retried_text)
    {
        if let Some((category_key, category_label)) = workflow_category_selection(
            &default_workflow_tool_menu_contract(),
            retried_text,
            Some(true),
        ) {
            *manual_toolbox_selected_category_key = category_key.clone();
            *manual_toolbox_selected_category_label = category_label.clone();
            workflow["tool_gate"]["selected_work_category"] = Value::String(category_key);
            workflow["workflow_control"]["direct_response_path"] = Value::String(
                manual_toolbox_pending_direct_response_path(
                    manual_toolbox_selected_category_key,
                    manual_toolbox_selected_family_key,
                    manual_toolbox_selected_tool_key,
                )
                .to_string(),
            );
            set_turn_workflow_final_stage_status(
                workflow,
                manual_toolbox_pending_stage_status(
                    manual_toolbox_selected_category_key,
                    manual_toolbox_selected_family_key,
                    manual_toolbox_selected_tool_key,
                ),
            );
        } else {
            *last_invalid_excerpt = first_sentence(retried_text, 220);
            *last_reject_reason = manual_toolbox_invalid_reject_reason(
                manual_toolbox_selected_category_key,
                manual_toolbox_selected_family_key,
                manual_toolbox_selected_tool_key,
            )
            .to_string();
            bump_workflow_quality_counter(workflow, "alignment_reject");
        }
        return Some(ManualToolboxPrivateGateOutcome::Continue);
    }
    if active_manual_toolbox_category_turn && response_tools.is_empty() {
        *last_invalid_excerpt = first_sentence(retried_text, 220);
        *last_reject_reason = manual_toolbox_invalid_reject_reason(
            manual_toolbox_selected_category_key,
            manual_toolbox_selected_family_key,
            manual_toolbox_selected_tool_key,
        )
        .to_string();
        workflow["workflow_control"]["direct_response_path"] = Value::String(
            manual_toolbox_pending_direct_response_path(
                manual_toolbox_selected_category_key,
                manual_toolbox_selected_family_key,
                manual_toolbox_selected_tool_key,
            )
            .to_string(),
        );
        set_turn_workflow_final_stage_status(
            workflow,
            manual_toolbox_pending_stage_status(
                manual_toolbox_selected_category_key,
                manual_toolbox_selected_family_key,
                manual_toolbox_selected_tool_key,
            ),
        );
        bump_workflow_quality_counter(workflow, "alignment_reject");
        return Some(ManualToolboxPrivateGateOutcome::Continue);
    }
    if active_manual_toolbox_family_turn && response_tools.is_empty() {
        if let Some((family_key, family_label)) =
            workflow_tool_family_selection_from_response(retried_text)
        {
            *manual_toolbox_selected_family_key = family_key.clone();
            *manual_toolbox_selected_family_label = family_label;
            workflow["tool_gate"]["selected_tool_family"] = Value::String(family_key);
            workflow["workflow_control"]["direct_response_path"] = Value::String(
                manual_toolbox_pending_direct_response_path(
                    manual_toolbox_selected_category_key,
                    manual_toolbox_selected_family_key,
                    manual_toolbox_selected_tool_key,
                )
                .to_string(),
            );
            set_turn_workflow_final_stage_status(
                workflow,
                manual_toolbox_pending_stage_status(
                    manual_toolbox_selected_category_key,
                    manual_toolbox_selected_family_key,
                    manual_toolbox_selected_tool_key,
                ),
            );
        } else {
            *last_invalid_excerpt = first_sentence(retried_text, 220);
            *last_reject_reason = manual_toolbox_invalid_reject_reason(
                manual_toolbox_selected_category_key,
                manual_toolbox_selected_family_key,
                manual_toolbox_selected_tool_key,
            )
            .to_string();
            bump_workflow_quality_counter(workflow, "alignment_reject");
        }
        return Some(ManualToolboxPrivateGateOutcome::Continue);
    }
    if active_manual_toolbox_tool_turn && response_tools.is_empty() {
        if let Some((tool_key, tool_label)) =
            workflow_tool_selection_from_response(manual_toolbox_selected_family_key, retried_text)
        {
            *manual_toolbox_selected_tool_key = tool_key.clone();
            *manual_toolbox_selected_tool_label = tool_label.clone();
            workflow["tool_gate"]["selected_tool"] = Value::String(tool_key);
            workflow["tool_gate"]["selected_tool_label"] = Value::String(tool_label);
            workflow["workflow_control"]["direct_response_path"] = Value::String(
                manual_toolbox_pending_direct_response_path(
                    manual_toolbox_selected_category_key,
                    manual_toolbox_selected_family_key,
                    manual_toolbox_selected_tool_key,
                )
                .to_string(),
            );
            set_turn_workflow_final_stage_status(
                workflow,
                manual_toolbox_pending_stage_status(
                    manual_toolbox_selected_category_key,
                    manual_toolbox_selected_family_key,
                    manual_toolbox_selected_tool_key,
                ),
            );
        } else {
            *last_invalid_excerpt = first_sentence(retried_text, 220);
            *last_reject_reason = manual_toolbox_invalid_reject_reason(
                manual_toolbox_selected_category_key,
                manual_toolbox_selected_family_key,
                manual_toolbox_selected_tool_key,
            )
            .to_string();
            bump_workflow_quality_counter(workflow, "alignment_reject");
        }
        return Some(ManualToolboxPrivateGateOutcome::Continue);
    }
    if active_manual_toolbox_payload_turn && response_tools.is_empty() {
        if let Some(input) = workflow_request_payload_from_response(
            manual_toolbox_selected_family_key,
            manual_toolbox_selected_tool_key,
            retried_text,
        ) {
            workflow["tool_gate"]["request_payload"] = input.clone();
            if let Some(pending_request) = manual_toolbox_pending_request_from_parts(
                manual_toolbox_selected_family_key,
                manual_toolbox_selected_tool_key,
                manual_toolbox_selected_tool_label,
                input,
                message,
            ) {
                record_manual_toolbox_pending_request_value(workflow, pending_request);
            }
        } else if let Some(fallback_pending_tool_request) =
            workflow_workspace_tool_request_inference(
                retried_text,
                message,
                manual_toolbox_selected_family_key,
            )
        {
            let fallback_payload =
                serde_json::to_string(&fallback_pending_tool_request).unwrap_or_default();
            if !fallback_payload.is_empty() {
                record_manual_toolbox_pending_request(workflow, &fallback_payload, message);
            }
        }
        if workflow
            .get("manual_toolbox_pending_tool_request")
            .filter(|value| value.is_object())
            .is_some()
        {
            mark_workflow_pending_gate_without_final_synthesis(
                workflow,
                "skipped_pending_tool_confirmation",
                "manual_toolbox_gate_submission",
                attempt,
            );
            return Some(ManualToolboxPrivateGateOutcome::Finalize);
        }
        *last_invalid_excerpt = first_sentence(retried_text, 220);
        *last_reject_reason = manual_toolbox_invalid_reject_reason(
            manual_toolbox_selected_category_key,
            manual_toolbox_selected_family_key,
            manual_toolbox_selected_tool_key,
        )
        .to_string();
        workflow["workflow_control"]["direct_response_path"] = Value::String(
            manual_toolbox_pending_direct_response_path(
                manual_toolbox_selected_category_key,
                manual_toolbox_selected_family_key,
                manual_toolbox_selected_tool_key,
            )
            .to_string(),
        );
        set_turn_workflow_final_stage_status(
            workflow,
            manual_toolbox_pending_stage_status(
                manual_toolbox_selected_category_key,
                manual_toolbox_selected_family_key,
                manual_toolbox_selected_tool_key,
            ),
        );
        bump_workflow_quality_counter(workflow, "alignment_reject");
        workflow["final_llm_response"]["runtime_interference_disabled"] = Value::Bool(true);
        workflow["final_llm_response"]["invalid_gate_draft_diagnostic_only"] = Value::Bool(true);
        return Some(ManualToolboxPrivateGateOutcome::Continue);
    }
    None
}

#[cfg(test)]
mod split_manual_toolbox_gate_tests {
    use super::*;

    #[test]
    fn split_manual_toolbox_gates_parse_family_tool_and_payload_independently() {
        let (family_key, family_label) =
            workflow_tool_family_selection_from_response("{\"tool_family\":\"web_research\"}")
                .expect("tool family selection");
        let (tool_key, tool_label) =
            workflow_tool_selection_from_response(&family_key, "{\"tool\":\"web_search\"}")
                .expect("tool selection");
        let request_payload = workflow_request_payload_from_response(
            &family_key,
            &tool_key,
            "{\"request_payload\":{\"source\":\"web\",\"query\":\"compare infring\",\"aperture\":\"medium\"}}",
        )
        .expect("request payload");
        let direct_request_payload = workflow_request_payload_from_response(
            &family_key,
            &tool_key,
            "{\"source\":\"web\",\"query\":\"compare infring\",\"aperture\":\"medium\"}",
        )
        .expect("direct request payload");
        assert_eq!(direct_request_payload, request_payload);
        let pending = manual_toolbox_pending_request_from_parts(
            &family_key,
            &tool_key,
            &tool_label,
            request_payload,
            "Compare infring to top agentic frameworks.",
        )
        .expect("split pending request");

        assert_eq!(family_key, "web_research");
        assert_eq!(family_label, "Web research");
        assert_eq!(tool_key, "web_search");
        assert!(!tool_label.is_empty());
        assert_eq!(
            pending.pointer("/input/query").and_then(Value::as_str),
            Some("compare infring")
        );
    }

    #[test]
    fn split_manual_toolbox_gate_accepts_common_family_alias_keys() {
        let aliases = [
            "{\"family\":\"web_research\"}",
            "{\"tool_family_key\":\"web_research\"}",
            "{\"selected_tool_family\":\"web_research\"}",
        ];

        for raw in aliases {
            let (family_key, family_label) =
                workflow_tool_family_selection_from_response(raw).expect("family alias");
            assert_eq!(family_key, "web_research");
            assert_eq!(family_label, "Web research");
        }
    }

    #[test]
    fn split_manual_toolbox_gate_accepts_common_tool_alias_keys() {
        let aliases = [
            "{\"selected_tool\":\"workspace_search\"}",
            "{\"tool_key\":\"workspace_search\"}",
            "{\"selected_tool_key\":\"workspace_search\"}",
        ];

        for raw in aliases {
            let (tool_key, tool_label) =
                workflow_tool_selection_from_response("workspace_files", raw)
                    .expect("tool alias");
            assert_eq!(tool_key, "workspace_search");
            assert_eq!(tool_label, "Search workspace");
        }
    }

    #[test]
    fn split_manual_toolbox_tool_prompt_lists_declared_tool_keys() {
        let prompt = workflow_tool_selection_prompt_context("workspace_files", "Workspace/files");

        assert!(prompt.contains("[\"parse_workspace\",\"file_read\",\"apply_patch\",\"workspace_search\"]"));
        assert!(prompt.contains("{\"tool\": \"<tool_key>\"}"));
        assert!(prompt.contains("workspace_search"));
    }

    #[test]
    fn split_manual_toolbox_max_attempts_honors_cd_retry_budget() {
        assert_eq!(manual_toolbox_private_gate_max_attempts(), 6);
    }

    #[test]
    fn split_manual_toolbox_promotes_one_valid_latent_candidate() {
        let candidates = json!([{
            "tool": "batch_query",
            "selected_tool_family": "web_research",
            "selected_tool_label": "Research query pack",
            "workflow_only": true,
            "selection_source": "latent_live_web_research",
            "discovery_receipt": "candidate-receipt",
            "input": {
                "source": "web",
                "query": "Compare three data tools for AI research agents.",
                "queries": [
                    "tool A official docs",
                    "tool B official docs"
                ],
                "aperture": "medium"
            }
        }]);
        let pending = manual_toolbox_pending_request_from_latent_candidates(
            &candidates,
            "Compare three data tools for AI research agents.",
        )
        .expect("single latent candidate promotion");

        assert_eq!(pending.get("source").and_then(Value::as_str), Some("latent_candidate_recovery"));
        assert_eq!(pending.get("tool_name").and_then(Value::as_str), Some("batch_query"));
        assert_eq!(
            pending.get("selected_tool_family").and_then(Value::as_str),
            Some("web_research")
        );
        assert_eq!(
            pending.pointer("/input/query").and_then(Value::as_str),
            Some("Compare three data tools for AI research agents.")
        );
        assert_eq!(
            pending.get("latent_candidate_receipt").and_then(Value::as_str),
            Some("candidate-receipt")
        );
    }

    #[test]
    fn split_manual_toolbox_refuses_ambiguous_latent_candidates() {
        let candidates = json!([
            {
                "tool": "web_search",
                "selected_tool_family": "web_research",
                "workflow_only": true,
                "input": {"query": "first", "aperture": "medium"}
            },
            {
                "tool": "web_search",
                "selected_tool_family": "web_research",
                "workflow_only": true,
                "input": {"query": "second", "aperture": "medium"}
            }
        ]);

        assert!(manual_toolbox_pending_request_from_latent_candidates(
            &candidates,
            "Find evidence."
        )
        .is_none());
    }

    #[test]
    fn split_manual_toolbox_retry_prompt_handles_empty_gate_output_from_cd() {
        let prompt = workflow_private_gate_retry_prompt_context(
            "gate_1_work_category_menu",
            "Research current options and recommend a path.",
            "tool_category_without_selection_diagnostic_only",
            "",
        );

        assert!(prompt.contains("INTERNAL RETRY"), "{prompt}");
        assert!(prompt.contains("gate_1_work_category_menu"), "{prompt}");
        assert!(
            prompt.contains("tool_category_without_selection_diagnostic_only"),
            "{prompt}"
        );
        assert!(prompt.contains("(empty response)"), "{prompt}");
        assert!(prompt.contains("exact JSON artifact"), "{prompt}");
        assert!(!prompt.contains("Infring"), "{prompt}");
    }

    #[test]
    fn workflow_workspace_tool_request_inference_builds_workspace_search_request() {
        let message = "Inspect the tiny fixture repo and identify the smallest bugfix you would make. Use workspace tools before answering.";
        let retried = "I'll inspect the repository and identify the smallest bugfix needed.";
        let pending =
            workflow_workspace_tool_request_inference(retried, message, "workspace_files")
                .expect("workspace fallback request");

        assert_eq!(
            pending.get("tool_family").and_then(Value::as_str),
            Some("workspace_files")
        );
        assert_eq!(
            pending.get("tool").and_then(Value::as_str),
            Some("workspace_search")
        );
        assert_eq!(
            pending
                .pointer("/request_payload/path")
                .and_then(Value::as_str),
            Some(".")
        );
        assert_eq!(
            pending
                .pointer("/request_payload/pattern")
                .and_then(Value::as_str),
            Some("tiny fixture repo bugfix")
        );
    }

    #[test]
    fn workflow_gate_stability_rows_score_split_gate_state_before_confirmation() {
        let workflow = json!({
            "selected_workflow": {
                "name": "simple_conversation_v1"
            },
            "workflow_control": {
                "direct_response_path": "gate_4_pending_llm_tool_request"
            },
            "tool_gate": {
                "selected_work_category": "web_research",
                "selected_tool_family": "web_research",
                "selected_tool": "web_search",
                "selected_tool_label": "Web search",
                "request_payload": {
                    "source": "web",
                    "query": "compare infring to top agentic frameworks",
                    "aperture": "medium"
                }
            },
            "tool_count": 0,
            "response": "",
            "final_llm_response": {
                "required": false,
                "status": "skipped_pending_tool_confirmation"
            }
        });
        let rows = workflow_gate_stability_rows(&workflow);

        assert_eq!(
            rows.iter()
                .find(|row| row.get("gate").and_then(Value::as_str)
                    == Some("gate_2_tool_family_menu"))
                .and_then(|row| row.get("status").and_then(Value::as_str)),
            Some("passed")
        );
        assert_eq!(
            rows.iter()
                .find(|row| row.get("gate").and_then(Value::as_str) == Some("gate_3_tool_menu"))
                .and_then(|row| row.get("status").and_then(Value::as_str)),
            Some("passed")
        );
        assert_eq!(
            rows.iter()
                .find(|row| row.get("gate").and_then(Value::as_str)
                    == Some("gate_4_request_payload_input"))
                .and_then(|row| row.get("status").and_then(Value::as_str)),
            Some("passed")
        );
    }

    #[test]
    fn workflow_gate_stability_rows_score_pending_workspace_request() {
        let workflow = json!({
            "selected_workflow": {
                "name": "simple_conversation_v1"
            },
            "workflow_control": {
                "direct_response_path": "first_gate_pending_tool_confirmation"
            },
            "tool_gate": {
                "selected_work_category": "workspace_files"
            },
            "manual_toolbox_pending_tool_request": {
                "status": "pending_confirmation",
                "tool_name": "workspace_search",
                "selected_tool_family": "workspace_files",
                "selected_tool_label": "workspace_search",
                "input": {
                    "path": ".",
                    "pattern": "tiny fixture repo bugfix"
                }
            },
            "tool_count": 0,
            "response": "",
            "final_llm_response": {
                "required": false,
                "status": "skipped_pending_tool_confirmation"
            },
            "stage_statuses": [
                {
                    "stage": "gate_1_work_category_menu",
                    "status": "presented"
                },
                {
                    "stage": "gate_6_llm_final_output",
                    "status": "skipped_pending_tool_confirmation"
                }
            ]
        });
        let rows = workflow_gate_stability_rows(&workflow);

        assert_eq!(
            rows.iter()
                .find(|row| row.get("gate").and_then(Value::as_str)
                    == Some("gate_1_work_category_menu"))
                .and_then(|row| row.get("status").and_then(Value::as_str)),
            Some("passed")
        );
        assert_eq!(
            rows.iter()
                .find(|row| row.get("gate").and_then(Value::as_str)
                    == Some("gate_2_tool_family_menu"))
                .and_then(|row| row.get("status").and_then(Value::as_str)),
            Some("passed")
        );
        assert_eq!(
            rows.iter()
                .find(|row| row.get("gate").and_then(Value::as_str) == Some("gate_3_tool_menu"))
                .and_then(|row| row.get("status").and_then(Value::as_str)),
            Some("passed")
        );
        assert_eq!(
            rows.iter()
                .find(|row| row.get("gate").and_then(Value::as_str)
                    == Some("gate_4_request_payload_input"))
                .and_then(|row| row.get("status").and_then(Value::as_str)),
            Some("passed")
        );
        assert_eq!(
            rows.iter()
                .find(|row| row.get("gate").and_then(Value::as_str)
                    == Some("gate_6_llm_final_output"))
                .and_then(|row| row.get("status").and_then(Value::as_str)),
            Some("not_applicable")
        );
    }
}
