fn search_query_shape_suggested_next_action(query: &str, reason: &str) -> Value {
    if reason == "query_prefers_fetch_url" {
        let requested_url = search_query_fetch_url_candidate(query)
            .unwrap_or_else(|| clean_text(query, 2_200).trim().to_string());
        json!({
            "action": "web_conduit_fetch",
            "payload": {
                "requested_url": requested_url,
                "requested_url_input": clean_text(query, 2_200),
                "summary_only": true
            }
        })
    } else {
        Value::Null
    }
}
