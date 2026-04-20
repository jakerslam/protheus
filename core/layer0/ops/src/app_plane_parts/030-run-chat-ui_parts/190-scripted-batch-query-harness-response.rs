fn scripted_batch_query_harness_response(root: &Path, query: &str) -> Option<Value> {
    let path = root.join("client/runtime/local/state/ui/infring_dashboard/test_chat_script.json");
    let mut script = read_json(&path).unwrap_or_else(|| json!({}));
    let step = script
        .get_mut("batch_query_queue")
        .and_then(Value::as_array_mut)
        .and_then(|queue| {
            if queue.is_empty() {
                None
            } else {
                Some(queue.remove(0))
            }
        });
    let mut payload = step?;
    if !payload.is_object() {
        payload = json!({});
    }
    if payload.get("type").is_none() {
        payload["type"] = json!("batch_query");
    }
    if payload.get("query").is_none() {
        payload["query"] = json!(clean(query, 320));
    }
    if let Some(obj) = script.as_object_mut() {
        let calls = obj
            .entry("batch_query_calls".to_string())
            .or_insert_with(|| json!([]));
        if let Some(rows) = calls.as_array_mut() {
            rows.push(json!({
                "query": clean(query, 320)
            }));
        }
    }
    let _ = write_json(&path, &script);
    Some(payload)
}

#[cfg(test)]
