fn finalize_agent_scope_tool_payload(
    root: &Path,
    agent_id: &str,
    tool_name: &str,
    tool_input: &Value,
    payload: &mut Value,
    nexus_connection: Option<Value>,
) {
    crate::dashboard_tool_turn_loop::annotate_tool_payload_tracking(
        root, agent_id, tool_name, payload,
    );
    let audit_receipt =
        append_tool_decision_audit(root, agent_id, tool_name, tool_input, payload, "none");
    if let Some(obj) = payload.as_object_mut() {
        obj.insert(
            "recovery_strategy".to_string(),
            Value::String("none".to_string()),
        );
        obj.insert("recovery_attempts".to_string(), json!(0));
        obj.insert(
            "decision_audit_receipt".to_string(),
            Value::String(audit_receipt),
        );
        if let Some(meta) = nexus_connection {
            obj.insert("nexus_connection".to_string(), meta);
        }
    }
}

const FILE_READ_ROUTING_CONTRACT_VERSION: &str = "v1";

fn file_read_many_routing_hint(
    ok_count: usize,
    text_count: usize,
    binary_count: usize,
    failed_count: usize,
    unclassified_count: usize,
    allow_binary: bool,
) -> &'static str {
    if ok_count == 0 {
        if binary_count > 0 && !allow_binary {
            return "binary_opt_in_required";
        }
        if failed_count > 0 {
            return "path_repair_required";
        }
        if unclassified_count > 0 && failed_count == 0 {
            return "path_repair_required";
        }
        return "no_readable_files";
    }
    if text_count > 0 && binary_count > 0 {
        return "mixed_text_binary_follow_up";
    }
    if text_count > 0 {
        if failed_count > 0 || unclassified_count > 0 {
            return "text_primary_partial";
        }
        return "text_primary";
    }
    if binary_count > 0 {
        if allow_binary {
            return "binary_primary";
        }
        return "binary_opt_in_required";
    }
    "no_readable_files"
}

fn file_read_single_routing_hint(binary: bool, truncated: bool, allow_binary: bool) -> &'static str {
    if binary {
        if allow_binary {
            return "binary_primary";
        }
        return "binary_opt_in_required";
    }
    if truncated {
        return "text_truncated";
    }
    "text_primary"
}

fn file_read_single_routing_reason(hint: &str, allow_binary: bool) -> &'static str {
    match hint {
        "path_required" => "path_missing",
        "path_outside_workspace" => "path_outside_workspace",
        "file_not_found" => "invalid_or_unreachable_path",
        "binary_opt_in_required" if !allow_binary => "binary_opt_in_missing",
        "binary_opt_in_required" => "binary_opt_in_required",
        "text_truncated" => "text_truncated_by_max_bytes",
        "text_primary" | "binary_primary" => "none",
        _ => "unknown",
    }
}

fn file_read_single_routing_retryable(hint: &str) -> bool {
    !matches!(hint, "text_primary" | "binary_primary")
}

fn file_read_single_recovery_action(hint: &str) -> (&'static str, &'static str) {
    match hint {
        "path_required" => (
            "provide_workspace_relative_path",
            "Provide a workspace-relative file path in path or file_path.",
        ),
        "path_outside_workspace" => (
            "use_workspace_relative_path",
            "The requested path was outside the workspace; retry with a workspace-relative path.",
        ),
        "file_not_found" => (
            "verify_path_or_export_folder_tree",
            "The requested file was not found; verify the path or export the folder tree first.",
        ),
        "binary_opt_in_required" => (
            "retry_with_allow_binary_true",
            "Enable allow_binary=true to receive base64 payload for binary files.",
        ),
        "text_truncated" => (
            "increase_max_bytes_or_full_true",
            "Content was truncated by max_bytes; raise max_bytes or set full=true.",
        ),
        _ => ("none", "No additional recovery action required."),
    }
}

fn file_read_recovery_retryable(action: &str) -> bool {
    action != "none"
}

fn file_read_recovery_class(action: &str) -> &'static str {
    match action {
        "none" => "none",
        "provide_workspace_relative_path" | "use_workspace_relative_path"
        | "verify_path_or_export_folder_tree" | "repair_paths_or_export_folder"
        | "repair_failed_paths" | "repair_paths_or_permissions" => "path_repair",
        "retry_with_allow_binary_true" | "review_text_then_opt_in_binary" => "binary_opt_in",
        "increase_max_bytes_or_full_true" => "content_expansion",
        "provide_paths_array" => "input_repair",
        "repair_nexus_ingress_policy" => "policy_repair",
        _ => "other",
    }
}

fn file_read_recovery_next_command(action: &str) -> &'static str {
    match action {
        "none" => "none",
        "provide_workspace_relative_path" | "use_workspace_relative_path"
        | "retry_with_allow_binary_true" | "increase_max_bytes_or_full_true" => "file_read",
        "provide_paths_array" | "review_text_then_opt_in_binary" | "repair_failed_paths" => {
            "file_read_many"
        }
        "verify_path_or_export_folder_tree" | "repair_paths_or_export_folder"
        | "repair_paths_or_permissions" => "folder_export",
        "repair_nexus_ingress_policy" => "nexus_policy_update",
        _ => "none",
    }
}

fn file_read_recovery_requires_user_input(action: &str) -> bool {
    matches!(
        action,
        "provide_workspace_relative_path"
            | "use_workspace_relative_path"
            | "verify_path_or_export_folder_tree"
            | "repair_paths_or_export_folder"
            | "repair_failed_paths"
            | "repair_paths_or_permissions"
            | "provide_paths_array"
            | "repair_nexus_ingress_policy"
    )
}

fn file_read_routing_severity(hint: &str) -> &'static str {
    match hint {
        "text_primary" | "binary_primary" => "ok",
        "text_truncated" | "mixed_text_binary_follow_up" | "text_primary_partial" => "degraded",
        _ => "error",
    }
}

fn file_read_routing_class(hint: &str) -> &'static str {
    match hint {
        "path_required" | "path_outside_workspace" | "file_not_found" | "paths_required"
        | "path_repair_required" => "path_repair",
        "binary_opt_in_required" | "binary_primary" | "mixed_text_binary_follow_up" => {
            "binary_lane"
        }
        "text_truncated" | "text_primary_partial" => "content_partial",
        "nexus_delivery_denied" => "policy_denied",
        "text_primary" => "none",
        "no_readable_files" => "no_data",
        _ => "other",
    }
}

fn file_read_routing_requires_binary_opt_in(hint: &str, allow_binary: bool) -> bool {
    match hint {
        "binary_opt_in_required" | "mixed_text_binary_follow_up" => !allow_binary,
        _ => false,
    }
}

fn file_read_routing_requires_path_repair(hint: &str) -> bool {
    matches!(
        hint,
        "path_required"
            | "path_outside_workspace"
            | "file_not_found"
            | "paths_required"
            | "path_repair_required"
    )
}

fn file_read_routing_requires_content_expansion(hint: &str, truncated_count: usize) -> bool {
    if hint == "text_truncated" {
        return true;
    }
    truncated_count > 0
        && matches!(
            hint,
            "text_primary" | "text_primary_partial" | "mixed_text_binary_follow_up"
        )
}

fn file_read_routing_requires_mixed_follow_up(hint: &str) -> bool {
    hint == "mixed_text_binary_follow_up"
}

fn file_read_routing_requires_partial_replay(partial: bool, hint: &str) -> bool {
    partial && matches!(hint, "text_primary_partial" | "mixed_text_binary_follow_up")
}

fn file_read_routing_requires_policy_repair(hint: &str) -> bool {
    hint == "nexus_delivery_denied"
}

fn file_read_routing_requires_follow_up(
    requires_policy_repair: bool,
    requires_path_repair: bool,
    requires_binary_opt_in: bool,
    requires_content_expansion: bool,
    requires_mixed_follow_up: bool,
    requires_partial_replay: bool,
) -> bool {
    requires_policy_repair
        || requires_path_repair
        || requires_binary_opt_in
        || requires_content_expansion
        || requires_mixed_follow_up
        || requires_partial_replay
}

