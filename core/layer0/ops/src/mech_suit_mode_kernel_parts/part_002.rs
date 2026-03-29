fn run_command(root: &Path, command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    let workspace = workspace_root(root);
    match command {
        "load-policy" => {
            let policy = load_policy(&workspace, payload);
            Ok(json!({ "ok": true, "policy": policy }))
        }
        "approx-token-count" => {
            let value = payload.get("value").cloned().unwrap_or(Value::Null);
            Ok(json!({ "ok": true, "token_count": approx_token_count_value(&value) }))
        }
        "classify-severity" => {
            let message = text_token(payload.get("message"), 600);
            let patterns = as_array(payload.get("patterns"))
                .iter()
                .map(|row| text_token(Some(row), 80).to_ascii_lowercase())
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>();
            Ok(json!({ "ok": true, "severity": classify_severity_value(&message, &patterns) }))
        }
        "should-emit-console" => {
            let message = text_token(payload.get("message"), 600);
            let method = text_token(payload.get("method"), 24).to_ascii_lowercase();
            let policy = if let Some(obj) = as_object(payload.get("policy")) {
                normalize_policy(
                    Some(obj),
                    &workspace,
                    &resolve_policy_path(&workspace, payload),
                )
            } else {
                load_policy(&workspace, payload)
            };
            Ok(
                json!({ "ok": true, "emit": should_emit_console_value(&message, &method, &policy), "policy": policy }),
            )
        }
        "update-status" => {
            let component = text_token(payload.get("component"), 80);
            if component.is_empty() {
                return Err("mech_suit_mode_kernel_missing_component".to_string());
            }
            let patch = payload.get("patch").cloned().unwrap_or_else(|| json!({}));
            let policy = if let Some(obj) = as_object(payload.get("policy")) {
                normalize_policy(
                    Some(obj),
                    &workspace,
                    &resolve_policy_path(&workspace, payload),
                )
            } else {
                load_policy(&workspace, payload)
            };
            let status = update_status_value(&workspace, &policy, &component, &patch)?;
            Ok(json!({ "ok": true, "status": status }))
        }
        "append-attention-event" => {
            let event = payload.get("event").cloned().unwrap_or_else(|| json!({}));
            let run_context =
                text_token(payload.get("run_context"), 40).if_empty_then("eyes".to_string());
            let policy = if let Some(obj) = as_object(payload.get("policy")) {
                normalize_policy(
                    Some(obj),
                    &workspace,
                    &resolve_policy_path(&workspace, payload),
                )
            } else {
                load_policy(&workspace, payload)
            };
            append_attention_event_value(&workspace, &policy, &event, &run_context)
        }
        _ => Err("mech_suit_mode_kernel_unknown_command".to_string()),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let Some(command) = argv.first().map(|v| v.as_str()) else {
        usage();
        return 1;
    };
    if matches!(command, "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("mech_suit_mode_kernel", &err));
            return 1;
        }
    };
    let payload = payload_obj(&payload).clone();
    match run_command(root, command, &payload) {
        Ok(out) => {
            print_json_line(&cli_receipt("mech_suit_mode_kernel", out));
            0
        }
        Err(err) => {
            print_json_line(&cli_error("mech_suit_mode_kernel", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_root(name: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "mech-suit-mode-kernel-{}-{}-{}",
            name,
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ));
        fs::create_dir_all(root.join("client/runtime/config")).unwrap();
        root
    }

    #[test]
    fn classify_and_emit_gate_match_policy() {
        let policy = default_policy_value(Path::new("/tmp"));
        assert_eq!(
            classify_severity_value("integrity fail in spine", &[]),
            "critical"
        );
        assert!(!should_emit_console_value(
            "warning: retry queued",
            "error",
            &policy
        ));
        assert!(should_emit_console_value(
            "critical integrity failure",
            "log",
            &policy
        ));
    }

    #[test]
    fn append_attention_event_writes_queue_and_status() {
        let root = temp_root("attention");
        let policy_path = root.join(DEFAULT_POLICY_REL);
        write_json(
            &policy_path,
            &json!({
                "enabled": true,
                "eyes": {
                    "push_attention_queue": true,
                    "push_event_types": ["eye_run_failed"]
                }
            }),
        )
        .unwrap();
        let payload = json!({
            "event": {
                "type": "eye_run_failed",
                "eye_id": "hn_frontpage",
                "error": "transport denied",
                "error_code": "auth_denied"
            }
        });
        let out = run_command(&root, "append-attention-event", payload_obj(&payload)).unwrap();
        assert_eq!(out.get("queued").and_then(Value::as_bool), Some(true));
        assert!(root.join(DEFAULT_ATTENTION_QUEUE_REL).exists());
        assert!(root.join(DEFAULT_STATUS_REL).exists());
    }
}

