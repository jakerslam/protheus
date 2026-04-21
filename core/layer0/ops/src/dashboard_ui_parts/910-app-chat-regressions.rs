#[cfg(test)]
mod app_chat_regression_tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    fn write_chat_script(root: &Path, payload: &Value) {
        let path = root.join("client/runtime/local/state/ui/infring_dashboard/test_chat_script.json");
        let parent = path.parent().expect("chat script parent");
        fs::create_dir_all(parent).expect("mkdir chat script");
        fs::write(path, serde_json::to_string_pretty(payload).expect("chat script json"))
            .expect("write chat script");
    }

    fn app_chat_response_text(payload: &Value) -> String {
        payload
            .get("response")
            .and_then(Value::as_str)
            .or_else(|| payload.pointer("/turn/assistant").and_then(Value::as_str))
            .unwrap_or("")
            .to_string()
    }

    fn app_chat_finalization_outcome(payload: &Value) -> String {
        payload
            .pointer("/response_finalization/outcome")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string()
    }

    fn app_chat_response_is_invalid_terminal_copy(response: &str) -> bool {
        let lowered = clean_text(response, 2_000).to_ascii_lowercase();
        if lowered.trim().is_empty() {
            return true;
        }
        lowered.starts_with("i'll get you an update")
            || lowered.starts_with("i will get you an update")
            || lowered.starts_with("let me get you an update")
            || lowered.contains("would you like me to retry")
            || lowered.contains("would you like me to try a more specific search")
            || lowered.contains("would you like me to try a different approach")
            || lowered.contains("tool completed")
            || lowered.contains("web search completed")
    }

    #[test]
    fn app_chat_rewrites_speculative_security_controls_copy_for_generic_search_prompt() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "I attempted to search for \"top AI agent frameworks\" but the system blocked the request. The security controls appear to be filtering out queries related to AI frameworks specifically, likely as a content-based security measure. The system requires proper authorization and external service allowlists.",
                        "tools": [
                            {
                                "name": "batch_query",
                                "status": "low_signal",
                                "result": "Key findings: LangGraph (langchain.com) official source captured; CrewAI official source captured; AutoGen secondary references captured.",
                                "input": {
                                    "source": "web",
                                    "query": "top AI agent frameworks official docs LangGraph OpenAI Agents SDK AutoGen CrewAI smolagents"
                                }
                            }
                        ]
                    }
                ],
                "calls": []
            }),
        );
        let lane = run_action(
            root.path(),
            "app.chat",
            &json!({
                "agent_id": "agent-regression-chat-ui",
                "input": "try doing a generic search \"top AI agent frameworks\""
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        let response = app_chat_response_text(&payload);
        let outcome = app_chat_finalization_outcome(&payload);
        let lowered = response.to_ascii_lowercase();
        assert!(!lowered.contains("security controls"), "{response}");
        assert!(!lowered.contains("allowlists"), "{response}");
        assert!(!lowered.contains("proper authorization"), "{response}");
        assert!(
            lowered.contains("found:")
                && lowered.contains("missing in this pass:")
                && lowered.contains("openai agents sdk")
                && lowered.contains("smolagents"),
            "{response}"
        );
        assert_eq!(outcome, "success_with_gaps");
    }

    #[test]
    fn app_chat_uses_structured_block_evidence_for_blocked_claims() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "The platform recognizes function requests but blocks external tool execution due to security controls and policy restrictions.",
                        "tools": [
                            {
                                "name": "batch_query",
                                "status": "blocked",
                                "type": "tool_pre_gate_blocked",
                                "error": "tool_permission_denied",
                                "result": "Tool blocked by command permission policy."
                            }
                        ]
                    }
                ],
                "calls": []
            }),
        );
        let lane = run_action(
            root.path(),
            "app.chat",
            &json!({
                "agent_id": "agent-regression-chat-ui-structured-block",
                "input": "try doing a generic search \"top AI agent frameworks\""
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        let response = app_chat_response_text(&payload);
        let outcome = app_chat_finalization_outcome(&payload);
        let lowered = response.to_ascii_lowercase();
        assert!(lowered.contains("blocked by policy"), "{response}");
        assert!(
            lowered.contains("tool_pre_gate_blocked")
                || lowered.contains("tool_permission_denied"),
            "{response}"
        );
        assert_eq!(outcome, "blocked_with_structured_evidence");
    }

    #[test]
    fn app_chat_suppresses_invalid_response_attempt_blocker_copy_without_tool_block_evidence() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "I attempted to run those web searches but the system blocked the function calls from executing entirely. It appears the security controls are preventing any web search operations at the moment, regardless of topic. The system flagged this as an invalid response attempt rather than processing the queries.",
                        "tools": [
                            {
                                "name": "batch_query",
                                "status": "low_signal",
                                "result": "No source-backed findings were produced in this pass."
                            }
                        ]
                    }
                ],
                "calls": []
            }),
        );
        let lane = run_action(
            root.path(),
            "app.chat",
            &json!({
                "agent_id": "agent-regression-chat-ui-invalid-response-attempt",
                "input": "try web search again and report the system output"
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        let response = app_chat_response_text(&payload);
        let outcome = app_chat_finalization_outcome(&payload);
        let lowered = response.to_ascii_lowercase();
        assert!(!lowered.contains("security controls"), "{response}");
        assert!(!lowered.contains("invalid response attempt"), "{response}");
        assert_eq!(outcome, "suppressed_unverified_blocker_claim");
    }

    #[test]
    fn app_chat_web_workflow_replay_suite_forces_non_empty_non_deferred_terminal_response() {
        let scenarios = vec![
            (
                "I'll get you an update on the current best AI agent frameworks.",
                json!([{ "name": "batch_query", "status": "low_signal", "result": "No source-backed findings were produced in this pass." }]),
                "try finding information about the current top agentic AI frameworks",
            ),
            (
                "Would you like me to retry with a narrower query or one specific source URL?",
                json!([{ "name": "batch_query", "status": "no_results", "result": "No useful result in this pass." }]),
                "give me an update on the current best agent frameworks",
            ),
            (
                "I attempted to run those web searches but the system blocked the function calls from executing entirely. It appears the security controls are preventing any web search operations at the moment, regardless of topic.",
                json!([{ "name": "batch_query", "status": "low_signal", "result": "No source-backed findings were produced in this pass." }]),
                "try web search again and report the system output",
            ),
        ];
        for (idx, (draft, tools, prompt)) in scenarios.into_iter().enumerate() {
            let root = tempfile::tempdir().expect("tempdir");
            write_chat_script(
                root.path(),
                &json!({
                    "queue": [
                        {
                            "response": draft,
                            "tools": tools
                        }
                    ],
                    "calls": []
                }),
            );
            let lane = run_action(
                root.path(),
                "app.chat",
                &json!({
                    "agent_id": format!("agent-regression-chat-replay-{idx}"),
                    "input": prompt
                }),
            );
            assert!(lane.ok);
            let payload = lane.payload.unwrap_or_else(|| json!({}));
            let response = app_chat_response_text(&payload);
            assert!(
                !app_chat_response_is_invalid_terminal_copy(&response),
                "invalid terminal response for scenario {idx}: {response}"
            );
        }
    }

    #[test]
    fn app_chat_forces_web_tool_attempt_for_explicit_chili_web_search_prompt() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "I'll get you an update on chili recipes.",
                        "tools": []
                    }
                ],
                "calls": []
            }),
        );
        let lane = run_action(
            root.path(),
            "app.chat",
            &json!({
                "agent_id": "agent-regression-web-force-chili",
                "input": "well try doing a web search and returning the results. make the websearch about best chili recipes",
                "__mock_web_batch_query": {
                    "ok": true,
                    "type": "batch_query",
                    "status": "ok",
                    "summary": "Key findings: allrecipes.com: Best Damn Chili Recipe.",
                    "evidence_refs": [
                        { "locator": "https://www.allrecipes.com/recipe/233613/best-damn-chili/" }
                    ]
                }
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        let response = app_chat_response_text(&payload).to_ascii_lowercase();
        assert!(
            response.contains("web search results for")
                && response.contains("best chili recipes"),
            "{response}"
        );
        let tools = payload
            .get("tools")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(tools.iter().any(|row| {
            clean_text(
                row.get("name")
                    .or_else(|| row.get("tool"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
                80,
            )
            .to_ascii_lowercase()
                == "batch_query"
        }));
        let invariant = payload
            .pointer("/response_finalization/web_invariant")
            .cloned()
            .unwrap_or_else(|| json!({}));
        assert_eq!(
            invariant.get("requires_live_web").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            invariant.get("tool_attempted").and_then(Value::as_bool),
            Some(true)
        );
        assert!(
            invariant
                .get("web_search_calls")
                .and_then(Value::as_u64)
                .unwrap_or(0)
                >= 1
        );
    }

    #[test]
    fn app_chat_fail_closes_when_live_web_requested_but_search_not_invoked() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "I'll get you an update on the current best AI agent frameworks.",
                        "tools": []
                    }
                ],
                "calls": []
            }),
        );
        let lane = run_action(
            root.path(),
            "app.chat",
            &json!({
                "agent_id": "agent-regression-web-not-invoked",
                "input": "try doing a generic search top ai agent frameworks",
                "__mock_web_batch_query": {
                    "ok": false,
                    "type": "batch_query",
                    "status": "failed",
                    "error": "mock_transport_error"
                }
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        let response = app_chat_response_text(&payload).to_ascii_lowercase();
        assert!(
            response.contains("error_code: web_tool_not_invoked"),
            "{response}"
        );
        let invariant = payload
            .pointer("/response_finalization/web_invariant")
            .cloned()
            .unwrap_or_else(|| json!({}));
        assert_eq!(
            invariant.get("classification").and_then(Value::as_str),
            Some("tool_not_invoked")
        );
        assert_eq!(
            invariant
                .get("web_search_calls")
                .and_then(Value::as_u64)
                .unwrap_or(999),
            0
        );
    }

    #[test]
    fn app_chat_exposes_web_invariant_metrics_when_live_web_requested() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "I'll get you an update on the current best AI agent frameworks.",
                        "tools": [
                            {
                                "name": "batch_query",
                                "status": "blocked",
                                "error": "web_search_nexus_delivery_denied",
                                "result": "Tool blocked by policy gate."
                            }
                        ]
                    }
                ],
                "calls": []
            }),
        );
        let lane = run_action(
            root.path(),
            "app.chat",
            &json!({
                "agent_id": "agent-regression-web-invariant",
                "input": "try finding information about the current top agentic AI frameworks"
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        let invariant = payload
            .pointer("/response_finalization/web_invariant")
            .cloned()
            .unwrap_or_else(|| json!({}));
        assert_eq!(
            invariant
                .get("requires_live_web")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            invariant.get("tool_attempted").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            invariant.get("classification").and_then(Value::as_str),
            Some("policy_blocked")
        );
    }

    #[test]
    fn app_chat_suppresses_competitive_programming_dump_from_web_lane() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "Tree Leaves problem statement. Given a tree, list all leaves in top-down left-to-right order. Input Specification: ... Sample Input ... Sample Output ... #include <stdio.h> int main() { return 0; }",
                        "tools": [
                            {
                                "name": "batch_query",
                                "status": "ok",
                                "result": "Given a tree ... input specification ... sample output ..."
                            }
                        ]
                    }
                ],
                "calls": []
            }),
        );
        let lane = run_action(
            root.path(),
            "app.chat",
            &json!({
                "agent_id": "agent-regression-competitive-dump",
                "input": "try doing a generic search \"top AI agent frameworks\""
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        let response = app_chat_response_text(&payload).to_ascii_lowercase();
        let outcome = app_chat_finalization_outcome(&payload);
        assert!(response.contains("web_tool_context_mismatch"), "{response}");
        assert_eq!(outcome, "suppressed_irrelevant_dump");
    }

    #[test]
    fn app_chat_meta_diagnostic_prompt_does_not_force_web_fallback() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "That was a test. I can explain what happened without running web tools.",
                        "tools": []
                    }
                ],
                "calls": []
            }),
        );
        let lane = run_action(
            root.path(),
            "app.chat",
            &json!({
                "agent_id": "agent-regression-meta-diagnostic",
                "input": "that was just a test"
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        let tools = payload
            .get("tools")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            !tools.iter().any(|row| {
                clean_text(
                    row.get("name")
                        .or_else(|| row.get("tool"))
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    80,
                )
                .to_ascii_lowercase()
                    == "batch_query"
            }),
            "{tools:?}"
        );
    }

    #[test]
    fn app_chat_local_file_intent_blocks_web_tooling_and_flags_violation() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "Web search results for local files: title: random page; originalUrl: https://example.com",
                        "tools": [
                            {
                                "name": "batch_query",
                                "status": "ok",
                                "result": "random metadata dump"
                            }
                        ]
                    }
                ],
                "calls": []
            }),
        );
        let lane = run_action(
            root.path(),
            "app.chat",
            &json!({
                "agent_id": "agent-regression-local-file-intent-web-block",
                "input": "can you use file tooling to list local files?"
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        assert_eq!(
            app_chat_finalization_outcome(&payload),
            "blocked_web_for_local_intent"
        );
        let invariant = payload
            .pointer("/response_finalization/web_invariant")
            .cloned()
            .unwrap_or_else(|| json!({}));
        assert_eq!(
            invariant
                .get("web_called_during_local_intent")
                .and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            invariant.get("classification").and_then(Value::as_str),
            Some("local_intent_web_violation")
        );
    }

    #[test]
    fn app_chat_malformed_tool_emit_is_fail_closed() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "<function=file_list>{\"path\":\".",
                        "tools": []
                    }
                ],
                "calls": []
            }),
        );
        let lane = run_action(
            root.path(),
            "app.chat",
            &json!({
                "agent_id": "agent-regression-malformed-tool-emit",
                "input": "check if file tooling works"
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        let response = app_chat_response_text(&payload).to_ascii_lowercase();
        assert!(response.contains("tool_call_schema_invalid"), "{response}");
        assert_eq!(
            app_chat_finalization_outcome(&payload),
            "suppressed_malformed_tool_call"
        );
    }

    #[test]
    fn app_chat_web_schema_guard_sets_terminal_retry_reason() {
        let root = tempfile::tempdir().expect("tempdir");
        write_chat_script(
            root.path(),
            &json!({
                "queue": [
                    {
                        "response": "attempting web lookup",
                        "tools": []
                    }
                ],
                "calls": []
            }),
        );
        let lane = run_action(
            root.path(),
            "app.chat",
            &json!({
                "agent_id": "agent-regression-web-schema-guard",
                "input": "web search \"a\""
            }),
        );
        assert!(lane.ok);
        let payload = lane.payload.unwrap_or_else(|| json!({}));
        let response = app_chat_response_text(&payload).to_ascii_lowercase();
        assert!(response.contains("tool_call_schema_invalid"), "{response}");
        let retry_contract = payload
            .pointer("/response_finalization/retry_contract")
            .cloned()
            .unwrap_or_else(|| json!({}));
        assert_eq!(
            retry_contract.get("max_retries").and_then(Value::as_i64),
            Some(1)
        );
        assert_eq!(
            retry_contract.get("terminal").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            retry_contract
                .get("terminal_reason")
                .and_then(Value::as_str),
            Some("tool_call_schema_invalid_pre_execution")
        );
    }

    #[test]
    fn troubleshooting_eval_flags_web_called_during_local_intent_violation() {
        let root = tempfile::tempdir().expect("tempdir");
        let lane_payload = json!({
            "response": "blocked by policy",
            "tools": [
                {
                    "name": "batch_query",
                    "status": "ok",
                    "result": "unexpected web output"
                }
            ],
            "response_finalization": {
                "outcome": "blocked_web_for_local_intent",
                "web_invariant": {
                    "requires_live_web": false,
                    "web_search_calls": 1,
                    "web_called_during_local_intent": true,
                    "classification": "local_intent_web_violation"
                },
                "tool_routing": {
                    "local_tooling_intent": true,
                    "explicit_web_intent": false,
                    "requires_live_web": false,
                    "web_allowed": false,
                    "web_called_during_local_intent": true
                }
            }
        });
        let _capture = dashboard_troubleshooting_capture_chat_exchange(
            root.path(),
            "agent-violation",
            "can you use file tooling to list local files?",
            &lane_payload,
            true,
            false,
        );
        let snapshot = dashboard_troubleshooting_capture_snapshot(
            root.path(),
            "unit_test",
            &json!({"case":"web_called_during_local_intent"}),
        );
        let report = dashboard_troubleshooting_generate_eval_report(
            &snapshot,
            "unit_test",
            "gpt-5.4",
            "test",
        );
        assert!(
            report
                .get("web_called_during_local_intent_count")
                .and_then(Value::as_i64)
                .unwrap_or(0)
                >= 1
        );
        let top_error = report
            .pointer("/top_errors/0/error_code")
            .and_then(Value::as_str)
            .unwrap_or("");
        assert_eq!(top_error, "web_called_during_local_intent");
    }

}
