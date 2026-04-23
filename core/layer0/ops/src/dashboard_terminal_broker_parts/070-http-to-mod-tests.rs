
pub fn handle_http(root: &Path, method: &str, path: &str, body: &[u8]) -> Option<Value> {
    if method == "GET" && path == "/api/terminal/sessions" {
        return Some(sessions_payload(root));
    }
    if method == "POST" && path == "/api/terminal/sessions" {
        return Some(create_session(root, &parse_json(body)));
    }
    if method == "POST" && path == "/api/terminal/queue" {
        return Some(exec_command(root, &parse_json(body)));
    }
    if method == "DELETE" && path.starts_with("/api/terminal/sessions/") {
        let sid = path.trim_start_matches("/api/terminal/sessions/");
        return Some(close_session(root, sid));
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_session_create_and_list() {
        let root = tempfile::tempdir().expect("tempdir");
        let created = create_session(root.path(), &json!({"id":"term-a"}));
        assert_eq!(created.get("ok").and_then(Value::as_bool), Some(true));
        let rows = sessions_payload(root.path())
            .get("sessions")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(rows.len(), 1);
    }

    #[test]
    fn terminal_exec_returns_stdout() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = create_session(root.path(), &json!({"id":"term-a"}));
        let out = exec_command(
            root.path(),
            &json!({"session_id":"term-a","command":"printf 'hello'"}),
        );
        assert_eq!(out.get("exit_code").and_then(Value::as_i64), Some(0));
        assert_eq!(out.get("stdout").and_then(Value::as_str), Some("hello"));
        assert_eq!(
            out.get("requested_command").and_then(Value::as_str),
            Some("printf 'hello'")
        );
        assert_eq!(
            out.get("executed_command").and_then(Value::as_str),
            Some("printf 'hello'")
        );
        assert_eq!(
            out.get("command_translated").and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            out.pointer("/nexus_connection/delivery/allowed")
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn terminal_exec_blocks_cwd_escape() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = create_session(root.path(), &json!({"id":"term-a"}));
        let out = exec_command(
            root.path(),
            &json!({"session_id":"term-a","command":"pwd","cwd":"/"}),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("cwd_outside_workspace")
        );
    }

    #[test]
    fn terminal_exec_pre_tool_gate_blocks_denied_command() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = create_session(root.path(), &json!({"id":"term-a"}));
        let out = exec_command(
            root.path(),
            &json!({"session_id":"term-a","command":"git reset --hard HEAD"}),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(out.get("blocked").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            Some("permission_denied_by_policy")
        );
        assert_eq!(out.get("exit_code").and_then(Value::as_i64), Some(126));
        assert_eq!(
            out.pointer("/permission_gate/verdict")
                .and_then(Value::as_str),
            Some("deny")
        );
    }

    #[test]
    fn terminal_exec_post_tool_filter_suppresses_ack_placeholder() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = create_session(root.path(), &json!({"id":"term-a"}));
        let out = exec_command(
            root.path(),
            &json!({"session_id":"term-a","command":"printf 'Web search completed.'"}),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("stdout").and_then(Value::as_str).unwrap_or(""), "");
        assert_eq!(
            out.get("low_signal_output").and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn terminal_exec_accepts_workspace_virtual_cwd_alias() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = create_session(root.path(), &json!({"id":"term-a", "cwd": "/workspace"}));
        let out = exec_command(
            root.path(),
            &json!({"session_id":"term-a","command":"pwd","cwd":"/workspace"}),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("exit_code").and_then(Value::as_i64), Some(0));
        assert_eq!(
            out.get("error").and_then(Value::as_str),
            None,
            "workspace alias should not be rejected as outside the workspace root"
        );
    }

    #[test]
    fn command_router_translates_diagnostic_surface() {
        let out = resolve_operator_command(
            "infring-ops diagnostic full-scan --priority=critical --output=telemetry-now",
        )
        .expect("translation");
        assert!(out.translated);
        assert_eq!(
            out.resolved_command,
            "infring-ops daemon-control diagnostics && infring-ops status --dashboard"
        );
        assert_eq!(
            out.translation_reason,
            "translated_unsupported_diagnostic_surface_to_daemon_diagnostics"
        );
    }

    #[test]
    fn command_router_translates_queue_optimize_aggressive() {
        let out = resolve_operator_command(
            "infring-ops queue optimize --strategy=aggressive --clean-orphaned=true",
        )
        .expect("translation");
        assert!(out.translated);
        assert_eq!(
            out.resolved_command,
            "infring-ops attention-queue compact --retain=128 && infring-ops attention-queue status"
        );
    }

    #[test]
    fn command_router_translates_infring_alias_to_core_binary() {
        let out = resolve_operator_command("infring daemon ping").expect("translation");
        assert!(out.translated);
        assert_eq!(out.resolved_command, "infring-ops daemon ping");
        assert_eq!(
            out.translation_reason,
            "translated_infring_cli_alias_to_infring_ops"
        );
    }

    #[test]
    fn command_router_translates_infring_help_surface_to_usage() {
        let out = resolve_operator_command("infring --help").expect("translation");
        assert!(out.translated);
        assert_eq!(
            out.resolved_command,
            "infring-ops command-list-kernel --mode=help"
        );
        assert_eq!(
            out.translation_reason,
            "translated_infring_help_surface_to_command_list_help"
        );
    }

    #[test]
    fn truncate_output_preserves_head_and_tail_context() {
        let text = format!(
            "head-marker:{}:{}tail-marker",
            "x".repeat(OUTPUT_MAX_BYTES),
            "y".repeat(OUTPUT_MAX_BYTES)
        );
        let out = truncate_output(&text);
        assert!(out.contains("head-marker"));
        assert!(out.contains("tail-marker"));
        assert!(out.contains("... (output truncated) ..."));
        assert!(out.as_bytes().len() <= OUTPUT_MAX_BYTES);
    }

    #[test]
    fn truncate_output_handles_utf8_boundaries() {
        let text = format!("前置{}后置", "界".repeat(OUTPUT_MAX_BYTES));
        let out = truncate_output(&text);
        assert!(out.contains("后置"));
        assert!(out.contains("... (output truncated) ..."));
        assert!(out.as_bytes().len() <= OUTPUT_MAX_BYTES);
    }
}
