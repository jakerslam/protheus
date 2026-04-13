#[test]
fn ack_only_detector_flags_key_findings_source_scaffold_copy() {
    assert!(response_looks_like_tool_ack_without_findings(
        "Key findings for \"Infring AI vs competitors comparison 2024\": - Potential sources: hai.stanford.edu, artificialanalysis.ai, epoch.ai."
    ));
}

#[test]
fn ack_only_detector_flags_duckduckgo_findings_placeholder_copy() {
    assert!(response_looks_like_tool_ack_without_findings(
        "I couldn't extract usable findings for this yet. The search response came from https://duckduckgo.com/html/?q=agent+systems"
    ));
}

#[test]
fn response_tools_summary_drops_ack_only_tool_rows() {
    let synthesized = response_tools_summary_for_user(
        &[json!({
            "name": "web_search",
            "is_error": false,
            "result": "Web search completed."
        })],
        4,
    );
    assert!(synthesized.is_empty());
}

#[test]
fn response_tools_summary_drops_key_findings_source_scaffold_rows() {
    let synthesized = response_tools_summary_for_user(
        &[json!({
            "name": "batch_query",
            "is_error": false,
            "result": "Key findings for \"Infring AI vs competitors comparison 2024\": - Potential sources: hai.stanford.edu, artificialanalysis.ai, epoch.ai."
        })],
        4,
    );
    assert!(synthesized.is_empty());
}

#[test]
fn finalize_user_facing_response_replaces_ack_with_findings() {
    let finalized = finalize_user_facing_response(
        "Web search completed.".to_string(),
        Some("Here's what I found:\n- arxiv.org/abs/2601.12345".to_string()),
    );
    let lowered = finalized.to_ascii_lowercase();
    assert!(!lowered.contains("web search completed"));
    assert!(lowered.contains("here's what i found"));
    assert!(!response_looks_like_tool_ack_without_findings(&finalized));
}

#[test]
fn finalize_user_facing_response_replaces_ack_without_findings() {
    let finalized = finalize_user_facing_response("Web search completed.".to_string(), None);
    let lowered = finalized.to_ascii_lowercase();
    assert!(!lowered.contains("web search completed"));
    assert!(lowered.contains("usable tool findings"));
    assert!(!response_looks_like_tool_ack_without_findings(&finalized));
}

#[test]
fn finalize_user_facing_response_rewrites_generic_tool_failure_placeholder() {
    let finalized = finalize_user_facing_response(
        "I couldn't complete system_diagnostic right now.".to_string(),
        None,
    );
    let lowered = finalized.to_ascii_lowercase();
    assert!(lowered.contains("doctor --json"));
    assert!(!lowered.contains("couldn't complete system_diagnostic right now"));
}

#[test]
fn finalize_user_facing_response_never_leaks_tool_status_text() {
    let finalized = finalize_user_facing_response(
        "Tool call finished.".to_string(),
        Some("Tool call finished.".to_string()),
    );
    let lowered = finalized.to_ascii_lowercase();
    assert!(!lowered.contains("tool call finished"));
    assert!(!response_looks_like_tool_ack_without_findings(&finalized));
}

#[test]
fn comparative_detector_matches_peer_ranking_language() {
    assert!(message_requests_comparative_answer(
        "find out how Infring ranks among its peers"
    ));
    assert!(message_requests_comparative_answer(
        "compare infring versus top competitors"
    ));
    assert!(!message_requests_comparative_answer(
        "top AI agent frameworks"
    ));
}

#[test]
fn comparative_live_web_detector_matches_openclaw_vs_workspace_language() {
    assert!(message_requests_live_web_comparison(
        "compare this system (infring) to openclaw"
    ));
    assert!(message_requests_live_web_comparison(
        "compare openclaw to this system/workspace"
    ));
}

#[test]
fn natural_web_intent_routes_openclaw_comparison_to_batch_query() {
    let route = natural_web_intent_from_user_message(
        "compare openclaw to this system/workspace"
    )
    .expect("route");
    assert_eq!(route.0, "batch_query");
    assert_eq!(
        route.1.get("source").and_then(Value::as_str),
        Some("web")
    );
    let query = route
        .1
        .get("query")
        .and_then(Value::as_str)
        .unwrap_or("");
    assert!(query.to_ascii_lowercase().contains("openclaw"));
    assert!(query.to_ascii_lowercase().contains("workspace"));
}

#[test]
fn natural_web_intent_normalizes_try_to_web_search_query() {
    let route = natural_web_intent_from_user_message(
        "try to web search \"top AI agent frameworks\""
    )
    .expect("route");
    assert_eq!(route.0, "batch_query");
    assert_eq!(
        route.1.get("query").and_then(Value::as_str),
        Some("top AI agent frameworks")
    );
}