fn file_read_routing_follow_up_can_auto_retry(
    requires_follow_up: bool,
    recovery_retryable: bool,
    recovery_requires_user_input: bool,
) -> bool {
    requires_follow_up && recovery_retryable && !recovery_requires_user_input
}

fn file_read_routing_follow_up_mode(
    requires_follow_up: bool,
    recovery_retryable: bool,
    recovery_requires_user_input: bool,
) -> &'static str {
    if !requires_follow_up {
        return "none";
    }
    if recovery_retryable && !recovery_requires_user_input {
        return "auto_retry";
    }
    "user_input"
}

fn file_read_routing_follow_up_task(
    requires_policy_repair: bool,
    requires_path_repair: bool,
    requires_binary_opt_in: bool,
    requires_content_expansion: bool,
    requires_mixed_follow_up: bool,
    requires_partial_replay: bool,
) -> &'static str {
    if requires_policy_repair {
        return "policy_repair";
    }
    if requires_path_repair {
        return "path_repair";
    }
    if requires_binary_opt_in {
        return "binary_opt_in";
    }
    if requires_content_expansion {
        return "content_expansion";
    }
    if requires_mixed_follow_up {
        return "mixed_follow_up";
    }
    if requires_partial_replay {
        return "partial_replay";
    }
    "none"
}

fn file_read_routing_follow_up_priority(
    follow_up_task: &str,
    requires_follow_up: bool,
) -> &'static str {
    if !requires_follow_up {
        return "none";
    }
    match follow_up_task {
        "policy_repair" => "critical",
        "path_repair" | "binary_opt_in" => "high",
        "mixed_follow_up" | "partial_replay" => "medium",
        "content_expansion" => "low",
        _ => "low",
    }
}

fn file_read_routing_follow_up_blocking(
    follow_up_priority: &str,
    requires_follow_up: bool,
) -> bool {
    requires_follow_up && matches!(follow_up_priority, "critical" | "high")
}

fn file_read_routing_follow_up_bucket(
    requires_follow_up: bool,
    follow_up_blocking: bool,
    follow_up_mode: &str,
) -> &'static str {
    if !requires_follow_up {
        return "none";
    }
    if follow_up_blocking {
        return "blocking";
    }
    if follow_up_mode == "auto_retry" {
        return "auto_retry";
    }
    "advisory"
}

fn file_read_routing_follow_up_sla_seconds(
    requires_follow_up: bool,
    follow_up_priority: &str,
) -> u64 {
    if !requires_follow_up {
        return 0;
    }
    match follow_up_priority {
        "critical" => 0,
        "high" => 30,
        "medium" => 120,
        "low" => 300,
        _ => 300,
    }
}

fn file_read_routing_follow_up_requires_immediate_action(
    requires_follow_up: bool,
    follow_up_sla_seconds: u64,
) -> bool {
    requires_follow_up && follow_up_sla_seconds <= 30
}

fn file_read_routing_follow_up_owner(
    requires_follow_up: bool,
    follow_up_mode: &str,
) -> &'static str {
    if !requires_follow_up {
        return "none";
    }
    if follow_up_mode == "auto_retry" {
        return "system";
    }
    "operator"
}

fn file_read_routing_follow_up_requires_operator_input(
    requires_follow_up: bool,
    follow_up_owner: &str,
) -> bool {
    requires_follow_up && follow_up_owner == "operator"
}

fn file_read_routing_follow_up_requires_confirmation(
    requires_follow_up: bool,
    follow_up_owner: &str,
    follow_up_mode: &str,
) -> bool {
    requires_follow_up && follow_up_owner == "operator" && follow_up_mode == "user_input"
}

fn file_read_routing_follow_up_action_kind(
    requires_follow_up: bool,
    follow_up_mode: &str,
) -> &'static str {
    if !requires_follow_up {
        return "none";
    }
    if follow_up_mode == "auto_retry" {
        return "retry";
    }
    "repair"
}

fn file_read_routing_follow_up_requires_path_input(
    requires_follow_up: bool,
    requires_path_repair: bool,
) -> bool {
    requires_follow_up && requires_path_repair
}

fn file_read_routing_follow_up_can_defer(
    requires_follow_up: bool,
    follow_up_requires_immediate_action: bool,
) -> bool {
    requires_follow_up && !follow_up_requires_immediate_action
}

fn file_read_routing_follow_up_requires_binary_opt_in_input(
    requires_follow_up: bool,
    requires_binary_opt_in: bool,
) -> bool {
    requires_follow_up && requires_binary_opt_in
}

fn file_read_routing_follow_up_requires_policy_repair_input(
    requires_follow_up: bool,
    requires_policy_repair: bool,
) -> bool {
    requires_follow_up && requires_policy_repair
}

fn file_read_routing_follow_up_requires_content_expansion_input(
    requires_follow_up: bool,
    requires_content_expansion: bool,
) -> bool {
    requires_follow_up && requires_content_expansion
}

fn file_read_routing_follow_up_requires_mixed_follow_up_input(
    requires_follow_up: bool,
    requires_mixed_follow_up: bool,
) -> bool {
    requires_follow_up && requires_mixed_follow_up
}

fn file_read_routing_follow_up_requires_partial_replay_input(
    requires_follow_up: bool,
    requires_partial_replay: bool,
) -> bool {
    requires_follow_up && requires_partial_replay
}

fn file_read_routing_follow_up_user_text_input_kind(
    requires_follow_up: bool,
    follow_up_owner: &str,
    follow_up_requires_path_input: bool,
    follow_up_requires_policy_repair_input: bool,
    follow_up_requires_binary_opt_in_input: bool,
    follow_up_requires_content_expansion_input: bool,
    follow_up_requires_mixed_follow_up_input: bool,
    follow_up_requires_partial_replay_input: bool,
) -> &'static str {
    if !requires_follow_up || follow_up_owner != "operator" {
        return "none";
    }
    if follow_up_requires_path_input {
        return "path_input";
    }
    if follow_up_requires_policy_repair_input {
        return "policy_repair_input";
    }
    if follow_up_requires_binary_opt_in_input {
        return "binary_opt_in_input";
    }
    if follow_up_requires_content_expansion_input {
        return "content_expansion_input";
    }
    if follow_up_requires_mixed_follow_up_input {
        return "mixed_follow_up_input";
    }
    if follow_up_requires_partial_replay_input {
        return "partial_replay_input";
    }
    "user_text"
}

fn file_read_routing_follow_up_requires_user_text_input(
    requires_follow_up: bool,
    follow_up_owner: &str,
    follow_up_requires_path_input: bool,
    follow_up_requires_policy_repair_input: bool,
    follow_up_requires_binary_opt_in_input: bool,
    follow_up_requires_content_expansion_input: bool,
    follow_up_requires_mixed_follow_up_input: bool,
    follow_up_requires_partial_replay_input: bool,
) -> bool {
    file_read_routing_follow_up_user_text_input_kind(
        requires_follow_up,
        follow_up_owner,
        follow_up_requires_path_input,
        follow_up_requires_policy_repair_input,
        follow_up_requires_binary_opt_in_input,
        follow_up_requires_content_expansion_input,
        follow_up_requires_mixed_follow_up_input,
        follow_up_requires_partial_replay_input,
    ) != "none"
}

fn file_read_routing_blocker(
    hint: &str,
    requires_path_repair: bool,
    requires_binary_opt_in: bool,
    requires_content_expansion: bool,
    requires_partial_replay: bool,
    requires_mixed_follow_up: bool,
) -> &'static str {
    if hint == "nexus_delivery_denied" {
        return "policy_repair";
    }
    if requires_path_repair {
        return "path_repair";
    }
    if requires_binary_opt_in {
        return "binary_opt_in";
    }
    if requires_content_expansion {
        return "content_expansion";
    }
    if requires_partial_replay {
        return "partial_replay";
    }
    if requires_mixed_follow_up {
        return "mixed_follow_up";
    }
    "none"
}

