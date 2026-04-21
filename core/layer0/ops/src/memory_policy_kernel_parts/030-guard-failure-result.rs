
fn guard_failure_result(validation: Option<&Value>, context: Option<&Value>) -> Value {
    let reason = validation
        .and_then(|value| value.get("reason_code"))
        .and_then(Value::as_str)
        .unwrap_or("policy_validation_failed");
    let mut payload = Map::<String, Value>::new();
    payload.insert("ok".to_string(), Value::Bool(false));
    payload.insert(
        "type".to_string(),
        Value::String("memory_policy_guard_reject".to_string()),
    );
    payload.insert("reason".to_string(), Value::String(reason.to_string()));
    payload.insert(
        "layer".to_string(),
        Value::String("client_runtime_memory_guard".to_string()),
    );
    payload.insert("fail_closed".to_string(), Value::Bool(true));

    if let Some(context_obj) = context.and_then(Value::as_object) {
        for (key, value) in context_obj {
            payload.insert(key.clone(), value.clone());
        }
    }

    json!({
        "ok": false,
        "status": 2,
        "stdout": format!("{}\n", Value::Object(payload.clone())),
        "stderr": format!("memory_policy_guard_reject:{}\n", reason),
        "payload": Value::Object(payload),
    })
}

pub fn run(_cwd: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let payload = match command.as_str() {
        "status" => Ok(json!({
            "ok": true,
            "type": "memory_policy_kernel_status",
            "default_policy": Policy::default(),
        })),
        "parse-cli" => load_payload(argv).map(|payload| {
            json!({
                "ok": true,
                "parsed": parse_cli_args(&payload.args),
            })
        }),
        "command-name" => load_payload(argv).map(|payload| {
            let fallback = payload.fallback.unwrap_or_else(|| "status".to_string());
            let parsed = parse_cli_args(&payload.args);
            let command = parsed
                .positional
                .first()
                .cloned()
                .unwrap_or(fallback)
                .trim()
                .to_ascii_lowercase();
            json!({
                "ok": true,
                "command": command,
            })
        }),
        "validate" => load_payload(argv).map(|payload| {
            json!({
                "ok": true,
                "validation": validate_memory_policy(&payload.args, payload.options.as_ref()),
            })
        }),
        "validate-ranking" => load_payload(argv).map(|payload| {
            json!({
                "ok": true,
                "validation": validate_descending_ranking(
                    &payload.scores.unwrap_or_default(),
                    &payload.ids.unwrap_or_default(),
                ),
            })
        }),
        "validate-lensmap" => load_payload(argv).map(|payload| {
            json!({
                "ok": true,
                "validation": validate_lensmap_annotation(payload.annotation.as_ref()),
            })
        }),
        "severity-rank" => load_payload(argv).map(|payload| {
            let value = payload.value.unwrap_or(Value::Null);
            json!({
                "ok": true,
                "rank": severity_rank_value(&value_as_text(&value)),
            })
        }),
        "guard-failure" => load_payload(argv).map(|payload| {
            json!({
                "ok": true,
                "result": guard_failure_result(payload.validation.as_ref(), payload.context.as_ref()),
            })
        }),
        _ => Err(format!("memory_policy_kernel_unknown_command:{command}")),
    };

    match payload {
        Ok(payload) => {
            let ok = payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
            print_json_line(&cli_receipt(
                &format!("memory_policy_kernel_{}", command.replace('-', "_")),
                payload,
            ));
            if ok {
                0
            } else {
                1
            }
        }
        Err(err) => {
            print_json_line(&cli_error("memory_policy_kernel_error", &err));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_direct_file_reads() {
        let validation = validate_memory_policy(
            &[
                "query-index".to_string(),
                "--session-id=s1".to_string(),
                "--path=local/workspace/memory/2026-03-15.md".to_string(),
            ],
            None,
        );
        assert_eq!(validation.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            validation.get("reason_code").and_then(Value::as_str),
            Some("direct_file_read_forbidden")
        );
    }

    #[test]
    fn validates_lensmap_annotation_rules() {
        let failed = validate_lensmap_annotation(Some(&json!({
            "node_id": "n1",
            "tags": [],
            "jots": [],
        })));
        assert_eq!(failed.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            failed.get("reason_code").and_then(Value::as_str),
            Some("lensmap_annotation_missing_tags_or_jots")
        );

        let passed = validate_lensmap_annotation(Some(&json!({
            "node_id": "n1",
            "tags": ["memory"],
            "jots": ["note"],
        })));
        assert_eq!(passed.get("ok").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn guard_failure_result_is_fail_closed() {
        let result = guard_failure_result(
            Some(&json!({
                "reason_code": "index_first_bypass_forbidden"
            })),
            Some(&json!({
                "stage": "client_preflight"
            })),
        );
        assert_eq!(result.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(result.get("status").and_then(Value::as_i64), Some(2));
        assert!(result
            .get("stderr")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .contains("memory_policy_guard_reject:index_first_bypass_forbidden"));
    }
}