#[test]
fn natural_web_intent_routes_test_web_fetch_probe_to_example_dot_com() {
    let route = natural_web_intent_from_user_message("do a test web fetch").expect("route");
    assert_eq!(route.0, "web_fetch");
    assert_eq!(
        route.1.get("url").and_then(Value::as_str),
        Some("https://example.com")
    );
    assert_eq!(
        route.1.get("diagnostic").and_then(Value::as_str),
        Some("natural_language_test_web_fetch")
    );
}

#[test]
fn direct_tool_intent_does_not_auto_route_openclaw_probe() {
    assert!(
        direct_tool_intent_from_user_message(
            "compare openclaw to this system/workspace and do a test web fetch"
        )
        .is_none()
    );
}

#[test]
fn latent_tool_candidates_normalize_try_to_web_search_query() {
    let candidates = latent_tool_candidates_for_message(
        "try to web search \"top AI agent frameworks\"",
        &[],
    );
    let batch = candidates
        .iter()
        .find(|row| row.get("tool").and_then(Value::as_str) == Some("batch_query"))
        .cloned()
        .expect("batch query candidate");
    assert_eq!(
        batch.pointer("/proposed_input/query").and_then(Value::as_str),
        Some("top AI agent frameworks")
    );
}

#[test]
fn latent_tool_candidates_surface_chat_operator_hints_without_direct_routing() {
    let slash_candidates = latent_tool_candidates_for_message("/search top AI agentic frameworks", &[]);
    assert!(slash_candidates.iter().any(|row| {
        row.get("tool").and_then(Value::as_str) == Some("batch_query")
            && row.get("selection_source").and_then(Value::as_str) == Some("slash_search_hint")
    }));

    let explicit_candidates =
        latent_tool_candidates_for_message("tool::fetch:::https://example.com", &[]);
    assert!(explicit_candidates.iter().any(|row| {
        row.get("tool").and_then(Value::as_str) == Some("web_fetch")
            && row.get("selection_source").and_then(Value::as_str)
                == Some("explicit_tool_command")
    }));
}

#[test]
fn comparative_no_findings_fallback_is_actionable() {
    let fallback = comparative_no_findings_fallback("rank infring among peers");
    let lowered = fallback.to_ascii_lowercase();
    assert!(lowered.contains("infring"));
    assert!(lowered.contains("strongest"));
    assert!(lowered.contains("batch_query"));
    assert!(!response_is_no_findings_placeholder(&fallback));
}

#[test]
fn tooling_failure_fallback_triggers_for_diagnostic_prompt() {
    let fallback = maybe_tooling_failure_fallback(
        "so the tooling isnt working at all?",
        "I couldn't extract usable findings for \"current technology news\" yet.",
        "",
    )
    .expect("fallback");
    let lowered = fallback.to_ascii_lowercase();
    assert!(lowered.contains("partially working"));
    assert!(lowered.contains("batch_query"));
    assert!(lowered.contains("doctor --json"));
}

#[test]
fn tooling_failure_fallback_triggers_for_repeated_placeholder_loop() {
    let repeated = "I couldn't extract usable findings for \"current technology news\" yet.";
    let fallback = maybe_tooling_failure_fallback("?", repeated, repeated).expect("fallback");
    assert!(fallback.to_ascii_lowercase().contains("parse miss"));
}

#[test]
fn system_diagnostic_failure_summary_is_not_generic_dead_end() {
    let summary = user_facing_tool_failure_summary(
        "system_diagnostic",
        &json!({"ok": false, "error": "request_read_failed"}),
    )
    .expect("summary");
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("diagnose manually"));
    assert!(!lowered.contains("couldn't complete `system_diagnostic` right now"));
}

#[test]
fn web_search_request_read_failed_summary_is_actionable() {
    let summary = user_facing_tool_failure_summary(
        "web_search",
        &json!({"ok": false, "error": "request_read_failed:Resource temporarily unavailable (os error 35)"}),
    )
    .expect("summary");
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("retry transient failures"));
    assert!(lowered.contains("doctor --json"));
    assert!(lowered.contains("request_read_failed"));
}

#[test]
fn web_search_context_guard_failure_summary_is_actionable() {
    let summary = user_facing_tool_failure_summary(
        "web_search",
        &json!({"ok": false, "error": "Context overflow: estimated context size exceeds safe threshold during tool loop."}),
    )
    .expect("summary");
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("fit safely in context"));
    assert!(lowered.contains("narrower query"));
}

