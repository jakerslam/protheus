
pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = crate::parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|row| row.to_ascii_lowercase())
        .unwrap_or_else(|| "list".to_string());
    let nexus_connection = if context_stacks_nexus_enabled(&parsed) {
        match authorize_context_stacks_command_with_nexus(command.as_str()) {
            Ok(meta) => Some(meta),
            Err(err) => {
                let fail_payload = json!({
                    "ok": false,
                    "status": "blocked",
                    "error": "context_stacks_nexus_error",
                    "reason": clean(err.as_str(), 220),
                    "fail_closed": true
                });
                println!(
                    "{}",
                    serde_json::to_string_pretty(&fail_payload).unwrap_or_else(|_| {
                        "{\"ok\":false,\"status\":\"blocked\",\"error\":\"encode_failed\"}"
                            .to_string()
                    })
                );
                return 1;
            }
        }
    } else {
        Some(json!({
            "enabled": false,
            "reason": "nexus_disabled_by_flag_or_env"
        }))
    };
    let payload = match command.as_str() {
        "help" | "--help" | "-h" => {
            context_stacks_usage();
            json!({"ok": true, "type": "context_stacks_help"})
        }
        "status" => context_stacks_status(root),
        "policy" => context_stacks_policy_json(root),
        "create" => create_context_stack(root, &parsed),
        "list" => list_context_stacks(root, &parsed),
        "archive" => archive_context_stack(root, &parsed),
        "tail-merge" | "tail_merge" | "tail-append" | "tail_append" => {
            merge_context_stack_tail(root, &parsed)
        }
        "tail-promote" | "tail_promote" => promote_context_stack_tail(root, &parsed),
        "render" => render_context_stack(root, &parsed),
        "batch-class" | "batch_class" => batch_class_context_stack(root, &parsed),
        "scheduler-check" | "scheduler_check" => scheduler_check_context_stack(root, &parsed),
        "node-spike" | "node_spike" | "spike" => context_stacks_node_spike(root, &parsed),
        "contract-verify" | "contract_verify" => context_stacks_contract_verify(root, &parsed),
        "taste-tune" | "taste_tune" => context_stacks_taste_tune(root, &parsed),
        "partial-merge" | "partial_merge" => context_stacks_partial_merge(root, &parsed),
        "hybrid-retrieve" | "hybrid_retrieve" => context_stacks_hybrid_retrieve(root, &parsed),
        "speculative-start" | "speculative_start" => context_stacks_speculative_start(root, &parsed),
        "speculative-merge" | "speculative_merge" => context_stacks_speculative_merge(root, &parsed),
        "speculative-rollback" | "speculative_rollback" => {
            context_stacks_speculative_rollback(root, &parsed)
        }
        "speculative-status" | "speculative_status" => context_stacks_speculative_status(root, &parsed),
        _ => json!({
            "ok": false,
            "status": "blocked",
            "error": "context_stacks_unknown_command",
            "command": command
        }),
    };
    let mut payload = payload;
    if let Some(meta) = nexus_connection {
        payload["nexus_connection"] = meta;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&payload)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
    if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        0
    } else {
        1
    }
}
