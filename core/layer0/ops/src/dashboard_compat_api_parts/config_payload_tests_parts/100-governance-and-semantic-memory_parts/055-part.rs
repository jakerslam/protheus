#[test]
fn compare_workflow_completes_missing_web_evidence_from_latent_candidates() {
    let root = governance_temp_root();
    let snapshot = governance_ok_snapshot();
    let created = handle(
        root.path(),
        "POST",
        "/api/agents",
        br#"{"name":"compare-workflow-latent-agent","role":"researcher"}"#,
        &snapshot,
    )
    .expect("agent create");
    let agent_id = clean_agent_id(
        created
            .payload
            .get("agent_id")
            .or_else(|| created.payload.get("id"))
            .and_then(Value::as_str)
            .unwrap_or(""),
    );
    assert!(!agent_id.is_empty());
    write_json(
        &governance_test_chat_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "response": "<function=workspace_analyze>{\"path\":\".\",\"query\":\"compare this system (infring) to openclaw\",\"full\":true}</function>"
                },
                {
                    "response": "Using both local and external evidence, Infring centers workflow-gated synthesis while OpenClaw emphasizes governed web/media tooling."
                }
            ],
            "calls": []
        }),
    );
    write_json(
        &governance_test_tool_script_path(root.path()),
        &json!({
            "queue": [
                {
                    "tool": "terminal_exec",
                    "payload": {
                        "ok": true,
                        "status": "ok",
                        "summary": "Local workspace evidence shows workflow-gated synthesis via complex_prompt_chain_v1."
                    }
                },
                {
                    "tool": "batch_query",
                    "payload": {
                        "ok": true,
                        "status": "ok",
                        "summary": "External web evidence highlights OpenClaw's governed web/media tooling."
                    }
                }
            ],
            "calls": []
        }),
    );
    let response = handle(
        root.path(),
        "POST",
        &format!("/api/agents/{agent_id}/message"),
        br#"{"message":"compare this system (infring) to openclaw"}"#,
        &snapshot,
    )
    .expect("message response");
    let tool_names = response
        .payload
        .get("tools")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.get("name").and_then(Value::as_str).map(ToString::to_string))
        .collect::<Vec<_>>();
    assert_eq!(tool_names, vec!["workspace_analyze".to_string(), "batch_query".to_string()]);
    let response_text = response
        .payload
        .get("response")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(response_text.contains("Infring"), "{response_text}");
    assert!(response_text.contains("OpenClaw"), "{response_text}");
}

#[test]
fn workflow_more_tooling_detector_matches_compare_follow_up_question() {
    assert!(workflow_response_requests_more_tooling(
        "Would you like me to search for specific OpenClaw technical documentation or architecture details to enable a more substantive comparison?"
    ));
}