fn file_read_many_recovery_action(
    hint: &str,
    partial: bool,
    allow_binary: bool,
) -> (&'static str, &'static str) {
    match hint {
        "binary_opt_in_required" => (
            "retry_with_allow_binary_true",
            "Enable allow_binary=true to read binary files via base64 payload lane.",
        ),
        "path_repair_required" => (
            "repair_paths_or_export_folder",
            "Use workspace-relative paths or run folder export to discover valid files.",
        ),
        "no_readable_files" => (
            "repair_paths_or_permissions",
            "No readable files were returned; verify paths and workspace access.",
        ),
        "mixed_text_binary_follow_up" if !allow_binary => (
            "review_text_then_opt_in_binary",
            "Text files were read; retry with allow_binary=true for binary entries.",
        ),
        "text_primary_partial" if partial => (
            "repair_failed_paths",
            "Some files succeeded; repair failed/unclassified paths and retry.",
        ),
        _ => ("none", "No additional recovery action required."),
    }
}

fn file_read_many_routing_reason(hint: &str, allow_binary: bool) -> &'static str {
    match hint {
        "paths_required" => "paths_missing",
        "binary_opt_in_required" if !allow_binary => "binary_opt_in_missing",
        "binary_opt_in_required" => "binary_opt_in_required",
        "path_repair_required" => "invalid_or_unreachable_paths",
        "mixed_text_binary_follow_up" if !allow_binary => "binary_entries_pending_opt_in",
        "mixed_text_binary_follow_up" => "mixed_text_binary_lanes",
        "text_primary_partial" => "partial_batch_failures",
        "text_primary" | "binary_primary" => "none",
        "no_readable_files" => "no_readable_files",
        _ => "unknown",
    }
}

fn file_read_many_routing_retryable(hint: &str) -> bool {
    !matches!(hint, "text_primary" | "binary_primary")
}

