
#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn normalize_topics_includes_defaults() {
        let payload = json!({"topics": ["alpha", "decision"]});
        let out = command_normalize_topics(lane_utils::payload_obj(&payload));
        let topics = out
            .get("topics")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(topics
            .iter()
            .any(|row| row.as_str() == Some("conversation")));
        assert!(topics.iter().any(|row| row.as_str() == Some("alpha")));
        assert_eq!(
            topics,
            vec![
                json!("conversation"),
                json!("decision"),
                json!("insight"),
                json!("directive"),
                json!("t1"),
                json!("alpha"),
            ]
        );
    }

    #[test]
    fn process_nodes_dedupes_edges_and_preserves_topic_order() {
        let payload = json!({
            "index": { "emitted_node_ids": {} },
            "topics": ["conversation", "decision", "insight", "directive", "t1", "browser", "fetch"],
            "max_items": 1,
            "candidates": [
                {
                    "node": {
                        "node_id": "n1",
                        "ts": "2026-01-01T00:00:00Z",
                        "title": "First node",
                        "preview": "Collected from the web",
                        "level": 3,
                        "node_tags": ["collector", "collector", "web"],
                        "edges_to": ["alpha", "alpha", "beta"]
                    }
                }
            ]
        });
        let out = command_process_nodes(lane_utils::payload_obj(&payload));
        let items = out
            .get("items")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert_eq!(items.len(), 1);
        let item = items[0].as_object().cloned().unwrap_or_default();
        assert_eq!(
            item.get("topics").cloned().unwrap_or(Value::Null),
            json!(["conversation", "decision", "insight", "directive", "t1", "browser", "fetch"])
        );
        assert_eq!(
            item.get("edges_to").cloned().unwrap_or(Value::Null),
            json!(["alpha", "beta"])
        );
    }

    #[test]
    fn apply_node_rejects_duplicate() {
        let payload = json!({
            "index": { "emitted_node_ids": {"abc": "2026-01-01T00:00:00Z"} },
            "node": {"node_id": "abc", "ts": "2026-01-01T00:00:00Z", "level": 3}
        });
        let out = command_apply_node(lane_utils::payload_obj(&payload));
        assert_eq!(out.get("allowed").and_then(Value::as_bool), Some(false));
        assert_eq!(out.get("reason").and_then(Value::as_str), Some("duplicate"));
    }

    #[test]
    fn process_nodes_batches_and_limits_items() {
        let payload = json!({
            "index": { "emitted_node_ids": {} },
            "topics": ["conversation", "decision"],
            "max_items": 1,
            "candidates": [
                {
                    "node": { "node_id": "n1", "title": "one", "preview":"p1", "ts":"2026-03-27T00:00:00Z", "level": 3 },
                    "recall": { "matches":[{"node_id":"x"}], "attention": {"queued": true} }
                },
                {
                    "node": { "node_id": "n2", "title": "two", "preview":"p2", "ts":"2026-03-27T00:00:01Z", "level": 3 },
                    "recall": { "matches":[{"node_id":"y"}], "attention": {"queued": true} }
                }
            ]
        });
        let out = command_process_nodes(lane_utils::payload_obj(&payload));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("items")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
        assert_eq!(out.get("node_writes").and_then(Value::as_u64), Some(1));
    }

    #[test]
    fn begin_collection_hydrates_runtime_payload() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path();
        let history_path = root.join(DEFAULT_HISTORY_REL);
        if let Some(parent) = history_path.parent() {
            fs::create_dir_all(parent).expect("mkdir history parent");
        }
        fs::write(
            &history_path,
            "{\"id\":\"row1\",\"text\":\"hello from history\"}\n",
        )
        .expect("write history jsonl");

        let out = command_begin_collection(
            root,
            lane_utils::payload_obj(&json!({
                "budgets": { "max_items": 3 }
            })),
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("success").and_then(Value::as_bool), Some(true));
        assert!(out
            .get("source_rows")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        assert!(out
            .get("topics")
            .and_then(Value::as_array)
            .map(|rows| rows.iter().any(|r| r.as_str() == Some("conversation")))
            .unwrap_or(false));
    }
}

