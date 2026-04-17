fn run_paths(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let abs = store_abs_path(root, payload)?;
    Ok(json!({
        "ok": true,
        "default_rel_path": DEFAULT_REL_PATH,
        "default_abs_path": default_abs_path(root).to_string_lossy(),
        "store_path": abs.to_string_lossy(),
        "mutation_log_path": mutation_log_path(root).to_string_lossy(),
        "pointer_path": adaptive_pointers_path(root).to_string_lossy(),
        "pointer_index_path": adaptive_pointer_index_path(root).to_string_lossy()
    }))
}

fn run_default_state() -> Value {
    json!({ "ok": true, "state": default_focus_state() })
}

fn run_normalize_state(payload: &Map<String, Value>) -> Value {
    json!({
        "ok": true,
        "state": normalize_state(payload.get("state"), payload.get("fallback"))
    })
}

fn run_read_state(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let abs = store_abs_path(root, payload)?;
    let fallback = payload.get("fallback");
    let state = read_json_value(&abs);
    Ok(json!({
        "ok": true,
        "exists": state.is_some(),
        "path": abs.to_string_lossy(),
        "state": normalize_state(state.as_ref(), fallback)
    }))
}

fn run_ensure_state(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let abs = store_abs_path(root, payload)?;
    let meta = payload
        .get("meta")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let existed = abs.exists();
    let state = if existed {
        normalize_state(read_json_value(&abs).as_ref(), Some(&default_focus_state()))
    } else {
        let state = default_focus_state();
        write_json_atomic(&abs, &state)?;
        state
    };
    append_mutation_log(
        root,
        &abs,
        &meta,
        &state,
        if existed {
            "ensure_focus_state_existing"
        } else {
            "ensure_focus_state"
        },
    )?;
    append_pointer_rows(root, &abs, &state)?;
    Ok(json!({
        "ok": true,
        "path": abs.to_string_lossy(),
        "created": !existed,
        "state": state
    }))
}

fn run_set_state(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let abs = store_abs_path(root, payload)?;
    let meta = payload
        .get("meta")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let state = normalize_state(
        payload.get("state").or_else(|| payload.get("value")),
        Some(&default_focus_state()),
    );
    write_json_atomic(&abs, &state)?;
    append_mutation_log(root, &abs, &meta, &state, "set_focus_state")?;
    append_pointer_rows(root, &abs, &state)?;
    Ok(json!({
        "ok": true,
        "path": abs.to_string_lossy(),
        "state": state
    }))
}

fn with_execution_receipt(command: &str, status: &str, payload: Value) -> Value {
    json!({
        "execution_receipt": {
            "lane": "focus_trigger_store_kernel",
            "command": command,
            "status": status,
            "source": "OPENCLAW-TOOLING-WEB-099",
            "tool_runtime_class": "receipt_wrapped"
        },
        "payload": payload
    })
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    let payload = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("focus_trigger_store_kernel_error", err.as_str()));
            return 1;
        }
    };
    let payload = payload_obj(&payload);
    let result = match command.as_str() {
        "paths" => run_paths(root, payload),
        "default-state" => Ok(run_default_state()),
        "normalize-state" => Ok(run_normalize_state(payload)),
        "read-state" => run_read_state(root, payload),
        "ensure-state" => run_ensure_state(root, payload),
        "set-state" => run_set_state(root, payload),
        "help" | "--help" | "-h" => {
            usage();
            return 0;
        }
        _ => Err(format!(
            "focus_trigger_store_kernel_unknown_command:{command}"
        )),
    };

    match result {
        Ok(payload) => {
            print_json_line(&cli_receipt(
                "focus_trigger_store_kernel",
                with_execution_receipt(command.as_str(), "success", payload),
            ));
            0
        }
        Err(err) => {
            print_json_line(&cli_receipt(
                "focus_trigger_store_kernel",
                with_execution_receipt(
                    command.as_str(),
                    "error",
                    json!({
                        "ok": false,
                        "error": err,
                        "error_kind": "command_failed",
                        "retryable": false
                    }),
                ),
            ));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalize_state_assigns_uid_and_sorts_by_weight() {
        let state = normalize_state(
            Some(&json!({
                "triggers": [
                    {"key":"beta signal", "weight": 3},
                    {"key":"alpha signal", "weight": 9}
                ]
            })),
            None,
        );
        let rows = state
            .get("triggers")
            .and_then(Value::as_array)
            .expect("triggers");
        assert_eq!(rows.len(), 2);
        assert_eq!(
            rows[0].get("key").and_then(Value::as_str),
            Some("alpha_signal")
        );
        assert!(rows[0]
            .get("uid")
            .and_then(Value::as_str)
            .map(|value| !value.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn explicit_path_override_is_denied() {
        let temp = tempfile::tempdir().expect("tempdir");
        let payload = json!({ "file_path": temp.path().join("other.json").to_string_lossy() });
        let err = store_abs_path(temp.path(), payload.as_object().unwrap()).unwrap_err();
        assert!(err.contains("path override denied"));
    }
}
