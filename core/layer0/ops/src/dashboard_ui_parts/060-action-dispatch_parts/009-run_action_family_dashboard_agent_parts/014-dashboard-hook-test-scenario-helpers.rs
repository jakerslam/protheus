fn dashboard_hook_test_setup_fixture(root: &Path, payload: &Value) -> Value {
    let fixture = clean_text(
        payload
            .get("fixture")
            .and_then(Value::as_str)
            .or_else(|| payload.get("name").and_then(Value::as_str))
            .unwrap_or("default"),
        120,
    );
    let state = dashboard_hook_mutate_state(root, |state| {
        state["test_fixture"] = Value::String(fixture.clone());
        state["test_fixture_set_at"] = Value::String(crate::now_iso());
    });
    json!({
        "ok": true,
        "type": "dashboard_hooks_test_setup_fixture",
        "fixture": fixture,
        "state": state
    })
}

fn dashboard_hook_test_factory_validate(root: &Path, payload: &Value) -> Value {
    let hook_id = dashboard_hook_resolve_id(payload);
    let phase = clean_text(
        payload
            .get("phase")
            .and_then(Value::as_str)
            .unwrap_or("pre_tool_use"),
        60,
    );
    let command = clean_text(
        payload
            .get("command")
            .and_then(Value::as_str)
            .or_else(|| payload.get("exec").and_then(Value::as_str))
            .unwrap_or(""),
        400,
    );
    let valid = !hook_id.is_empty() && !phase.is_empty() && !command.is_empty();
    let state = dashboard_hook_mutate_state(root, |state| {
        state["last_factory_validation"] = json!({
            "hook_id": hook_id,
            "phase": phase,
            "command": command,
            "valid": valid,
            "validated_at": crate::now_iso()
        });
    });
    json!({
        "ok": true,
        "type": "dashboard_hooks_test_factory_validate",
        "valid": valid,
        "state": state
    })
}

fn dashboard_hook_test_model_context_build(payload: &Value) -> Value {
    let task_id = clean_text(
        payload
            .get("task_id")
            .and_then(Value::as_str)
            .or_else(|| payload.get("taskId").and_then(Value::as_str))
            .unwrap_or(""),
        120,
    );
    let tool_name = clean_text(
        payload
            .get("tool_name")
            .and_then(Value::as_str)
            .or_else(|| payload.get("tool").and_then(Value::as_str))
            .unwrap_or(""),
        120,
    );
    let prompt = clean_text(
        payload
            .get("prompt")
            .and_then(Value::as_str)
            .or_else(|| payload.get("user_input").and_then(Value::as_str))
            .unwrap_or(""),
        1200,
    );
    json!({
        "ok": true,
        "type": "dashboard_hooks_test_model_context_build",
        "context": {
            "task_id": task_id,
            "tool_name": tool_name,
            "prompt": prompt,
            "source": "dashboard.hooks.model_context",
            "generated_at": crate::now_iso()
        }
    })
}

fn dashboard_hook_test_utils_normalize(payload: &Value) -> Value {
    let rows = payload
        .get("values")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut set = std::collections::BTreeSet::<String>::new();
    for row in rows {
        if let Some(raw) = row.as_str() {
            let cleaned = clean_text(raw, 140).to_ascii_lowercase();
            if !cleaned.is_empty() {
                set.insert(cleaned);
            }
        }
    }
    json!({
        "ok": true,
        "type": "dashboard_hooks_test_utils_normalize",
        "normalized": set.into_iter().collect::<Vec<_>>()
    })
}

fn dashboard_hook_test_shell_escape_inspect(payload: &Value) -> Value {
    let input = clean_text(
        payload
            .get("input")
            .and_then(Value::as_str)
            .or_else(|| payload.get("value").and_then(Value::as_str))
            .unwrap_or(""),
        800,
    );
    let escaped = input.replace('\\', "\\\\").replace('"', "\\\"");
    json!({
        "ok": true,
        "type": "dashboard_hooks_test_shell_escape_inspect",
        "input": input,
        "escaped": escaped
    })
}

