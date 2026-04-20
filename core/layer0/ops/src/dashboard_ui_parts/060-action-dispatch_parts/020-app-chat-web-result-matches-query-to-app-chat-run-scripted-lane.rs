
fn app_chat_web_result_matches_query(query: &str, output: &str) -> bool {
    let query_terms = app_chat_alignment_terms(query, 16);
    if query_terms.len() < 2 {
        return true;
    }
    let lowered = clean_text(output, 4_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    let matched = query_terms
        .iter()
        .filter(|term| lowered.contains(term.as_str()))
        .count();
    let required_hits = 2.min(query_terms.len());
    if matched >= required_hits {
        return true;
    }
    let ratio = (matched as f64) / (query_terms.len() as f64);
    let ratio_floor = if query_terms.len() >= 6 { 0.40 } else { 0.34 };
    ratio >= ratio_floor
}

fn app_chat_contains_irrelevant_dump(raw_input: &str, response: &str) -> bool {
    let user_lowered = clean_text(raw_input, 1_200).to_ascii_lowercase();
    let response_lowered = clean_text(response, 16_000).to_ascii_lowercase();
    if response_lowered.is_empty() {
        return false;
    }

    let role_preamble_hits = [
        "i am an expert in the field",
        "my role is to provide",
        "the user has provided",
        "my task is to refine",
        "workflow metadata",
        "the error: context collapse",
    ]
    .iter()
    .filter(|marker| response_lowered.contains(**marker))
    .count();
    if role_preamble_hits >= 2
        && !user_lowered.contains("system prompt")
        && !user_lowered.contains("role prompt")
        && !user_lowered.contains("prompt")
    {
        return true;
    }

    let competitive_dump_hits = [
        "given a tree",
        "input specification",
        "output specification",
        "sample input",
        "sample output",
        "#include <stdio.h>",
        "int main()",
        "public class",
        "translate the following java code",
        "intelligent recommendation",
        "smart recommendations",
    ]
    .iter()
    .filter(|marker| response_lowered.contains(**marker))
    .count();
    competitive_dump_hits >= 3
        && !user_lowered.contains("translate")
        && !user_lowered.contains("python function")
        && !user_lowered.contains("java code")
}

fn app_chat_tool_name_is_web_search(name: &str) -> bool {
    let lowered = clean_text(name, 120).to_ascii_lowercase();
    lowered.contains("web_search")
        || lowered.contains("search_web")
        || lowered.contains("web_query")
        || lowered.contains("batch_query")
        || lowered == "search"
        || lowered.contains("web_fetch")
}

fn app_chat_web_search_call_count(tools: &[Value]) -> usize {
    tools.iter()
        .filter(|row| {
            app_chat_tool_name_is_web_search(
                row.get("name")
                    .or_else(|| row.get("tool"))
                    .or_else(|| row.get("type"))
                    .and_then(Value::as_str)
                    .unwrap_or(""),
            )
        })
        .count()
}

fn app_chat_run_web_batch_query(root: &Path, query: &str, _payload: &Value) -> LaneResult {
    #[cfg(test)]
    {
        if let Some(mock) = _payload.get("__mock_web_batch_query") {
            let mut mock_payload = if mock.is_object() { mock.clone() } else { json!({}) };
            if mock_payload.get("type").is_none() {
                mock_payload["type"] = json!("batch_query");
            }
            if mock_payload.get("query").is_none() {
                mock_payload["query"] = json!(clean_text(query, 320));
            }
            let ok = mock_payload.get("ok").and_then(Value::as_bool).unwrap_or(true);
            return LaneResult {
                ok,
                status: if ok { 0 } else { 1 },
                argv: vec![
                    "batch-query".to_string(),
                    "--source=web".to_string(),
                    format!("--query={}", clean_text(query, 320)),
                    "--aperture=medium".to_string(),
                ],
                payload: Some(mock_payload),
            };
        }
    }
    run_lane(
        root,
        "batch-query",
        &[
            "--source=web".to_string(),
            format!("--query={}", clean_text(query, 320)),
            "--aperture=medium".to_string(),
        ],
    )
}

#[cfg(test)]
fn app_chat_run_scripted_lane(root: &Path, agent_id: &str, input: &str) -> Option<LaneResult> {
    let path = root.join("client/runtime/local/state/ui/infring_dashboard/test_chat_script.json");
    let mut script = std::fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| json!({}));
    let mut step = None::<Value>;
    if let Some(queue) = script.get_mut("queue").and_then(Value::as_array_mut) {
        if !queue.is_empty() {
            step = Some(queue.remove(0));
        }
    }
    let step = step?;
    let mut lane_payload = if step.is_object() { step } else { json!({}) };
    let response = clean_text(
        lane_payload
            .get("response")
            .or_else(|| lane_payload.get("output"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        32_000,
    );
    if lane_payload.get("response").is_none() {
        lane_payload["response"] = Value::String(response.clone());
    }
    if lane_payload.get("output").is_none() {
        lane_payload["output"] = Value::String(response.clone());
    }
    if lane_payload.pointer("/turn/assistant").is_none() {
        lane_payload["turn"] = json!({
            "assistant": response,
            "user": clean_text(input, 2_000),
            "session_id": clean_text(agent_id, 140)
        });
    }
    if !lane_payload.get("tools").map(Value::is_array).unwrap_or(false) {
        lane_payload["tools"] = Value::Array(Vec::new());
    }
    if lane_payload.get("ok").is_none() {
        lane_payload["ok"] = Value::Bool(true);
    }
    if lane_payload.get("type").is_none() {
        lane_payload["type"] = json!("app_plane_chat_ui");
    }
    if let Some(obj) = script.as_object_mut() {
        if !obj.get("calls").map(Value::is_array).unwrap_or(false) {
            obj.insert("calls".to_string(), Value::Array(Vec::new()));
        }
        if let Some(rows) = obj.get_mut("calls").and_then(Value::as_array_mut) {
            rows.push(json!({
                "action": "app.chat",
                "agent_id": clean_text(agent_id, 140),
                "input": clean_text(input, 2_000),
                "ts": crate::now_iso()
            }));
        }
    }
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(body) = serde_json::to_string_pretty(&script) {
        let _ = std::fs::write(&path, body);
    }
    Some(LaneResult {
        ok: lane_payload
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        status: 0,
        argv: vec![
            "app-plane".to_string(),
            "run".to_string(),
            "--app=chat-ui".to_string(),
            format!("--session-id={}", clean_text(agent_id, 140)),
            format!("--input={}", clean_text(input, 2_000)),
        ],
        payload: Some(lane_payload),
    })
}
