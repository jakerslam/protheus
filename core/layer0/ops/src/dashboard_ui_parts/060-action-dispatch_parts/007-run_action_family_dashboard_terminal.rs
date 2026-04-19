fn run_action_family_dashboard_terminal(root: &Path, normalized: &str, payload: &Value) -> LaneResult {
    match normalized {
        "dashboard.terminal.session.create" => {
            let result = dashboard_terminal_broker::create_session(root, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.terminal.session.create".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.terminal.exec" => {
            let result = dashboard_terminal_broker::exec_command(root, payload);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: result.get("exit_code").and_then(Value::as_i64).unwrap_or(
                    if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                        0
                    } else {
                        2
                    },
                ) as i32,
                argv: vec!["dashboard.terminal.exec".to_string()],
                payload: Some(result),
            }
        }
        "dashboard.terminal.session.close" => {
            let session_id = payload
                .get("session_id")
                .or_else(|| payload.get("sessionId"))
                .and_then(Value::as_str)
                .map(|v| clean_text(v, 120))
                .unwrap_or_default();
            let result = dashboard_terminal_broker::close_session(root, &session_id);
            LaneResult {
                ok: result.get("ok").and_then(Value::as_bool).unwrap_or(false),
                status: if result.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                    0
                } else {
                    2
                },
                argv: vec!["dashboard.terminal.session.close".to_string()],
                payload: Some(result),
            }
        }
        _ => run_action_family_dashboard_system(root, normalized, payload),
    }
}