fn dashboard_hook_test_notification_emit(root: &Path, payload: &Value) -> Value {
    let level = clean_text(
        payload
            .get("level")
            .and_then(Value::as_str)
            .unwrap_or("info"),
        40,
    );
    let message = clean_text(
        payload
            .get("message")
            .and_then(Value::as_str)
            .unwrap_or(""),
        500,
    );
    let state = dashboard_hook_mutate_state(root, |state| {
        state["last_notification"] = json!({
            "level": level,
            "message": message,
            "at": crate::now_iso()
        });
        state["notification_count"] = Value::from(
            i64_from_value(state.get("notification_count"), 0).saturating_add(1),
        );
    });
    json!({
        "ok": true,
        "type": "dashboard_hooks_test_notification_emit",
        "state": state
    })
}

fn dashboard_hook_test_process_simulate(root: &Path, payload: &Value) -> Value {
    let hook_id = dashboard_hook_resolve_id(payload);
    let start = dashboard_hook_process_start(
        root,
        &json!({
            "hook_id": hook_id,
            "phase": clean_text(payload.get("phase").and_then(Value::as_str).unwrap_or("pre_tool_use"), 60),
            "context": clean_text(payload.get("context").and_then(Value::as_str).unwrap_or(""), 400)
        }),
    );
    let run_id = clean_text(
        start.pointer("/run/run_id").and_then(Value::as_str).unwrap_or(""),
        160,
    );
    let status = clean_text(
        payload
            .get("status")
            .and_then(Value::as_str)
            .unwrap_or("completed"),
        60,
    );
    let complete = dashboard_hook_process_complete(
        root,
        &json!({
            "run_id": run_id,
            "status": status,
            "message": clean_text(payload.get("message").and_then(Value::as_str).unwrap_or(""), 400)
        }),
    );
    json!({
        "ok": true,
        "type": "dashboard_hooks_test_process_simulate",
        "start": start,
        "complete": complete
    })
}

fn dashboard_hook_test_task_cancel(root: &Path, payload: &Value) -> Value {
    dashboard_hook_test_process_simulate(
        root,
        &json!({
            "hook_id": dashboard_hook_resolve_id(payload),
            "phase": "task_cancel",
            "status": "cancelled",
            "message": "task cancel simulated"
        }),
    )
}

fn dashboard_hook_test_task_complete(root: &Path, payload: &Value) -> Value {
    dashboard_hook_test_process_simulate(
        root,
        &json!({
            "hook_id": dashboard_hook_resolve_id(payload),
            "phase": "task_complete",
            "status": "completed",
            "message": "task complete simulated"
        }),
    )
}

fn dashboard_hook_test_task_resume(root: &Path, payload: &Value) -> Value {
    dashboard_hook_test_process_simulate(
        root,
        &json!({
            "hook_id": dashboard_hook_resolve_id(payload),
            "phase": "task_resume",
            "status": "completed",
            "message": "task resume simulated"
        }),
    )
}

fn dashboard_hook_test_route(root: &Path, normalized: &str, payload: &Value) -> Value {
    match normalized {
        "dashboard.hooks.test.setupFixture" => dashboard_hook_test_setup_fixture(root, payload),
        "dashboard.hooks.test.factory.validate" => dashboard_hook_test_factory_validate(root, payload),
        "dashboard.hooks.test.modelContext.build" => dashboard_hook_test_model_context_build(payload),
        "dashboard.hooks.test.process.simulate" => dashboard_hook_test_process_simulate(root, payload),
        "dashboard.hooks.test.utils.normalize" => dashboard_hook_test_utils_normalize(payload),
        "dashboard.hooks.test.notification.emit" => dashboard_hook_test_notification_emit(root, payload),
        "dashboard.hooks.test.shellEscape.inspect" => dashboard_hook_test_shell_escape_inspect(payload),
        "dashboard.hooks.test.taskCancel.simulate" => dashboard_hook_test_task_cancel(root, payload),
        "dashboard.hooks.test.taskComplete.simulate" => dashboard_hook_test_task_complete(root, payload),
        "dashboard.hooks.test.taskResume.simulate" => dashboard_hook_test_task_resume(root, payload),
        _ => {
            if let Some(result) = dashboard_hook_test_route_extension(root, normalized, payload) {
                result
            } else {
                json!({
                    "ok": false,
                    "type": "dashboard_hooks_test_route_error",
                    "error": format!("unsupported_hook_test_action:{normalized}")
                })
            }
        }
    }
}

include!("015-dashboard-hook-test-scenario-helpers-extended.rs");