#[test]
fn transient_tool_failure_detects_request_read_failed_signature() {
    assert!(transient_tool_failure(&json!({
        "ok": false,
        "error": "request_read_failed:Resource temporarily unavailable (os error 35)"
    })));
}

#[test]
fn web_search_summary_avoids_completed_placeholder_copy() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "query": "agent reliability",
            "summary": "safe search region picker noise only"
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(!lowered.contains("web search completed"));
    assert!(!lowered.contains("completed."));
    assert!(!lowered.trim().is_empty());
}

#[test]
fn web_search_summary_reports_root_cause_without_legacy_no_findings_template() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "query": "ai assistant systems comparison 2024 capabilities landscape",
            "requested_url": "https://duckduckgo.com/html/?q=ai+assistant+systems+comparison",
            "domain": "duckduckgo.com",
            "summary": "AI assistant systems comparison 2024 capabilities landscape at DuckDuckGo All Regions Argentina Australia Safe search Any time"
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("low-signal"));
    assert!(lowered.contains("batch_query"));
    assert!(!lowered.contains("search response came from"));
    assert!(!lowered.contains("couldn't extract usable findings"));
}

#[test]
fn web_search_summary_discards_potential_sources_scaffold_output() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "query": "Infring AI agent platform capabilities features 2024",
            "summary": "Key findings for \"Infring AI agent platform capabilities features 2024\":\n- Potential sources: nlplogix.com, gartner.com, insightpartners.com.\n- Potential sources: salesforce.com, microsoft.com, lyzr.ai."
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(!lowered.contains("potential sources:"));
    assert!(!lowered.contains("key findings for"));
    assert!(lowered.contains("low-signal") || lowered.contains("no extractable findings"));
}

