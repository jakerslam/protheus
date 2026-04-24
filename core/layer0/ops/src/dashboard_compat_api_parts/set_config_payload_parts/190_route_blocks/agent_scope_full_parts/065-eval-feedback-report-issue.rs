fn write_eval_report_transport_request(root: &Path, agent_id: &str, request: &Value) -> PathBuf {
    let hash = crate::deterministic_receipt_hash(&json!({
        "agent_id": clean_agent_id(agent_id),
        "request": request
    }));
    let file_name = format!(
        "{}-{}.json",
        clean_agent_id(agent_id),
        hash.chars().take(16).collect::<String>()
    );
    let path = root
        .join("local/state/ops/eval_agent_chat_monitor/report_requests")
        .join(file_name);
    write_json_pretty(&path, request);
    path
}

fn trigger_orchestration_eval_chat_report(root: &Path, agent_id: &str, request: &Value) -> Value {
    let request_path = write_eval_report_transport_request(root, agent_id, request);
    let manifest_path = root.join("surface/orchestration/Cargo.toml");
    let output = std::process::Command::new("cargo")
        .arg("run")
        .arg("--quiet")
        .arg("--manifest-path")
        .arg(manifest_path)
        .arg("--bin")
        .arg("eval_chat_report_runtime")
        .arg("--")
        .arg(format!("--root={}", root.to_string_lossy()))
        .arg(format!("--agent-id={}", clean_agent_id(agent_id)))
        .arg(format!("--request={}", request_path.to_string_lossy()))
        .current_dir(root)
        .output();
    let Ok(output) = output else {
        return json!({
            "ok": false,
            "error": "orchestration_eval_report_spawn_failed",
            "request_path": request_path.to_string_lossy().to_string()
        });
    };
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let mut payload = parse_json_loose(&stdout).unwrap_or_else(|| {
        json!({
            "ok": false,
            "error": "orchestration_eval_report_output_invalid",
            "stdout": clean_text(&stdout, 800)
        })
    });
    if !output.status.success() {
        payload["ok"] = json!(false);
        payload["error"] = json!(payload
            .get("error")
            .and_then(Value::as_str)
            .unwrap_or("orchestration_eval_report_failed"));
        payload["stderr"] = json!(clean_text(&stderr, 800));
    }
    payload["transport_request_path"] = json!(request_path.to_string_lossy().to_string());
    payload
}

fn handle_agent_scope_eval_feedback_report_issue_routes(
    root: &Path,
    method: &str,
    segments: &[String],
    body: &[u8],
    agent_id: &str,
) -> Option<CompatApiResponse> {
    if method == "POST"
        && segments.len() == 2
        && segments[0] == "eval-feedback"
        && segments[1] == "report-issue"
    {
        let request = serde_json::from_slice::<Value>(body).unwrap_or_else(|_| json!({}));
        let payload = trigger_orchestration_eval_chat_report(root, agent_id, &request);
        return Some(CompatApiResponse {
            status: if payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                200
            } else {
                400
            },
            payload,
        });
    }
    None
}
