
pub fn memory_kv_semantic_query(root: &Path, agent_id: &str, query: &str, limit: usize) -> Value {
    let id = normalize_agent_id(agent_id);
    let cleaned_query = clean_text(query, 600);
    if id.is_empty() || cleaned_query.is_empty() {
        return json!({"ok": false, "error": "agent_id_and_query_required"});
    }
    let state = load_session_state(root, &id);
    let query_terms = query_tokens(&cleaned_query, 10);
    let mut matches = state
        .get("memory_kv")
        .and_then(Value::as_object)
        .map(|rows| {
            rows.iter()
                .filter_map(|(key, value)| {
                    let value_text = value_search_text(value, 2000);
                    let score = memory_semantic_score(key, &value_text, &query_terms);
                    if score <= 0 {
                        return None;
                    }
                    let duality_tags = memory_duality_tags(&state, key);
                    Some(json!({
                        "key": clean_text(key, 200),
                        "value": value,
                        "score": score,
                        "snippet": clean_text(&value_text, 220),
                        "duality_tags": duality_tags
                    }))
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    matches.sort_by(|a, b| {
        b.get("score")
            .and_then(Value::as_i64)
            .unwrap_or(0)
            .cmp(&a.get("score").and_then(Value::as_i64).unwrap_or(0))
            .then_with(|| {
                clean_text(a.get("key").and_then(Value::as_str).unwrap_or(""), 200).cmp(
                    &clean_text(b.get("key").and_then(Value::as_str).unwrap_or(""), 200),
                )
            })
    });
    matches.truncate(limit.clamp(1, 25));
    json!({
        "ok": true,
        "type": "dashboard_agent_memory_semantic_query",
        "agent_id": id,
        "query": cleaned_query,
        "matches": matches
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn session_controls_create_switch_delete() {
        let root = tempfile::tempdir().expect("tempdir");
        let created = create_session(root.path(), "agent-z", "Ops");
        assert_eq!(created.get("ok").and_then(Value::as_bool), Some(true));
        let sid = created
            .get("active_session_id")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        assert!(sid.starts_with("s-"));
        let switched = switch_session(root.path(), "agent-z", &sid);
        assert_eq!(switched.get("ok").and_then(Value::as_bool), Some(true));
        let deleted = delete_session(root.path(), "agent-z", &sid);
        assert_eq!(deleted.get("ok").and_then(Value::as_bool), Some(true));
    }

    #[test]
    fn memory_kv_controls_round_trip() {
        let root = tempfile::tempdir().expect("tempdir");
        let set = memory_kv_set(root.path(), "agent-z", "focus.topic", &json!("reliability"));
        assert_eq!(set.get("ok").and_then(Value::as_bool), Some(true));
        let pairs = memory_kv_pairs(root.path(), "agent-z");
        assert_eq!(
            pairs
                .get("kv_pairs")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
        let got = memory_kv_get(root.path(), "agent-z", "focus.topic");
        assert_eq!(
            got.get("value").and_then(Value::as_str),
            Some("reliability")
        );
        let deleted = memory_kv_delete(root.path(), "agent-z", "focus.topic");
        assert_eq!(deleted.get("removed").and_then(Value::as_bool), Some(true));
        let missing = memory_kv_get(root.path(), "agent-z", "focus.topic");
        assert!(missing.get("value").map(Value::is_null).unwrap_or(false));
    }

    #[test]
    fn memory_semantic_query_returns_ranked_matches() {
        let root = tempfile::tempdir().expect("tempdir");
        let _ = memory_kv_set(
            root.path(),
            "agent-q",
            "fact.auth.flow",
            &json!("OAuth callback uses PKCE and nonce binding"),
        );
        let _ = memory_kv_set(
            root.path(),
            "agent-q",
            "fact.release.notes",
            &json!("Dashboard blur transition was tuned for resize"),
        );
        let out = memory_kv_semantic_query(root.path(), "agent-q", "auth pkce", 5);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        let rows = out
            .get("matches")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!rows.is_empty());
        let first_key = rows
            .first()
            .and_then(|row| row.get("key").and_then(Value::as_str))
            .unwrap_or("");
        assert_eq!(first_key, "fact.auth.flow");
    }
}
