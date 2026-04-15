fn command_queue(root: &Path, argv: &[String]) -> Value {
    let queue_path = resolve_queue_path(root, argv);
    let payload = match load_payload(argv) {
        Ok(payload) => payload,
        Err(error) => return cli_error("approval_gate_kernel_queue", &error),
    };
    let action_envelope = match payload.action_envelope {
        Some(value) => value,
        None => {
            return cli_error(
                "approval_gate_kernel_queue",
                "approval_gate_kernel_action_envelope_missing",
            )
        }
    };
    let reason = payload
        .reason
        .unwrap_or_else(|| "approval_required".to_string());
    let mut queue = match read_queue(&queue_path) {
        Ok(queue) => queue,
        Err(error) => return cli_error("approval_gate_kernel_queue", &error),
    };
    let entry = match queue_entry_from_payload(&action_envelope, &reason) {
        Ok(entry) => entry,
        Err(error) => return cli_error("approval_gate_kernel_queue", &error),
    };
    queue.pending.push(entry.clone());
    if let Err(error) = write_queue(&queue_path, &queue) {
        return cli_error("approval_gate_kernel_queue", &error);
    }
    cli_receipt(
        "approval_gate_kernel_queue",
        json!({
            "ok": true,
            "queue_path": queue_path.to_string_lossy(),
            "queue": queue,
            "result": {
                "success": true,
                "action_id": entry.action_id,
                "message": generate_approval_message(&entry)
            }
        }),
    )
}

fn transition_entry(
    queue: &mut ApprovalQueue,
    action_id: &str,
    deny_reason: Option<&str>,
) -> Result<Value, String> {
    let Some(idx) = queue
        .pending
        .iter()
        .position(|entry| entry.action_id == action_id)
    else {
        return Err(format!("approval_gate_kernel_action_not_found:{action_id}"));
    };
    let mut entry = queue.pending.remove(idx);
    let ts = now_iso();
    let success_message;
    if let Some(reason) = deny_reason {
        entry.status = "DENIED".to_string();
        entry.denied_at = ts.clone();
        entry.deny_reason = reason.to_string();
        queue.denied.push(entry.clone());
        let mut history = entry.clone();
        history.action = "denied".to_string();
        history.history_at = ts;
        queue.history.push(history);
        success_message = format!("DENIED: {}", entry.summary);
    } else {
        entry.status = "APPROVED".to_string();
        entry.approved_at = ts.clone();
        queue.approved.push(entry.clone());
        let mut history = entry.clone();
        history.action = "approved".to_string();
        history.history_at = ts;
        queue.history.push(history);
        success_message = format!(
            "APPROVED: {}. You can now re-run this action.",
            entry.summary
        );
    }
    Ok(json!({
        "success": true,
        "action_id": action_id,
        "message": success_message
    }))
}

fn command_approve(root: &Path, argv: &[String]) -> Value {
    let queue_path = resolve_queue_path(root, argv);
    let Some(action_id) = lane_utils::parse_flag(argv, "action-id", false) else {
        return cli_error(
            "approval_gate_kernel_approve",
            "approval_gate_kernel_action_id_missing",
        );
    };
    let mut queue = match read_queue(&queue_path) {
        Ok(queue) => queue,
        Err(error) => return cli_error("approval_gate_kernel_approve", &error),
    };
    let result = match transition_entry(&mut queue, action_id.trim(), None) {
        Ok(result) => result,
        Err(error) => return cli_error("approval_gate_kernel_approve", &error),
    };
    if let Err(error) = write_queue(&queue_path, &queue) {
        return cli_error("approval_gate_kernel_approve", &error);
    }
    cli_receipt(
        "approval_gate_kernel_approve",
        json!({
            "ok": true,
            "queue_path": queue_path.to_string_lossy(),
            "queue": queue,
            "result": result
        }),
    )
}

fn command_deny(root: &Path, argv: &[String]) -> Value {
    let queue_path = resolve_queue_path(root, argv);
    let Some(action_id) = lane_utils::parse_flag(argv, "action-id", false) else {
        return cli_error(
            "approval_gate_kernel_deny",
            "approval_gate_kernel_action_id_missing",
        );
    };
    let reason = lane_utils::parse_flag(argv, "reason", false)
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "User denied".to_string());
    let mut queue = match read_queue(&queue_path) {
        Ok(queue) => queue,
        Err(error) => return cli_error("approval_gate_kernel_deny", &error),
    };
    let result = match transition_entry(&mut queue, action_id.trim(), Some(&reason)) {
        Ok(result) => result,
        Err(error) => return cli_error("approval_gate_kernel_deny", &error),
    };
    if let Err(error) = write_queue(&queue_path, &queue) {
        return cli_error("approval_gate_kernel_deny", &error);
    }
    cli_receipt(
        "approval_gate_kernel_deny",
        json!({
            "ok": true,
            "queue_path": queue_path.to_string_lossy(),
            "queue": queue,
            "result": result
        }),
    )
}

