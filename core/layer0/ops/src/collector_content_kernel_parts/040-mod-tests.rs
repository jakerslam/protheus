
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn extract_entries_parses_rss_and_atom() {
        let xml = r#"
        <rss><channel>
          <item><title>RSS One</title><link>https://example.com/a</link><description>A</description></item>
        </channel></rss>
        <feed>
          <entry><title>Atom One</title><link href="https://example.com/b"/><summary>B</summary></entry>
        </feed>
        "#;
        let out = extract_entries(xml);
        assert_eq!(out.len(), 2);
    }

    #[test]
    fn map_feed_items_dedupes_and_marks_signal() {
        let payload = json!({
            "collector_id": "demo",
            "entries": [
                { "title": "Alpha", "link": "https://x/a", "description": "urgent event", "published": "" },
                { "title": "Alpha", "link": "https://x/a", "description": "urgent event", "published": "" }
            ],
            "seen_ids": [],
            "signal_regex": "urgent",
            "topics": ["ops"],
            "max_items": 20,
            "bytes_per_entry": 128
        });
        let out = map_feed_items(lane_utils::payload_obj(&payload));
        assert_eq!(
            out.get("items")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
        assert_eq!(
            out.get("items")
                .and_then(Value::as_array)
                .and_then(|rows| rows.first())
                .and_then(|row| row.get("signal"))
                .and_then(Value::as_bool),
            Some(true)
        );
    }

    #[test]
    fn extract_json_rows_handles_known_collectors() {
        let hf = extract_json_rows(lane_utils::payload_obj(&json!({
            "collector_id": "huggingface_papers",
            "topics": ["research"],
            "payload": [
                { "id": "abc", "title": "A paper", "summary": "sum", "upvotes": 25, "publishedAt": "2026-03-01" }
            ]
        })));
        assert_eq!(
            hf.get("rows").and_then(Value::as_array).map(|v| v.len()),
            Some(1)
        );

        let ollama = extract_json_rows(lane_utils::payload_obj(&json!({
            "collector_id": "ollama_search",
            "topics": ["ai"],
            "payload": { "models": [{ "name": "qwen:4b", "size": 123456, "modified_at": "2026-03-01" }] }
        })));
        assert_eq!(
            ollama
                .get("rows")
                .and_then(Value::as_array)
                .and_then(|rows| rows.first())
                .and_then(|row| row.get("title"))
                .and_then(Value::as_str),
            Some("qwen:4b")
        );
    }
}