#[test]
fn web_search_summary_uses_content_domains_when_summary_is_search_chrome() {
    let summary = summarize_tool_payload(
        "web_search",
        &json!({
            "ok": true,
            "query": "latest technology news today",
            "requested_url": "https://duckduckgo.com/html/?q=latest+technology+news+today",
            "summary": "latest technology news today at DuckDuckGo All Regions Argentina Australia Safe Search Any Time",
            "content": "latest technology news today at DuckDuckGo All Regions Any Time Tech News | Today's Latest Technology News | Reuters www.reuters.com/technology/ Find latest technology news from every corner of the globe. Technology: Latest Tech News Articles Today | AP News apnews.com/technology Don't miss an update on the latest tech news. The Latest News in Technology | PCMag www.pcmag.com/news Get the latest technology news and in-depth analysis."
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(
        lowered.contains("key web findings"),
        "unexpected summary: {summary}"
    );
    assert!(lowered.contains("reuters.com"));
    assert!(!lowered.contains("couldn't extract usable findings"));
    assert!(!lowered.contains("search response came from"));
}

#[test]
fn batch_query_summary_rewrites_unsynthesized_domain_dump_to_structured_evidence() {
    let summary = summarize_tool_payload(
        "batch_query",
        &json!({
            "ok": true,
            "status": "ok",
            "summary": "Web benchmark synthesis: bing.com: compare [A with B] vs compare A [with B] | WordReference Forums — https://forum.wordreference.com/threads/compare-a-with-b-vs-compare-a-with-b.4047424/",
            "evidence_refs": [
                {
                    "title": "OpenClaw — Personal AI Assistant",
                    "locator": "https://openclaw.ai/"
                }
            ]
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("batch query evidence"));
    assert!(!lowered.contains("bing.com: compare"));
}

#[test]
fn batch_query_context_guard_comparison_uses_comparative_fallback() {
    let summary = summarize_tool_payload(
        "batch_query",
        &json!({
            "ok": true,
            "status": "ok",
            "query": "compare openclaw to this system/workspace",
            "source": "web",
            "summary": "Context overflow: estimated context size exceeds safe threshold during tool loop."
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(lowered.contains("infring is strongest"));
    assert!(lowered.contains("source-backed ranked table"));
}

#[test]
fn batch_query_summary_rewrites_no_useful_information_copy_to_actionable_guidance() {
    let summary = summarize_tool_payload(
        "batch_query",
        &json!({
            "ok": true,
            "status": "ok",
            "query": "top AI agent frameworks",
            "summary": "Search returned no useful information."
        }),
    );
    let lowered = summary.to_ascii_lowercase();
    assert!(
        lowered.contains("usable tool findings") || lowered.contains("source-backed findings"),
        "unexpected summary: {summary}"
    );
    assert!(!lowered.contains("search returned no useful information"));
}

#[test]
fn maybe_tooling_failure_fallback_rewrites_context_guard_diagnosis() {
    let fallback = maybe_tooling_failure_fallback(
        "why is web search failing lately",
        "Context overflow: estimated context size exceeds safe threshold during tool loop.",
        "",
    )
    .expect("fallback");
    let lowered = fallback.to_ascii_lowercase();
    assert!(lowered.contains("fit safely in context"));
    assert!(lowered.contains("partial result"));
}

#[test]
fn unsynthesized_web_snippet_detector_flags_domain_dump_copy() {
    assert!(response_looks_like_unsynthesized_web_snippet_dump(
        "From web retrieval: bing.com: OpenClaw — Personal AI Assistant — https://openclaw.ai/ bing.com: OpenClaw docs — https://openclaw.ai/docs"
    ));
    assert!(!response_looks_like_unsynthesized_web_snippet_dump(
        "In short: OpenClaw focuses on cross-platform local execution, while Infring emphasizes policy-gated orchestration and receipts."
    ));
}

#[test]
fn finalize_user_facing_response_rewrites_raw_placeholder_dump() {
    let finalized = finalize_user_facing_response(
        "Example Domain This domain is for use in documentation examples without needing permission."
            .to_string(),
        None,
    );
    let lowered = finalized.to_ascii_lowercase();
    assert!(lowered.contains("raw web output"));
    assert!(lowered.contains("batch_query"));
    assert!(!lowered.contains("without needing permission"));
}

#[test]
fn finalize_user_facing_response_unwraps_internal_payload_json_response() {
    let raw = json!({
        "agent_id": "agent-83ed64e07515",
        "response": "From web retrieval: benchmark summary with sources. https://example.com/benchmarks",
        "response_finalization": {"tool_completion": {"completion_state": "reported_findings"}},
        "tools": [{"name": "batch_query", "is_error": false, "result": "raw tool output"}],
        "turn_loop_tracking": {"ok": true},
        "turn_transaction": {"tool_execute": "complete"}
    })
    .to_string();
    let finalized = finalize_user_facing_response(raw, None);
    assert_eq!(
        finalized,
        "From web retrieval: benchmark summary with sources. https://example.com/benchmarks"
    );
    assert!(!finalized.contains("agent_id"));
    assert!(!finalized.starts_with('{'));
}

#[test]
fn finalize_user_facing_response_unwraps_wrapped_internal_payload_json_response() {
    let raw = format!(
        "tool output follows:\n{}\nend",
        json!({
            "agent_id": "agent-83ed64e07515",
            "response": "Synthesized answer with linked sources.",
            "response_finalization": {"tool_completion": {"completion_state": "reported_findings"}},
            "tools": [{"name": "batch_query", "is_error": false, "result": "raw tool output"}],
            "turn_loop_tracking": {"ok": true},
            "turn_transaction": {"tool_execute": "complete"}
        })
    );
    let finalized = finalize_user_facing_response(raw, None);
    assert_eq!(finalized, "Synthesized answer with linked sources.");
    assert!(!finalized.contains("agent_id"));
}

#[test]
fn finalize_user_facing_response_blocks_internal_payload_json_without_response() {
    let raw = json!({
        "agent_id": "agent-83ed64e07515",
        "response_finalization": {"tool_completion": {"completion_state": "reported_reason"}},
        "tools": [{"name": "manage_agent", "is_error": false, "result": "{\"ok\":true}"}],
        "turn_loop_tracking": {"ok": true},
        "turn_transaction": {"tool_execute": "complete"}
    })
    .to_string();
    let finalized = finalize_user_facing_response(raw, None);
    let lowered = finalized.to_ascii_lowercase();
    assert!(
        lowered.contains("no synthesized response")
            || lowered.contains("usable tool findings from this turn yet")
    );
    assert!(!lowered.contains("agent_id"));
    assert!(!finalized.starts_with('{'));
}

#[test]
fn summarize_tool_payload_unknown_tool_avoids_raw_json_dump() {
    let payload = json!({
        "ok": true,
        "agent_id": "agent-raw-dump",
        "runtime_model": "tool-router",
        "turn_loop_tracking": {"ok": true},
        "response_finalization": {"tool_completion": {"completion_state": "reported_findings"}},
        "result_count": 3,
        "source": "web"
    });
    let summary = summarize_tool_payload("manage_agent", &payload);
    let lowered = summary.to_ascii_lowercase();
    assert!(!summary.trim_start().starts_with('{'));
    assert!(!lowered.contains("\"agent_id\""));
    assert!(lowered.contains("completed"));
}