fn command_was_approved(root: &Path, argv: &[String]) -> Value {
    let queue_path = resolve_queue_path(root, argv);
    let Some(action_id) = lane_utils::parse_flag(argv, "action-id", false) else {
        return cli_error(
            "approval_gate_kernel_was_approved",
            "approval_gate_kernel_action_id_missing",
        );
    };
    let queue = match read_queue(&queue_path) {
        Ok(queue) => queue,
        Err(error) => return cli_error("approval_gate_kernel_was_approved", &error),
    };
    let approved = queue
        .approved
        .iter()
        .any(|entry| entry.action_id == action_id.trim());
    cli_receipt(
        "approval_gate_kernel_was_approved",
        json!({
            "ok": true,
            "queue_path": queue_path.to_string_lossy(),
            "action_id": action_id.trim(),
            "approved": approved
        }),
    )
}

fn command_parse_command(argv: &[String]) -> Value {
    match decode_text_flag(argv, "text-base64") {
        Ok(text) => cli_receipt(
            "approval_gate_kernel_parse_command",
            json!({
                "ok": true,
                "command": parse_approval_command(&text)
            }),
        ),
        Err(error) => cli_error("approval_gate_kernel_parse_command", &error),
    }
}

fn command_parse_yaml(argv: &[String]) -> Value {
    match decode_text_flag(argv, "text-base64") {
        Ok(text) => match serde_yaml::from_str::<ApprovalQueue>(&text) {
            Ok(queue) => cli_receipt(
                "approval_gate_kernel_parse_yaml",
                json!({
                    "ok": true,
                    "queue": queue
                }),
            ),
            Err(error) => cli_error(
                "approval_gate_kernel_parse_yaml",
                &format!("approval_gate_kernel_parse_queue_failed:{error}"),
            ),
        },
        Err(error) => cli_error("approval_gate_kernel_parse_yaml", &error),
    }
}

fn command_replace(root: &Path, argv: &[String]) -> Value {
    let queue_path = resolve_queue_path(root, argv);
    let payload = match load_payload(argv) {
        Ok(payload) => payload,
        Err(error) => return cli_error("approval_gate_kernel_replace", &error),
    };
    let queue = match payload.queue {
        Some(queue) => queue,
        None => {
            return cli_error(
                "approval_gate_kernel_replace",
                "approval_gate_kernel_queue_missing",
            )
        }
    };
    if let Err(error) = write_queue(&queue_path, &queue) {
        return cli_error("approval_gate_kernel_replace", &error);
    }
    cli_receipt(
        "approval_gate_kernel_replace",
        json!({
            "ok": true,
            "queue_path": queue_path.to_string_lossy(),
            "queue": queue
        }),
    )
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    let receipt = match command.as_str() {
        "status" => command_status(root, argv),
        "queue" => command_queue(root, argv),
        "approve" => command_approve(root, argv),
        "deny" => command_deny(root, argv),
        "was-approved" | "was_approved" => command_was_approved(root, argv),
        "parse-command" | "parse_command" => command_parse_command(argv),
        "parse-yaml" | "parse_yaml" => command_parse_yaml(argv),
        "replace" => command_replace(root, argv),
        "help" | "--help" | "-h" => {
            usage();
            cli_receipt("approval_gate_kernel_help", json!({ "ok": true }))
        }
        _ => cli_error("approval_gate_kernel_error", "unknown_command"),
    };
    let exit_code = if receipt.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        0
    } else {
        1
    };
    print_json_line(&receipt);
    exit_code
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_command_recognizes_approve_and_deny() {
        let approve = parse_approval_command("APPROVE act_123");
        assert_eq!(
            approve.get("action").and_then(Value::as_str),
            Some("approve")
        );
        assert_eq!(
            approve.get("action_id").and_then(Value::as_str),
            Some("act_123")
        );

        let deny = parse_approval_command("deny act_456");
        assert_eq!(deny.get("action").and_then(Value::as_str), Some("deny"));
        assert_eq!(
            deny.get("action_id").and_then(Value::as_str),
            Some("act_456")
        );
    }

    #[test]
    fn queue_round_trip_and_transition_work() {
        let entry = queue_entry_from_payload(
            &json!({
                "action_id": "act_123",
                "type": "publish_publicly",
                "summary": "Ship a change",
            }),
            "needs approval",
        )
        .expect("entry");
        let mut queue = ApprovalQueue::default();
        queue.pending.push(entry);
        let result = transition_entry(&mut queue, "act_123", None).expect("approve");
        assert_eq!(result.get("success").and_then(Value::as_bool), Some(true));
        assert!(queue.pending.is_empty());
        assert_eq!(queue.approved.len(), 1);
        assert_eq!(queue.history.len(), 1);

        let encoded = serde_yaml::to_string(&queue).expect("encode");
        let decoded = serde_yaml::from_str::<ApprovalQueue>(&encoded).expect("decode");
        assert_eq!(decoded.approved.len(), 1);
        assert_eq!(decoded.history.len(), 1);
    }
}
