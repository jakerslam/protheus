
fn tool_summary_enabled() -> bool {
    primitives_enabled() && bool_env("INFRING_TOOL_SUMMARY_ENABLED", true)
}

fn recovery_hints_enabled() -> bool {
    primitives_enabled() && bool_env("INFRING_TOOL_RECOVERY_HINTS_ENABLED", true)
}

fn default_deny_rules() -> Vec<String> {
    vec![
        "rm -rf /".to_string(),
        "rm -rf /*".to_string(),
        "sudo rm -rf /".to_string(),
        "git reset --hard*".to_string(),
        "git checkout -- *".to_string(),
        "shutdown*".to_string(),
        "reboot*".to_string(),
    ]
}

fn default_ask_rules() -> Vec<String> {
    vec![
        "git push*".to_string(),
        "gh pr create*".to_string(),
        "gh repo create*".to_string(),
        "curl *".to_string(),
        "wget *".to_string(),
        "scp *".to_string(),
        "ssh *".to_string(),
    ]
}

fn load_permission_rules(root: &Path, request: &Value) -> (Vec<String>, Vec<String>) {
    let mut deny = default_deny_rules();
    let mut ask = default_ask_rules();
    if let Some(policy) = read_json(&root.join(TERMINAL_PERMISSION_POLICY_REL)) {
        let (policy_deny, policy_ask) =
            crate::command_permission_kernel::collect_permission_rules_for_kernel(Some(&policy));
        deny.extend(policy_deny);
        ask.extend(policy_ask);
    }
    let (request_deny, request_ask) =
        crate::command_permission_kernel::collect_permission_rules_for_kernel(Some(request));
    deny.extend(request_deny);
    ask.extend(request_ask);
    deny.sort();
    deny.dedup();
    ask.sort();
    ask.dedup();
    (deny, ask)
}

fn permission_gate_payload(root: &Path, request: &Value, command: &str) -> Value {
    let (deny_rules, ask_rules) = load_permission_rules(root, request);
    let (verdict, matched) =
        crate::command_permission_kernel::evaluate_command_permission_for_kernel(
            command,
            &deny_rules,
            &ask_rules,
        );
    json!({
        "verdict": verdict.as_str(),
        "matched": matched,
        "deny_rules_count": deny_rules.len(),
        "ask_rules_count": ask_rules.len()
    })
}

fn output_tokens_estimate(stdout: &str, stderr: &str) -> usize {
    (stdout.len() + stderr.len()) / 4
}

fn command_recovery_hints(command: &str, exit_code: i64, permission_verdict: &str) -> Vec<String> {
    let mut hints = Vec::<String>::new();
    if permission_verdict == "deny" {
        hints.push(
            "Blocked by command policy. Try a safer read-only command or explicit approval."
                .to_string(),
        );
    } else if permission_verdict == "ask" {
        hints.push("Confirmation required for this command before execution.".to_string());
    }
    let detail =
        crate::session_command_discovery_kernel::classify_command_detail_for_kernel(command);
    if detail
        .get("ignored")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        hints.push(
            "This command often returns little output. Add `&& ls` to verify context.".to_string(),
        );
    } else if !detail
        .get("supported")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        let base = clean_text(
            detail
                .get("base_command")
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        );
        if !base.is_empty() {
            hints.push(format!(
                "Unsupported `{base}` flow detected; consider `infring {base}` or `/help`."
            ));
        }
    } else if let Some(canonical) = detail.get("canonical").and_then(Value::as_str) {
        let canonical_clean = clean_text(canonical, 120);
        if !canonical_clean.is_empty() {
            hints.push(format!(
                "Try `{canonical_clean}` for a deterministic wrapper lane."
            ));
        }
    }
    if exit_code != 0 {
        hints.push(
            "Command exited non-zero. Re-run with `--help` or inspect stderr details.".to_string(),
        );
    }
    hints.truncate(3);
    hints
}

fn apply_post_tool_output_filter(
    stdout: String,
    stderr: String,
) -> (String, String, Vec<String>, bool) {
    if !post_tool_filter_enabled() {
        return (stdout, stderr, Vec::new(), false);
    }
    let mut stdout_out = stdout;
    let mut stderr_out = stderr;
    let mut filter_events = Vec::<String>::new();
    let mut low_signal = false;

    let stdout_trimmed = clean_text(&stdout_out, 8_000);
    if crate::tool_output_match_filter::matches_ack_placeholder(&stdout_trimmed) {
        filter_events.push("stdout_ack_placeholder".to_string());
        stdout_out.clear();
        low_signal = true;
    } else if let Some((rewritten, rule_id)) =
        crate::tool_output_match_filter::rewrite_failure_placeholder(&stdout_trimmed)
    {
        stdout_out = rewritten;
        filter_events.push(format!("stdout_rewrite:{rule_id}"));
    }

    let stderr_trimmed = clean_text(&stderr_out, 8_000);
    if crate::tool_output_match_filter::matches_ack_placeholder(&stderr_trimmed) {
        filter_events.push("stderr_ack_placeholder".to_string());
        stderr_out.clear();
        low_signal = true;
    } else if let Some((rewritten, rule_id)) =
        crate::tool_output_match_filter::rewrite_failure_placeholder(&stderr_trimmed)
    {
        stderr_out = rewritten;
        filter_events.push(format!("stderr_rewrite:{rule_id}"));
    }

    if clean_text(&stdout_out, 200).is_empty() && clean_text(&stderr_out, 200).is_empty() {
        low_signal = true;
    }
    (stdout_out, stderr_out, filter_events, low_signal)
}

fn maybe_track_command(
    root: &Path,
    session_id: &str,
    command: &str,
    output_tokens: usize,
) -> Option<Value> {
    if !tracking_enabled() {
        return None;
    }
    let payload = json!({
        "session_id": clean_text(session_id, 120),
        "records": [
            {
                "session_id": clean_text(session_id, 120),
                "command": clean_text(command, 3000),
                "output_tokens": output_tokens as u64
            }
        ]
    });
    crate::session_command_tracking_kernel::record_batch_for_kernel(root, &payload).ok()
}
