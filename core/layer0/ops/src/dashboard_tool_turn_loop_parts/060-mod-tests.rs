
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn post_filter_rewrites_ack_placeholder_copy() {
        let mut payload = json!({"ok": true, "summary": "Web search completed."});
        let report = apply_post_tool_output_filter(&mut payload);
        assert_eq!(report.get("applied").and_then(Value::as_bool), Some(true));
        let lowered = payload
            .get("summary")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        assert!(lowered.contains("usable tool findings"));
        assert!(!lowered.contains("web search completed"));
    }

    #[test]
    fn post_filter_rewrites_raw_payload_dump_summary() {
        let mut payload = json!({
            "ok": true,
            "summary": "{\"agent_id\":\"agent-83ed64e07515\",\"input_tokens\":33,\"output_tokens\":85,\"latent_tool_candidates\":[],\"nexus_connection\":{},\"turn_loop_tracking\":{},\"turn_transaction\":{},\"response_finalization\":{},\"tools\":[]}"
        });
        let report = apply_post_tool_output_filter(&mut payload);
        assert_eq!(report.get("applied").and_then(Value::as_bool), Some(true));
        let summary = payload.get("summary").and_then(Value::as_str).unwrap_or("");
        assert!(summary
            .to_ascii_lowercase()
            .contains("suppressed raw runtime payload"));
    }

    #[test]
    fn post_filter_rewrites_unsynthesized_web_dump_to_actionable_copy() {
        let mut payload = json!({
            "ok": true,
            "summary": "Web benchmark synthesis: bing.com: compare [A with B] vs compare A [with B]."
        });
        let report = apply_post_tool_output_filter(&mut payload);
        assert_eq!(report.get("applied").and_then(Value::as_bool), Some(true));
        let summary = payload.get("summary").and_then(Value::as_str).unwrap_or("");
        assert!(summary
            .to_ascii_lowercase()
            .contains("source-backed answer"));
        assert!(!summary.to_ascii_lowercase().contains("bing.com"));
    }

    #[test]
    fn pre_gate_respects_confirm_for_ask_verdicts() {
        let root = tempfile::tempdir().expect("tempdir");
        let policy_path = root.path().join(TERMINAL_PERMISSION_POLICY_REL);
        if let Some(parent) = policy_path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(&policy_path, r#"{"ask_rules":["Bash(echo *)"]}"#).expect("write policy");
        let blocked = pre_tool_permission_gate(
            root.path(),
            "terminal_exec",
            &json!({"command":"echo hello"}),
        )
        .expect("blocked");
        assert_eq!(
            blocked.get("error").and_then(Value::as_str),
            Some("tool_confirmation_required")
        );
        let allowed = pre_tool_permission_gate(
            root.path(),
            "terminal_exec",
            &json!({"command":"echo hello","confirm":true}),
        );
        assert!(allowed.is_none());
    }

    #[test]
    fn pre_gate_allows_spawn_without_confirm_for_ask_verdicts() {
        let root = tempfile::tempdir().expect("tempdir");
        let policy_path = root.path().join(TERMINAL_PERMISSION_POLICY_REL);
        if let Some(parent) = policy_path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(&policy_path, r#"{"ask_rules":["spawn_subagents*"]}"#)
            .expect("write policy");
        let out = pre_tool_permission_gate(
            root.path(),
            "spawn_subagents",
            &json!({"count": 2, "objective": "parallelize"}),
        );
        assert!(out.is_none());
    }

    #[test]
    fn pre_gate_still_denies_spawn_when_policy_denies() {
        let root = tempfile::tempdir().expect("tempdir");
        let policy_path = root.path().join(TERMINAL_PERMISSION_POLICY_REL);
        if let Some(parent) = policy_path.parent() {
            std::fs::create_dir_all(parent).expect("mkdir");
        }
        std::fs::write(&policy_path, r#"{"deny_rules":["spawn_subagents*"]}"#)
            .expect("write policy");
        let blocked = pre_tool_permission_gate(
            root.path(),
            "spawn_subagents",
            &json!({"count": 2, "objective": "parallelize"}),
        )
        .expect("blocked");
        assert_eq!(
            blocked.get("error").and_then(Value::as_str),
            Some("tool_permission_denied")
        );
    }

    #[test]
    fn ingress_nexus_authorization_succeeds_for_web_search_tool_route() {
        let route = ingress_route_for_tool("web_search");
        let out =
            authorize_client_ingress_route_with_nexus_inner("tool:web_search", route, false, None)
                .expect("nexus route");
        assert_eq!(
            out.get("source").and_then(Value::as_str),
            Some(CLIENT_INGRESS_SUB_NEXUS)
        );
        assert_eq!(
            out.get("target").and_then(Value::as_str),
            Some("context_stacks")
        );
        assert_eq!(
            out.pointer("/delivery/allowed").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn ingress_nexus_authorization_fails_closed_when_pair_blocked() {
        let route = ingress_route_for_tool("web_search");
        let err =
            authorize_client_ingress_route_with_nexus_inner("tool:web_search", route, true, None)
                .expect_err("blocked");
        assert!(err.contains("lease_denied"));
    }

    #[test]
    fn ingress_nexus_authorization_fails_when_client_ingress_quiesced() {
        let route = ingress_route_for_tool("file_read");
        let err = authorize_client_ingress_route_with_nexus_inner(
            "tool:file_read",
            route,
            false,
            Some(ModuleLifecycleState::Quiesced),
        )
        .expect_err("quiesced blocked");
        assert!(err.contains("lease_source_not_accepting_new_leases"));
    }

    #[test]
    fn ingress_route_descriptor_maps_batch_query_to_context_stacks() {
        let route = ingress_route_for_tool("batch_query");
        assert_eq!(route.target, "context_stacks");
        assert_eq!(route.schema_id, "client_ingress.tool.retrieval");
        assert_eq!(route.verb, "invoke");
    }

    #[test]
    fn ingress_route_descriptor_maps_file_read_many_to_context_stacks() {
        let route = ingress_route_for_tool("file_read_many");
        assert_eq!(route.target, "context_stacks");
        assert_eq!(route.schema_id, "client_ingress.tool.retrieval");
        assert_eq!(route.verb, "invoke");
    }

    #[test]
    fn ingress_nexus_authorization_can_bypass_batch_query_in_relaxed_test_mode() {
        std::env::set_var("INFRING_WEB_TOOLING_RELAXED_TEST_MODE", "1");
        let out = authorize_ingress_tool_call_with_nexus("batch_query")
            .expect("authorize")
            .expect("connection");
        std::env::remove_var("INFRING_WEB_TOOLING_RELAXED_TEST_MODE");
        assert_eq!(
            out.get("policy_bypass").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(
            out.get("bypass_reason").and_then(Value::as_str),
            Some("web_tooling_relaxed_test_mode")
        );
    }
}