#[test]
fn workspace_plus_web_comparison_payload_targets_openclaw_docs() {
    let payload = workspace_plus_web_comparison_web_payload_from_message(
        "compare this system (infring) to openclaw",
    )
    .expect("comparison payload");
    assert_eq!(payload.get("source").and_then(Value::as_str), Some("web"));
    assert_eq!(
        payload.get("query").and_then(Value::as_str),
        Some("OpenClaw AI assistant architecture features docs")
    );
    let queries = payload
        .get("queries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(!queries.is_empty());
    assert!(queries.iter().any(|row| {
        row.as_str()
            .map(|value| value.contains("site:openclaw.ai"))
            .unwrap_or(false)
    }));
}

#[test]
fn inline_tool_web_search_comparison_hydrates_targeted_openclaw_query_pack() {
    let input = normalize_inline_tool_execution_input(
        "web_search",
        &json!({"query":"OpenClaw AI agent system features capabilities"}),
        "compare this system (infring) to openclaw",
    );
    assert_eq!(
        input.get("query").and_then(Value::as_str),
        Some("OpenClaw AI assistant architecture features docs")
    );
    let queries = input
        .get("queries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(queries.len() >= 3, "{queries:?}");
    assert!(queries.iter().any(|row| {
        row.as_str()
            .map(|value| value.contains("site:openclaw.ai"))
            .unwrap_or(false)
    }));
}

#[test]
fn framework_catalog_web_payload_targets_named_framework_queries() {
    let payload = framework_catalog_web_payload_from_query("top AI agentic frameworks")
        .expect("framework payload");
    assert_eq!(payload.get("source").and_then(Value::as_str), Some("web"));
    let query = payload.get("query").and_then(Value::as_str).unwrap_or("");
    assert!(query.contains("top AI agent frameworks"), "{query}");
    assert!(query.contains("LangGraph"), "{query}");
    assert!(query.contains("OpenAI Agents SDK"), "{query}");
    assert!(query.contains("official docs"), "{query}");
    assert!(!query.contains("vs"), "{query}");
    let queries = payload
        .get("queries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(queries.len() >= 6, "{queries:?}");
    assert!(queries.iter().any(|row| {
        row.as_str()
            .map(|value| value.contains("CrewAI"))
            .unwrap_or(false)
    }));
    assert!(queries.iter().any(|row| {
        row.as_str()
            .map(|value| value.contains("landscape"))
            .unwrap_or(false)
    }));
    assert!(queries.iter().any(|row| {
        row.as_str()
            .map(|value| value.contains("site:openai.github.io/openai-agents-python"))
            .unwrap_or(false)
    }));
    assert!(queries.iter().any(|row| {
        row.as_str()
            .map(|value| value.contains("site:microsoft.github.io"))
            .unwrap_or(false)
    }));
    assert!(queries.iter().any(|row| {
        row.as_str()
            .map(|value| value.contains("site:github.com huggingface/smolagents"))
            .unwrap_or(false)
    }));
}

#[test]
fn inline_tool_web_search_hydrates_framework_catalog_queries_from_broad_prompt() {
    let input = normalize_inline_tool_execution_input(
        "web_search",
        &json!({"query":"top AI agentic frameworks"}),
        "Try to web search \"top AI agentic frameworks\" and return the results",
    );
    let query = input.get("query").and_then(Value::as_str).unwrap_or("");
    assert!(query.contains("LangGraph"), "{query}");
    let queries = input
        .get("queries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    assert!(queries.len() >= 6, "{queries:?}");
    assert!(queries.iter().any(|row| {
        row.as_str()
            .map(|value| value.contains("site:openai.github.io/openai-agents-python"))
            .unwrap_or(false)
    }));
}

#[test]
fn summarize_unknown_workspace_analyze_payload_prefers_stdout_findings() {
    let summary = summarize_unknown_tool_payload(
        "workspace_analyze",
        &json!({
            "ok": true,
            "stdout": "docs/workspace/SRS.md:42: response_workflow\ndocs/workspace/SRS.md:43: complex_prompt_chain_v1\nclient/runtime/config/runtime.json:4: resident IPC"
        }),
    );
    assert!(summary.contains("response_workflow"), "{summary}");
    assert!(summary.contains("complex_prompt_chain_v1"), "{summary}");
    assert!(!summary.contains("`workspace_analyze` completed"), "{summary}");
}

#[test]
fn rewrite_workspace_analyze_raw_json_result_into_local_evidence_summary() {
    let rewritten = rewrite_tool_result_for_user_summary(
        "workspace_analyze",
        "Key findings: {\"stdout\":\"docs/workspace/SRS.md:42: response_workflow\\ndocs/workspace/SRS.md:43: complex_prompt_chain_v1\\nclient/runtime/config/runtime.json:4: resident IPC\"}",
    )
    .unwrap_or_default();
    assert!(rewritten.contains("Local workspace evidence"), "{rewritten}");
    assert!(rewritten.contains("response_workflow"), "{rewritten}");
    assert!(!rewritten.contains("{\"stdout\""), "{rewritten}");
}

#[test]
fn summarize_workspace_analyze_prefers_stdout_over_claim_bundle_dump() {
    let summary = summarize_tool_payload(
        "workspace_analyze",
        &json!({
            "ok": true,
            "stdout": "docs/workspace/SRS.md:42: response_workflow\\ndocs/workspace/SRS.md:43: complex_prompt_chain_v1\\nclient/runtime/config/runtime.json:4: resident IPC",
            "tool_pipeline": {
                "claim_bundle": {
                    "claims": [
                        {
                            "status": "supported",
                            "text": "{\"command_translated\":false,\"cwd\":\"/Users/jay/.openclaw/workspace\",\"executed_command\":\"rg -n ...\"}"
                        }
                    ]
                }
            }
        }),
    );
    assert!(summary.contains("response_workflow"), "{summary}");
    assert!(summary.contains("complex_prompt_chain_v1"), "{summary}");
    assert!(!summary.contains("{\"command_translated\""), "{summary}");
}

#[test]
fn summarize_tool_payload_strips_redundant_key_findings_from_claims() {
    let summary = summarize_tool_payload(
        "batch_query",
        &json!({
            "ok": true,
            "tool_pipeline": {
                "claim_bundle": {
                    "claims": [
                        {
                            "status": "supported",
                            "text": "Key findings: openclaw.ai: OpenClaw — Personal AI Assistant"
                        }
                    ]
                }
            }
        }),
    );
    assert_eq!(
        summary,
        "Key findings: openclaw.ai: OpenClaw — Personal AI Assistant"
    );
}

#[test]
fn summarize_web_search_batch_query_payload_prefers_native_summary_over_claim_bundle() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "type": "batch_query",
            "summary": "Key findings: langchain.com: LangGraph overview.; crewai.com: CrewAI overview.; openai.github.io: OpenAI Agents SDK overview.",
            "evidence_refs": [
                {"title":"Web result from langchain.com","locator":"https://www.langchain.com/langgraph"},
                {"title":"Web result from crewai.com","locator":"https://crewai.com/"},
                {"title":"Web result from openai.github.io","locator":"https://openai.github.io/openai-agents-python/"}
            ],
            "tool_pipeline": {
                "claim_bundle": {
                    "claims": [
                        {
                            "status": "partial",
                            "text": "Key findings: langchain.com: LangGraph overview."
                        }
                    ]
                }
            }
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("langgraph"), "{summary}");
    assert!(lowered.contains("crewai"), "{summary}");
    assert!(lowered.contains("openai agents sdk"), "{summary}");
}

#[test]
fn response_tools_summary_preserves_batch_query_domains_in_key_findings() {
    let summary = response_tools_summary_for_user(
        &[json!({
            "name": "batch_query",
            "result": "Key findings: docs.openclaw.ai: OpenClaw is a self-hosted gateway that connects chat apps to AI; openclaw.ai: OpenClaw personal assistant overview."
        })],
        4,
    );
    assert!(summary.contains("docs.openclaw.ai"), "{summary}");
    assert_ne!(summary.trim(), "Here's what I found:\n- batch query: docs.");
}

#[test]
fn summarize_web_search_framework_payload_rewrites_noisy_sources_from_evidence_refs() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "query": "top AI agentic frameworks",
            "summary": "Key findings: langgraph.com.cn: LangGraph overview; zhihu.com: AutoGen discussion; crewai.org.cn: CrewAI overview.",
            "evidence_refs": [
                {"title":"LangGraph: Agent Orchestration Framework for Reliable AI Agents - LangChain","locator":"https://www.langchain.com/langgraph"},
                {"title":"CrewAI official site","locator":"https://crewai.com/"},
                {"title":"OpenAI Agents SDK docs","locator":"https://openai.github.io/openai-agents-python/"}
            ]
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("langgraph"), "{summary}");
    assert!(lowered.contains("crewai"), "{summary}");
    assert!(lowered.contains("openai agents sdk"), "{summary}");
    assert!(!lowered.contains("langgraph.com.cn"), "{summary}");
    assert!(!lowered.contains("crewai.org.cn"), "{summary}");
    assert!(!lowered.contains("zhihu.com"), "{summary}");
}

#[test]
fn summarize_web_search_framework_payload_prefers_locator_domain_over_noisy_title_domain() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "query": "top AI agentic frameworks",
            "summary": "Key findings: framework search returned official documentation hits.",
            "evidence_refs": [
                {"title":"Web result from academy.langchain.com","locator":"https://www.langchain.com/langgraph"},
                {"title":"Web result from watsonx.ai","locator":"https://crewai.com/"},
                {"title":"Web result from github.com","locator":"https://github.com/huggingface/smolagents"}
            ]
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("langgraph (langchain.com)"), "{summary}");
    assert!(lowered.contains("crewai (crewai.com)"), "{summary}");
    assert!(!lowered.contains("academy.langchain.com"), "{summary}");
    assert!(!lowered.contains("watsonx.ai"), "{summary}");
}