fn handle_agent_scope_file_read_routes(
    root: &Path,
    method: &str,
    segments: &[String],
    body: &[u8],
    agent_id: &str,
    existing: &Option<Value>,
) -> Option<CompatApiResponse> {
    if method == "POST" && segments.len() == 2 && segments[0] == "file" && segments[1] == "read" {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let requested_path = clean_text(
            request
                .get("path")
                .and_then(Value::as_str)
                .or_else(|| request.get("file_path").and_then(Value::as_str))
                .unwrap_or(""),
            4000,
        );
        if requested_path.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({
                    "ok": false,
                    "error": "path_required",
                    "routing": {
                        "hint": "path_required",
                        "allow_binary": false,
                        "partial": false,
                        "binary_opt_in_blocked_count": 0,
                        "requires_policy_repair": false,
                        "requires_follow_up": true,
                        "follow_up_can_auto_retry": false,
                        "follow_up_mode": "user_input",
                        "follow_up_task": "path_repair",
                        "follow_up_priority": "high",
                        "follow_up_blocking": true,
                        "follow_up_bucket": "blocking",
                        "follow_up_sla_seconds": 30,
                        "follow_up_requires_immediate_action": true,
                        "follow_up_owner": "operator",
                        "follow_up_requires_operator_input": true,
                        "follow_up_requires_confirmation": true,
                        "follow_up_action_kind": "repair",
                        "follow_up_requires_path_input": true,
                        "follow_up_can_defer": false,
                        "follow_up_requires_binary_opt_in_input": false,
                        "follow_up_requires_policy_repair_input": false,
                        "follow_up_requires_content_expansion_input": false,
                        "follow_up_requires_mixed_follow_up_input": false,
                        "follow_up_requires_partial_replay_input": false,
                        "follow_up_requires_user_text_input": true,
                        "follow_up_user_text_input_kind": "path_input",
                        "requires_binary_opt_in": false,
                        "requires_path_repair": true,
                        "requires_content_expansion": false,
                        "requires_mixed_follow_up": false,
                        "requires_partial_replay": false,
                        "blocker": "path_repair",
                        "class": "path_repair",
                        "reason": "path_missing",
                        "retryable": true,
                        "severity": "error",
                        "contract_version": FILE_READ_ROUTING_CONTRACT_VERSION
                    },
                    "recovery": {
                        "action": "provide_workspace_relative_path",
                        "message": "Provide a workspace-relative file path in path or file_path.",
                        "retryable": true,
                        "class": "path_repair",
                        "next_command": "file_read",
                        "requires_user_input": true,
                        "example": "notes/plan.txt"
                    }
                }),
            });
        }
        let nexus_connection =
            match crate::dashboard_tool_turn_loop::authorize_ingress_tool_call_with_nexus(
                "file_read",
            ) {
                Ok(meta) => meta,
                Err(err) => {
                    return Some(CompatApiResponse {
                        status: 403,
                        payload: json!({
                            "ok": false,
                            "error": "file_read_nexus_delivery_denied",
                            "message": "File read blocked by hierarchical nexus ingress policy.",
                            "nexus_error": clean_text(&err, 240),
                            "routing": {
                                "hint": "nexus_delivery_denied",
                                "allow_binary": false,
                                "partial": false,
                                "binary_opt_in_blocked_count": 0,
                                "requires_policy_repair": true,
                                "requires_follow_up": true,
                                "follow_up_can_auto_retry": false,
                                "follow_up_mode": "user_input",
                                "follow_up_task": "policy_repair",
                                "follow_up_priority": "critical",
                                "follow_up_blocking": true,
                                "follow_up_bucket": "blocking",
                                "follow_up_sla_seconds": 0,
                                "follow_up_requires_immediate_action": true,
                                "follow_up_owner": "operator",
                                "follow_up_requires_operator_input": true,
                                "follow_up_requires_confirmation": true,
                                "follow_up_action_kind": "repair",
                                "follow_up_requires_path_input": false,
                                "follow_up_can_defer": false,
                                "follow_up_requires_binary_opt_in_input": false,
                                "follow_up_requires_policy_repair_input": true,
                                "follow_up_requires_content_expansion_input": false,
                                "follow_up_requires_mixed_follow_up_input": false,
                                "follow_up_requires_partial_replay_input": false,
                                "follow_up_requires_user_text_input": true,
                                "follow_up_user_text_input_kind": "policy_repair_input",
                                "requires_binary_opt_in": false,
                                "requires_path_repair": false,
                                "requires_content_expansion": false,
                                "requires_mixed_follow_up": false,
                                "requires_partial_replay": false,
                                "blocker": "policy_repair",
                                "class": "policy_denied",
                                "reason": "ingress_policy_denied",
                                "retryable": true,
                                "severity": "error",
                                "contract_version": FILE_READ_ROUTING_CONTRACT_VERSION
                            },
                            "recovery": {
                                "action": "repair_nexus_ingress_policy",
                                "message": "Adjust nexus ingress policy or tool permissions, then retry file_read.",
                                "retryable": true,
                                "class": "policy_repair",
                                "next_command": "nexus_policy_update",
                                "requires_user_input": true
                            }
                        }),
                    })
                }
            };
        let workspace_base = workspace_base_for_agent(root, existing.as_ref());
        let target = resolve_workspace_path(&workspace_base, &requested_path);
        let Some(target_path) = target else {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({
                    "ok": false,
                    "error": "path_outside_workspace",
                    "path": requested_path,
                    "routing": {
                        "hint": "path_outside_workspace",
                        "allow_binary": false,
                        "partial": false,
                        "binary_opt_in_blocked_count": 0,
                        "requires_policy_repair": false,
                        "requires_follow_up": true,
                        "follow_up_can_auto_retry": false,
                        "follow_up_mode": "user_input",
                        "follow_up_task": "path_repair",
                        "follow_up_priority": "high",
                        "follow_up_blocking": true,
                        "follow_up_bucket": "blocking",
                        "follow_up_sla_seconds": 30,
                        "follow_up_requires_immediate_action": true,
                        "follow_up_owner": "operator",
                        "follow_up_requires_operator_input": true,
                        "follow_up_requires_confirmation": true,
                        "follow_up_action_kind": "repair",
                        "follow_up_requires_path_input": true,
                        "follow_up_can_defer": false,
                        "follow_up_requires_binary_opt_in_input": false,
                        "follow_up_requires_policy_repair_input": false,
                        "follow_up_requires_content_expansion_input": false,
                        "follow_up_requires_mixed_follow_up_input": false,
                        "follow_up_requires_partial_replay_input": false,
                        "follow_up_requires_user_text_input": true,
                        "follow_up_user_text_input_kind": "path_input",
                        "requires_binary_opt_in": false,
                        "requires_path_repair": true,
                        "requires_content_expansion": false,
                        "requires_mixed_follow_up": false,
                        "requires_partial_replay": false,
                        "blocker": "path_repair",
                        "class": "path_repair",
                        "reason": "path_outside_workspace",
                        "retryable": true,
                        "severity": "error",
                        "contract_version": FILE_READ_ROUTING_CONTRACT_VERSION
                    },
                    "recovery": {
                        "action": "use_workspace_relative_path",
                        "message": "The requested path was outside the workspace; retry with a workspace-relative path.",
                        "retryable": true,
                        "class": "path_repair",
                        "next_command": "file_read",
                        "requires_user_input": true
                    }
                }),
            });
        };
        if !target_path.is_file() {
            return Some(CompatApiResponse {
                status: 404,
                payload: json!({
                    "ok": false,
                    "error": "file_not_found",
                    "path": target_path.to_string_lossy().to_string(),
                    "routing": {
                        "hint": "file_not_found",
                        "allow_binary": false,
                        "partial": false,
                        "binary_opt_in_blocked_count": 0,
                        "requires_policy_repair": false,
                        "requires_follow_up": true,
                        "follow_up_can_auto_retry": false,
                        "follow_up_mode": "user_input",
                        "follow_up_task": "path_repair",
                        "follow_up_priority": "high",
                        "follow_up_blocking": true,
                        "follow_up_bucket": "blocking",
                        "follow_up_sla_seconds": 30,
                        "follow_up_requires_immediate_action": true,
                        "follow_up_owner": "operator",
                        "follow_up_requires_operator_input": true,
                        "follow_up_requires_confirmation": true,
                        "follow_up_action_kind": "repair",
                        "follow_up_requires_path_input": true,
                        "follow_up_can_defer": false,
                        "follow_up_requires_binary_opt_in_input": false,
                        "follow_up_requires_policy_repair_input": false,
                        "follow_up_requires_content_expansion_input": false,
                        "follow_up_requires_mixed_follow_up_input": false,
                        "follow_up_requires_partial_replay_input": false,
                        "follow_up_requires_user_text_input": true,
                        "follow_up_user_text_input_kind": "path_input",
                        "requires_binary_opt_in": false,
                        "requires_path_repair": true,
                        "requires_content_expansion": false,
                        "requires_mixed_follow_up": false,
                        "requires_partial_replay": false,
                        "blocker": "path_repair",
                        "class": "path_repair",
                        "reason": "invalid_or_unreachable_path",
                        "retryable": true,
                        "severity": "error",
                        "contract_version": FILE_READ_ROUTING_CONTRACT_VERSION
                    },
                    "recovery": {
                        "action": "verify_path_or_export_folder_tree",
                        "message": "The requested file was not found; verify the path or export the folder tree first.",
                        "retryable": true,
                        "class": "path_repair",
                        "next_command": "folder_export",
                        "requires_user_input": true
                    },
                    "file": {"ok": false, "path": target_path.to_string_lossy().to_string()}
                }),
            });
        }
        let bytes = fs::read(&target_path).unwrap_or_default();
        let full = request
            .get("full")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let allow_binary = request
            .get("allow_binary")
            .or_else(|| request.get("binary"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let max_bytes = if full {
            bytes.len().max(1)
        } else {
            request
                .get("max_bytes")
                .and_then(Value::as_u64)
                .unwrap_or((256 * 1024) as u64)
                .clamp(1, (8 * 1024 * 1024) as u64) as usize
        };
        let binary = bytes_look_binary(&bytes);
        let content_type = guess_mime_type_for_file(&target_path, &bytes);
        if binary && !allow_binary {
            return Some(CompatApiResponse {
                status: 415,
                payload: json!({
                    "ok": false,
                    "error": "binary_file_requires_opt_in",
                    "routing": {
                        "hint": "binary_opt_in_required",
                        "allow_binary": allow_binary,
                        "partial": false,
                        "binary_opt_in_blocked_count": 1,
                        "requires_policy_repair": false,
                        "requires_follow_up": true,
                        "follow_up_can_auto_retry": true,
                        "follow_up_mode": "auto_retry",
                        "follow_up_task": "binary_opt_in",
                        "follow_up_priority": "high",
                        "follow_up_blocking": true,
                        "follow_up_bucket": "blocking",
                        "follow_up_sla_seconds": 30,
                        "follow_up_requires_immediate_action": true,
                        "follow_up_owner": "system",
                        "follow_up_requires_operator_input": false,
                        "follow_up_requires_confirmation": false,
                        "follow_up_action_kind": "retry",
                        "follow_up_requires_path_input": false,
                        "follow_up_can_defer": false,
                        "follow_up_requires_binary_opt_in_input": true,
                        "follow_up_requires_policy_repair_input": false,
                        "follow_up_requires_content_expansion_input": false,
                        "follow_up_requires_mixed_follow_up_input": false,
                        "follow_up_requires_partial_replay_input": false,
                        "follow_up_requires_user_text_input": false,
                        "follow_up_user_text_input_kind": "none",
                        "requires_binary_opt_in": true,
                        "requires_path_repair": false,
                        "requires_content_expansion": false,
                        "requires_mixed_follow_up": false,
                        "requires_partial_replay": false,
                        "blocker": "binary_opt_in",
                        "class": "binary_lane",
                        "reason": "binary_opt_in_missing",
                        "retryable": true,
                        "severity": "error",
                        "contract_version": FILE_READ_ROUTING_CONTRACT_VERSION
                    },
                    "recovery": {
                        "action": "retry_with_allow_binary_true",
                        "message": "Enable allow_binary=true to receive base64 payload for binary files.",
                        "retryable": true,
                        "class": "binary_opt_in",
                        "next_command": "file_read",
                        "requires_user_input": false
                    },
                    "file": {
                        "ok": false,
                        "path": target_path.to_string_lossy().to_string(),
                        "bytes": bytes.len(),
                        "binary": true,
                        "content_type": content_type,
                        "file_name": clean_text(
                            target_path.file_name().and_then(|v| v.to_str()).unwrap_or("download.bin"),
                            180
                        )
                    }
                }),
            });
        }
        let (content, truncated) = if binary {
            (String::new(), bytes.len() > max_bytes)
        } else {
            truncate_utf8_lossy(&bytes, max_bytes)
        };
        let content_base64 = if binary {
            use base64::engine::general_purpose::STANDARD;
            use base64::Engine;
            let slice_end = bytes.len().min(max_bytes.max(1));
            STANDARD.encode(&bytes[..slice_end])
        } else {
            String::new()
        };
        let download_url = if bytes.len() <= (2 * 1024 * 1024) {
            data_url_from_bytes(&bytes, &content_type)
        } else {
            String::new()
        };
        let file_name = clean_text(
            target_path
                .file_name()
                .and_then(|v| v.to_str())
                .unwrap_or("download.txt"),
            180,
        );
        let routing_hint = file_read_single_routing_hint(binary, truncated, allow_binary);
        let routing_class = file_read_routing_class(routing_hint);
        let routing_reason = file_read_single_routing_reason(routing_hint, allow_binary);
        let routing_retryable = file_read_single_routing_retryable(routing_hint);
        let routing_severity = file_read_routing_severity(routing_hint);
        let routing_requires_binary_opt_in =
            file_read_routing_requires_binary_opt_in(routing_hint, allow_binary);
        let routing_requires_path_repair =
            file_read_routing_requires_path_repair(routing_hint);
        let routing_requires_content_expansion =
            file_read_routing_requires_content_expansion(routing_hint, if truncated { 1 } else { 0 });
        let routing_requires_mixed_follow_up =
            file_read_routing_requires_mixed_follow_up(routing_hint);
        let routing_requires_partial_replay =
            file_read_routing_requires_partial_replay(false, routing_hint);
        let routing_requires_policy_repair =
            file_read_routing_requires_policy_repair(routing_hint);
        let routing_requires_follow_up = file_read_routing_requires_follow_up(
            routing_requires_policy_repair,
            routing_requires_path_repair,
            routing_requires_binary_opt_in,
            routing_requires_content_expansion,
            routing_requires_mixed_follow_up,
            routing_requires_partial_replay,
        );
        let routing_binary_opt_in_blocked_count: usize =
            if routing_requires_binary_opt_in { 1 } else { 0 };
        let routing_blocker = file_read_routing_blocker(
            routing_hint,
            routing_requires_path_repair,
            routing_requires_binary_opt_in,
            routing_requires_content_expansion,
            routing_requires_partial_replay,
            routing_requires_mixed_follow_up,
        );
        let (recovery_action, recovery_message) = file_read_single_recovery_action(routing_hint);
        let recovery_retryable = file_read_recovery_retryable(recovery_action);
        let recovery_class = file_read_recovery_class(recovery_action);
        let recovery_next_command = file_read_recovery_next_command(recovery_action);
        let recovery_requires_user_input =
            file_read_recovery_requires_user_input(recovery_action);
        let routing_follow_up_can_auto_retry = file_read_routing_follow_up_can_auto_retry(
            routing_requires_follow_up,
            recovery_retryable,
            recovery_requires_user_input,
        );
        let routing_follow_up_mode = file_read_routing_follow_up_mode(
            routing_requires_follow_up,
            recovery_retryable,
            recovery_requires_user_input,
        );
        let routing_follow_up_task = file_read_routing_follow_up_task(
            routing_requires_policy_repair,
            routing_requires_path_repair,
            routing_requires_binary_opt_in,
            routing_requires_content_expansion,
            routing_requires_mixed_follow_up,
            routing_requires_partial_replay,
        );
        let routing_follow_up_priority = file_read_routing_follow_up_priority(
            routing_follow_up_task,
            routing_requires_follow_up,
        );
        let routing_follow_up_blocking = file_read_routing_follow_up_blocking(
            routing_follow_up_priority,
            routing_requires_follow_up,
        );
        let routing_follow_up_bucket = file_read_routing_follow_up_bucket(
            routing_requires_follow_up,
            routing_follow_up_blocking,
            routing_follow_up_mode,
        );
        let routing_follow_up_sla_seconds = file_read_routing_follow_up_sla_seconds(
            routing_requires_follow_up,
            routing_follow_up_priority,
        );
        let routing_follow_up_requires_immediate_action =
            file_read_routing_follow_up_requires_immediate_action(
                routing_requires_follow_up,
                routing_follow_up_sla_seconds,
            );
        let routing_follow_up_owner = file_read_routing_follow_up_owner(
            routing_requires_follow_up,
            routing_follow_up_mode,
        );
        let routing_follow_up_requires_operator_input =
            file_read_routing_follow_up_requires_operator_input(
                routing_requires_follow_up,
                routing_follow_up_owner,
            );
        let routing_follow_up_requires_confirmation =
            file_read_routing_follow_up_requires_confirmation(
                routing_requires_follow_up,
                routing_follow_up_owner,
                routing_follow_up_mode,
            );
        let routing_follow_up_action_kind = file_read_routing_follow_up_action_kind(
            routing_requires_follow_up,
            routing_follow_up_mode,
        );
        let routing_follow_up_requires_path_input =
            file_read_routing_follow_up_requires_path_input(
                routing_requires_follow_up,
                routing_requires_path_repair,
            );
        let routing_follow_up_can_defer = file_read_routing_follow_up_can_defer(
            routing_requires_follow_up,
            routing_follow_up_requires_immediate_action,
        );
        let routing_follow_up_requires_binary_opt_in_input =
            file_read_routing_follow_up_requires_binary_opt_in_input(
                routing_requires_follow_up,
                routing_requires_binary_opt_in,
            );
        let routing_follow_up_requires_policy_repair_input =
            file_read_routing_follow_up_requires_policy_repair_input(
                routing_requires_follow_up,
                routing_requires_policy_repair,
            );
        let routing_follow_up_requires_content_expansion_input =
            file_read_routing_follow_up_requires_content_expansion_input(
                routing_requires_follow_up,
                routing_requires_content_expansion,
            );
        let routing_follow_up_requires_mixed_follow_up_input =
            file_read_routing_follow_up_requires_mixed_follow_up_input(
                routing_requires_follow_up,
                routing_requires_mixed_follow_up,
            );
        let routing_follow_up_requires_partial_replay_input =
            file_read_routing_follow_up_requires_partial_replay_input(
                routing_requires_follow_up,
                routing_requires_partial_replay,
            );
        let routing_follow_up_requires_user_text_input =
            file_read_routing_follow_up_requires_user_text_input(
                routing_requires_follow_up,
                routing_follow_up_owner,
                routing_follow_up_requires_path_input,
                routing_follow_up_requires_policy_repair_input,
                routing_follow_up_requires_binary_opt_in_input,
                routing_follow_up_requires_content_expansion_input,
                routing_follow_up_requires_mixed_follow_up_input,
                routing_follow_up_requires_partial_replay_input,
            );
        let routing_follow_up_user_text_input_kind =
            file_read_routing_follow_up_user_text_input_kind(
                routing_requires_follow_up,
                routing_follow_up_owner,
                routing_follow_up_requires_path_input,
                routing_follow_up_requires_policy_repair_input,
                routing_follow_up_requires_binary_opt_in_input,
                routing_follow_up_requires_content_expansion_input,
                routing_follow_up_requires_mixed_follow_up_input,
                routing_follow_up_requires_partial_replay_input,
            );
        let mut payload = json!({
            "ok": true,
            "file": {
                "ok": true,
                "path": target_path.to_string_lossy().to_string(),
                "content": content,
                "content_base64": content_base64,
                "truncated": truncated,
                "bytes": bytes.len(),
                "max_bytes": max_bytes,
                "full": full,
                "binary": binary,
                "allow_binary": allow_binary,
                "download_url": download_url,
                "file_name": file_name,
                "content_type": content_type
            },
            "routing": {
                "hint": routing_hint,
                "allow_binary": allow_binary,
                "partial": false,
                "binary_opt_in_blocked_count": routing_binary_opt_in_blocked_count,
                "requires_policy_repair": routing_requires_policy_repair,
                "requires_follow_up": routing_requires_follow_up,
                "follow_up_can_auto_retry": routing_follow_up_can_auto_retry,
                "follow_up_mode": routing_follow_up_mode,
                "follow_up_task": routing_follow_up_task,
                "follow_up_priority": routing_follow_up_priority,
                "follow_up_blocking": routing_follow_up_blocking,
                "follow_up_bucket": routing_follow_up_bucket,
                "follow_up_sla_seconds": routing_follow_up_sla_seconds,
                "follow_up_requires_immediate_action": routing_follow_up_requires_immediate_action,
                "follow_up_owner": routing_follow_up_owner,
                "follow_up_requires_operator_input": routing_follow_up_requires_operator_input,
                "follow_up_requires_confirmation": routing_follow_up_requires_confirmation,
                "follow_up_action_kind": routing_follow_up_action_kind,
                "follow_up_requires_path_input": routing_follow_up_requires_path_input,
                "follow_up_can_defer": routing_follow_up_can_defer,
                "follow_up_requires_binary_opt_in_input": routing_follow_up_requires_binary_opt_in_input,
                "follow_up_requires_policy_repair_input": routing_follow_up_requires_policy_repair_input,
                "follow_up_requires_content_expansion_input": routing_follow_up_requires_content_expansion_input,
                "follow_up_requires_mixed_follow_up_input": routing_follow_up_requires_mixed_follow_up_input,
                "follow_up_requires_partial_replay_input": routing_follow_up_requires_partial_replay_input,
                "follow_up_requires_user_text_input": routing_follow_up_requires_user_text_input,
                "follow_up_user_text_input_kind": routing_follow_up_user_text_input_kind,
                "requires_binary_opt_in": routing_requires_binary_opt_in,
                "requires_path_repair": routing_requires_path_repair,
                "requires_content_expansion": routing_requires_content_expansion,
                "requires_mixed_follow_up": routing_requires_mixed_follow_up,
                "requires_partial_replay": routing_requires_partial_replay,
                "blocker": routing_blocker,
                "class": routing_class,
                "reason": routing_reason,
                "retryable": routing_retryable,
                "severity": routing_severity,
                "contract_version": FILE_READ_ROUTING_CONTRACT_VERSION
            },
            "recovery": {
                "action": recovery_action,
                "message": recovery_message,
                "retryable": recovery_retryable,
                "class": recovery_class,
                "next_command": recovery_next_command,
                "requires_user_input": recovery_requires_user_input
            }
        });
        let tool_input = json!({
            "path": requested_path,
            "full": full,
            "allow_binary": allow_binary
        });
        let trace_id = crate::deterministic_receipt_hash(&json!({
            "agent_id": agent_id,
            "tool": "file_read",
            "path": requested_path
        }));
        let task_id = format!(
            "tool-file-read-{}",
            trace_id.chars().take(12).collect::<String>()
        );
        let pipeline = tooling_pipeline_execute(
            &trace_id,
            &task_id,
            "file_read",
            &tool_input,
            |_| Ok(payload.clone()),
        );
        if pipeline.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            attach_tool_pipeline(&mut payload, &pipeline);
        }
        finalize_agent_scope_tool_payload(
            root,
            agent_id,
            "file_read",
            &tool_input,
            &mut payload,
            nexus_connection,
        );
        return Some(CompatApiResponse {
            status: 200,
            payload,
        });
    }
    if method == "POST"
        && segments.len() == 2
        && segments[0] == "file"
        && segments[1] == "read-many"
    {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let mut paths = request
            .get("paths")
            .or_else(|| request.get("sources"))
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(|row| row.as_str().map(|v| clean_text(v, 4000)))
            .filter(|row| !row.is_empty())
            .collect::<Vec<_>>();
        if paths.is_empty() {
            let single = clean_text(
                request
                    .get("path")
                    .and_then(Value::as_str)
                    .or_else(|| request.get("file_path").and_then(Value::as_str))
                    .unwrap_or(""),
                4000,
            );
            if !single.is_empty() {
                paths.push(single);
            }
        }
        if paths.is_empty() {
            return Some(CompatApiResponse {
                status: 400,
                payload: json!({
                    "ok": false,
                    "error": "paths_required",
                    "type": "file_read_many",
                    "routing": {
                        "hint": "paths_required",
                        "allow_binary": false,
                        "partial": false,
                        "binary_opt_in_blocked_count": 0,
                        "requires_policy_repair": false,
                        "requires_follow_up": true,
                        "follow_up_can_auto_retry": false,
                        "follow_up_mode": "user_input",
                        "follow_up_task": "path_repair",
                        "follow_up_priority": "high",
                        "follow_up_blocking": true,
                        "follow_up_bucket": "blocking",
                        "follow_up_sla_seconds": 30,
                        "follow_up_requires_immediate_action": true,
                        "follow_up_owner": "operator",
                        "follow_up_requires_operator_input": true,
                        "follow_up_requires_confirmation": true,
                        "follow_up_action_kind": "repair",
                        "follow_up_requires_path_input": true,
                        "follow_up_can_defer": false,
                        "follow_up_requires_binary_opt_in_input": false,
                        "follow_up_requires_policy_repair_input": false,
                        "follow_up_requires_content_expansion_input": false,
                        "follow_up_requires_mixed_follow_up_input": false,
                        "follow_up_requires_partial_replay_input": false,
                        "follow_up_requires_user_text_input": true,
                        "follow_up_user_text_input_kind": "path_input",
                        "requires_binary_opt_in": false,
                        "requires_path_repair": true,
                        "requires_content_expansion": false,
                        "requires_mixed_follow_up": false,
                        "requires_partial_replay": false,
                        "blocker": "path_repair",
                        "class": "path_repair",
                        "reason": "paths_missing",
                        "retryable": true,
                        "severity": "error",
                        "contract_version": FILE_READ_ROUTING_CONTRACT_VERSION
                    },
                    "counts": {
                        "requested": 0,
                        "ok": 0,
                        "failed": 0,
                        "unclassified": 0,
                        "text": 0,
                        "binary": 0,
                        "group_failed": 0,
                        "group_unclassified": 0
                    },
                    "recovery": {
                        "action": "provide_paths_array",
                        "message": "Provide at least one workspace-relative path in paths[] or path.",
                        "retryable": true,
                        "class": "input_repair",
                        "next_command": "file_read_many",
                        "requires_user_input": true
                    }
                }),
            });
        }
        let nexus_connection =
            match crate::dashboard_tool_turn_loop::authorize_ingress_tool_call_with_nexus(
                "file_read_many",
            ) {
                Ok(meta) => meta,
                Err(err) => {
                    return Some(CompatApiResponse {
                        status: 403,
                        payload: json!({
                            "ok": false,
                            "error": "file_read_many_nexus_delivery_denied",
                            "message": "File read-many blocked by hierarchical nexus ingress policy.",
                            "nexus_error": clean_text(&err, 240),
                            "routing": {
                                "hint": "nexus_delivery_denied",
                                "allow_binary": false,
                                "partial": false,
                                "binary_opt_in_blocked_count": 0,
                                "requires_policy_repair": true,
                                "requires_follow_up": true,
                                "follow_up_can_auto_retry": false,
                                "follow_up_mode": "user_input",
                                "follow_up_task": "policy_repair",
                                "follow_up_priority": "critical",
                                "follow_up_blocking": true,
                                "follow_up_bucket": "blocking",
                                "follow_up_sla_seconds": 0,
                                "follow_up_requires_immediate_action": true,
                                "follow_up_owner": "operator",
                                "follow_up_requires_operator_input": true,
                                "follow_up_requires_confirmation": true,
                                "follow_up_action_kind": "repair",
                                "follow_up_requires_path_input": false,
                                "follow_up_can_defer": false,
                                "follow_up_requires_binary_opt_in_input": false,
                                "follow_up_requires_policy_repair_input": true,
                                "follow_up_requires_content_expansion_input": false,
                                "follow_up_requires_mixed_follow_up_input": false,
                                "follow_up_requires_partial_replay_input": false,
                                "follow_up_requires_user_text_input": true,
                                "follow_up_user_text_input_kind": "policy_repair_input",
                                "requires_binary_opt_in": false,
                                "requires_path_repair": false,
                                "requires_content_expansion": false,
                                "requires_mixed_follow_up": false,
                                "requires_partial_replay": false,
                                "blocker": "policy_repair",
                                "class": "policy_denied",
                                "reason": "ingress_policy_denied",
                                "retryable": true,
                                "severity": "error",
                                "contract_version": FILE_READ_ROUTING_CONTRACT_VERSION
                            },
                            "recovery": {
                                "action": "repair_nexus_ingress_policy",
                                "message": "Adjust nexus ingress policy or tool permissions, then retry file_read_many.",
                                "retryable": true,
                                "class": "policy_repair",
                                "next_command": "nexus_policy_update",
                                "requires_user_input": true
                            }
                        }),
                    })
                }
            };
        let workspace_base = workspace_base_for_agent(root, existing.as_ref());
        let full = request
            .get("full")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let allow_binary = request
            .get("allow_binary")
            .or_else(|| request.get("binary"))
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let max_bytes = request
            .get("max_bytes")
            .and_then(Value::as_u64)
            .unwrap_or((256 * 1024) as u64)
            .clamp(1, (8 * 1024 * 1024) as u64) as usize;
        let mut files = Vec::<Value>::new();
        let mut failed = Vec::<Value>::new();
        let mut unclassified = Vec::<Value>::new();
        let mut grouped_text = Vec::<String>::new();
        let mut grouped_binary = Vec::<String>::new();
        let mut grouped_binary_opt_in_blocked = Vec::<String>::new();
        let mut grouped_failed = Vec::<String>::new();
        let mut truncated_count: usize = 0;
        let mut binary_opt_in_blocked_count: usize = 0;
        let mut grouped_unclassified = Vec::<String>::new();
        for requested_path in &paths {
            let target = resolve_workspace_path(&workspace_base, requested_path);
            let Some(target_path) = target else {
                failed.push(json!({
                    "path": requested_path,
                    "error": "path_outside_workspace",
                    "status": 400
                }));
                grouped_failed.push(requested_path.clone());
                grouped_unclassified.push(requested_path.clone());
                continue;
            };
            if !target_path.is_file() {
                let rendered = target_path.to_string_lossy().to_string();
                unclassified.push(json!({
                    "path": rendered,
                    "error": "file_not_found",
                    "status": 404
                }));
                grouped_failed.push(rendered.clone());
                grouped_unclassified.push(target_path.to_string_lossy().to_string());
                continue;
            }
            let bytes = fs::read(&target_path).unwrap_or_default();
            let file_max_bytes = if full { bytes.len().max(1) } else { max_bytes };
            let binary = bytes_look_binary(&bytes);
            let content_type = guess_mime_type_for_file(&target_path, &bytes);
            if binary && !allow_binary {
                let rendered_path = target_path.to_string_lossy().to_string();
                binary_opt_in_blocked_count += 1;
                failed.push(json!({
                    "path": rendered_path,
                    "error": "binary_file_requires_opt_in",
                    "status": 415,
                    "binary": true,
                    "bytes": bytes.len(),
                    "content_type": content_type
                }));
                grouped_failed.push(rendered_path.clone());
                grouped_binary.push(rendered_path.clone());
                grouped_binary_opt_in_blocked.push(rendered_path);
                continue;
            }
            let (content, truncated) = if binary {
                (String::new(), bytes.len() > file_max_bytes)
            } else {
                truncate_utf8_lossy(&bytes, file_max_bytes)
            };
            let content_base64 = if binary {
                use base64::engine::general_purpose::STANDARD;
                use base64::Engine;
                let slice_end = bytes.len().min(file_max_bytes.max(1));
                STANDARD.encode(&bytes[..slice_end])
            } else {
                String::new()
            };
            let download_url = if bytes.len() <= (2 * 1024 * 1024) {
                data_url_from_bytes(&bytes, &content_type)
            } else {
                String::new()
            };
            let file_name = clean_text(
                target_path
                    .file_name()
                    .and_then(|v| v.to_str())
                    .unwrap_or("download.txt"),
                180,
            );
            let rendered_path = target_path.to_string_lossy().to_string();
            if binary {
                grouped_binary.push(rendered_path.clone());
            } else {
                grouped_text.push(rendered_path.clone());
            }
            if truncated {
                truncated_count += 1;
            }
            files.push(json!({
                "ok": true,
                "path": rendered_path,
                "content": content,
                "content_base64": content_base64,
                "truncated": truncated,
                "bytes": bytes.len(),
                "max_bytes": file_max_bytes,
                "full": full,
                "binary": binary,
                "allow_binary": allow_binary,
                "download_url": download_url,
                "file_name": file_name,
                "content_type": content_type
            }));
        }
        let ok = !files.is_empty();
        let routing_hint = file_read_many_routing_hint(
            files.len(),
            grouped_text.len(),
            grouped_binary.len(),
            failed.len(),
            unclassified.len(),
            allow_binary,
        );
        let status = if ok {
            200
        } else {
            failed
                .first()
                .or_else(|| unclassified.first())
                .and_then(|row| row.get("status").and_then(Value::as_u64))
                .unwrap_or(400) as u16
        };
        let partial = ok && (!failed.is_empty() || !unclassified.is_empty());
        let (recovery_action, recovery_message) =
            file_read_many_recovery_action(routing_hint, partial, allow_binary);
        let recovery_retryable = file_read_recovery_retryable(recovery_action);
        let recovery_class = file_read_recovery_class(recovery_action);
        let recovery_next_command = file_read_recovery_next_command(recovery_action);
        let recovery_requires_user_input =
            file_read_recovery_requires_user_input(recovery_action);
        let routing_requires_policy_repair =
            file_read_routing_requires_policy_repair(routing_hint);
        let routing_requires_path_repair =
            file_read_routing_requires_path_repair(routing_hint);
        let routing_requires_binary_opt_in =
            file_read_routing_requires_binary_opt_in(routing_hint, allow_binary);
        let routing_requires_content_expansion =
            file_read_routing_requires_content_expansion(routing_hint, truncated_count);
        let routing_requires_mixed_follow_up =
            file_read_routing_requires_mixed_follow_up(routing_hint);
        let routing_requires_partial_replay =
            file_read_routing_requires_partial_replay(partial, routing_hint);
        let routing_requires_follow_up = file_read_routing_requires_follow_up(
            routing_requires_policy_repair,
            routing_requires_path_repair,
            routing_requires_binary_opt_in,
            routing_requires_content_expansion,
            routing_requires_mixed_follow_up,
            routing_requires_partial_replay,
        );
        let routing_follow_up_can_auto_retry = file_read_routing_follow_up_can_auto_retry(
            routing_requires_follow_up,
            recovery_retryable,
            recovery_requires_user_input,
        );
        let routing_follow_up_mode = file_read_routing_follow_up_mode(
            routing_requires_follow_up,
            recovery_retryable,
            recovery_requires_user_input,
        );
        let routing_follow_up_task = file_read_routing_follow_up_task(
            routing_requires_policy_repair,
            routing_requires_path_repair,
            routing_requires_binary_opt_in,
            routing_requires_content_expansion,
            routing_requires_mixed_follow_up,
            routing_requires_partial_replay,
        );
        let routing_follow_up_priority = file_read_routing_follow_up_priority(
            routing_follow_up_task,
            routing_requires_follow_up,
        );
        let routing_follow_up_blocking = file_read_routing_follow_up_blocking(
            routing_follow_up_priority,
            routing_requires_follow_up,
        );
        let routing_follow_up_bucket = file_read_routing_follow_up_bucket(
            routing_requires_follow_up,
            routing_follow_up_blocking,
            routing_follow_up_mode,
        );
        let routing_follow_up_sla_seconds = file_read_routing_follow_up_sla_seconds(
            routing_requires_follow_up,
            routing_follow_up_priority,
        );
        let routing_follow_up_requires_immediate_action =
            file_read_routing_follow_up_requires_immediate_action(
                routing_requires_follow_up,
                routing_follow_up_sla_seconds,
            );
        let routing_follow_up_owner = file_read_routing_follow_up_owner(
            routing_requires_follow_up,
            routing_follow_up_mode,
        );
        let routing_follow_up_requires_operator_input =
            file_read_routing_follow_up_requires_operator_input(
                routing_requires_follow_up,
                routing_follow_up_owner,
            );
        let routing_follow_up_requires_confirmation =
            file_read_routing_follow_up_requires_confirmation(
                routing_requires_follow_up,
                routing_follow_up_owner,
                routing_follow_up_mode,
            );
        let routing_follow_up_action_kind = file_read_routing_follow_up_action_kind(
            routing_requires_follow_up,
            routing_follow_up_mode,
        );
        let routing_follow_up_requires_path_input =
            file_read_routing_follow_up_requires_path_input(
                routing_requires_follow_up,
                routing_requires_path_repair,
            );
        let routing_follow_up_can_defer = file_read_routing_follow_up_can_defer(
            routing_requires_follow_up,
            routing_follow_up_requires_immediate_action,
        );
        let routing_follow_up_requires_binary_opt_in_input =
            file_read_routing_follow_up_requires_binary_opt_in_input(
                routing_requires_follow_up,
                routing_requires_binary_opt_in,
            );
        let routing_follow_up_requires_policy_repair_input =
            file_read_routing_follow_up_requires_policy_repair_input(
                routing_requires_follow_up,
                routing_requires_policy_repair,
            );
        let routing_follow_up_requires_content_expansion_input =
            file_read_routing_follow_up_requires_content_expansion_input(
                routing_requires_follow_up,
                routing_requires_content_expansion,
            );
        let routing_follow_up_requires_mixed_follow_up_input =
            file_read_routing_follow_up_requires_mixed_follow_up_input(
                routing_requires_follow_up,
                routing_requires_mixed_follow_up,
            );
        let routing_follow_up_requires_partial_replay_input =
            file_read_routing_follow_up_requires_partial_replay_input(
                routing_requires_follow_up,
                routing_requires_partial_replay,
            );
        let routing_follow_up_requires_user_text_input =
            file_read_routing_follow_up_requires_user_text_input(
                routing_requires_follow_up,
                routing_follow_up_owner,
                routing_follow_up_requires_path_input,
                routing_follow_up_requires_policy_repair_input,
                routing_follow_up_requires_binary_opt_in_input,
                routing_follow_up_requires_content_expansion_input,
                routing_follow_up_requires_mixed_follow_up_input,
                routing_follow_up_requires_partial_replay_input,
            );
        let routing_follow_up_user_text_input_kind =
            file_read_routing_follow_up_user_text_input_kind(
                routing_requires_follow_up,
                routing_follow_up_owner,
                routing_follow_up_requires_path_input,
                routing_follow_up_requires_policy_repair_input,
                routing_follow_up_requires_binary_opt_in_input,
                routing_follow_up_requires_content_expansion_input,
                routing_follow_up_requires_mixed_follow_up_input,
                routing_follow_up_requires_partial_replay_input,
            );
        let routing_reason = file_read_many_routing_reason(routing_hint, allow_binary);
        let routing_retryable = file_read_many_routing_retryable(routing_hint);
        let routing_severity = file_read_routing_severity(routing_hint);
        let routing_class = file_read_routing_class(routing_hint);
        let routing_requires_binary_opt_in =
            file_read_routing_requires_binary_opt_in(routing_hint, allow_binary);
        let routing_requires_path_repair =
            file_read_routing_requires_path_repair(routing_hint);
        let routing_requires_content_expansion =
            file_read_routing_requires_content_expansion(routing_hint, truncated_count);
        let routing_requires_mixed_follow_up =
            file_read_routing_requires_mixed_follow_up(routing_hint);
        let routing_requires_partial_replay =
            file_read_routing_requires_partial_replay(partial, routing_hint);
        let routing_requires_policy_repair =
            file_read_routing_requires_policy_repair(routing_hint);
        let routing_requires_follow_up = file_read_routing_requires_follow_up(
            routing_requires_policy_repair,
            routing_requires_path_repair,
            routing_requires_binary_opt_in,
            routing_requires_content_expansion,
            routing_requires_mixed_follow_up,
            routing_requires_partial_replay,
        );
        let routing_binary_opt_in_blocked_count: usize = binary_opt_in_blocked_count;
        let routing_blocker = file_read_routing_blocker(
            routing_hint,
            routing_requires_path_repair,
            routing_requires_binary_opt_in,
            routing_requires_content_expansion,
            routing_requires_partial_replay,
            routing_requires_mixed_follow_up,
        );
        let mut payload = json!({
            "ok": ok,
            "type": "file_read_many",
            "files": files,
            "failed": failed,
            "unclassified": unclassified,
            "partial": partial,
            "groups": {
                "text": grouped_text,
                "binary": grouped_binary,
                "binary_opt_in_blocked": grouped_binary_opt_in_blocked,
                "failed": grouped_failed,
                "unclassified": grouped_unclassified
            },
            "counts": {
                "requested": paths.len(),
                "ok": files.len(),
                "failed": failed.len(),
                "unclassified": unclassified.len(),
                "text": grouped_text.len(),
                "binary": grouped_binary.len(),
                "group_binary_opt_in_blocked": grouped_binary_opt_in_blocked.len(),
                "group_failed": grouped_failed.len(),
                "group_unclassified": grouped_unclassified.len(),
                "truncated": truncated_count,
                "binary_opt_in_blocked": binary_opt_in_blocked_count
            },
            "routing": {
                "hint": routing_hint,
                "allow_binary": allow_binary,
                "partial": partial,
                "binary_opt_in_blocked_count": routing_binary_opt_in_blocked_count,
                "requires_policy_repair": routing_requires_policy_repair,
                "requires_follow_up": routing_requires_follow_up,
                "follow_up_can_auto_retry": routing_follow_up_can_auto_retry,
                "follow_up_mode": routing_follow_up_mode,
                "follow_up_task": routing_follow_up_task,
                "follow_up_priority": routing_follow_up_priority,
                "follow_up_blocking": routing_follow_up_blocking,
                "follow_up_bucket": routing_follow_up_bucket,
                "follow_up_sla_seconds": routing_follow_up_sla_seconds,
                "follow_up_requires_immediate_action": routing_follow_up_requires_immediate_action,
                "follow_up_owner": routing_follow_up_owner,
                "follow_up_requires_operator_input": routing_follow_up_requires_operator_input,
                "follow_up_requires_confirmation": routing_follow_up_requires_confirmation,
                "follow_up_action_kind": routing_follow_up_action_kind,
                "follow_up_requires_path_input": routing_follow_up_requires_path_input,
                "follow_up_can_defer": routing_follow_up_can_defer,
                "follow_up_requires_binary_opt_in_input": routing_follow_up_requires_binary_opt_in_input,
                "follow_up_requires_policy_repair_input": routing_follow_up_requires_policy_repair_input,
                "follow_up_requires_content_expansion_input": routing_follow_up_requires_content_expansion_input,
                "follow_up_requires_mixed_follow_up_input": routing_follow_up_requires_mixed_follow_up_input,
                "follow_up_requires_partial_replay_input": routing_follow_up_requires_partial_replay_input,
                "follow_up_requires_user_text_input": routing_follow_up_requires_user_text_input,
                "follow_up_user_text_input_kind": routing_follow_up_user_text_input_kind,
                "requires_binary_opt_in": routing_requires_binary_opt_in,
                "requires_path_repair": routing_requires_path_repair,
                "requires_content_expansion": routing_requires_content_expansion,
                "requires_mixed_follow_up": routing_requires_mixed_follow_up,
                "requires_partial_replay": routing_requires_partial_replay,
                "blocker": routing_blocker,
                "class": routing_class,
                "reason": routing_reason,
                "retryable": routing_retryable,
                "severity": routing_severity,
                "contract_version": FILE_READ_ROUTING_CONTRACT_VERSION
            },
            "recovery": {
                "action": recovery_action,
                "message": recovery_message,
                "retryable": recovery_retryable,
                "class": recovery_class,
                "next_command": recovery_next_command,
                "requires_user_input": recovery_requires_user_input
            }
        });
        let tool_input = json!({
            "paths": paths,
            "full": full,
            "allow_binary": allow_binary
        });
        let trace_id = crate::deterministic_receipt_hash(&json!({
            "agent_id": agent_id,
            "tool": "file_read_many",
            "paths": paths
        }));
        let task_id = format!(
            "tool-file-read-many-{}",
            trace_id.chars().take(12).collect::<String>()
        );
        let pipeline = tooling_pipeline_execute(
            &trace_id,
            &task_id,
            "file_read_many",
            &tool_input,
            |_| Ok(payload.clone()),
        );
        if pipeline.get("ok").and_then(Value::as_bool).unwrap_or(false) {
            attach_tool_pipeline(&mut payload, &pipeline);
        }
        finalize_agent_scope_tool_payload(
            root,
            agent_id,
            "file_read_many",
            &tool_input,
            &mut payload,
            nexus_connection,
        );
        return Some(CompatApiResponse { status, payload });
    }
    None
}
