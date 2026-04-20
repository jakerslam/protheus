
#[cfg(test)]

mod tests {
    use super::*;
    use tempfile::tempdir;

    fn decode_binary_rows(path: &Path) -> Vec<Value> {
        let Ok(bytes) = fs::read(path) else {
            return Vec::new();
        };
        let mut out = Vec::<Value>::new();
        let mut idx = 0usize;
        while idx + 4 <= bytes.len() {
            let len =
                u32::from_le_bytes([bytes[idx], bytes[idx + 1], bytes[idx + 2], bytes[idx + 3]])
                    as usize;
            idx += 4;
            if idx + len > bytes.len() {
                break;
            }
            if let Ok(value) = serde_json::from_slice::<Value>(&bytes[idx..idx + len]) {
                out.push(value);
            }
            idx += len;
        }
        out
    }

    #[test]
    fn append_jsonl_with_limits_caps_history_and_binary_queue() {
        let dir = tempdir().expect("tempdir");
        let history_path = dir.path().join("history.jsonl");
        for idx in 0..120 {
            let payload = json!({
                "idx": idx,
                "text": "x".repeat(64)
            });
            append_jsonl_with_limits(&history_path, &payload, 1024, true, 1024).expect("append");
        }

        let history_size = fs::metadata(&history_path).expect("history metadata").len();
        assert!(history_size <= 1024 + RETENTION_TAIL_SLACK_BYTES);

        let history_rows = read_jsonl(&history_path);
        assert!(!history_rows.is_empty());
        assert_eq!(
            history_rows
                .last()
                .and_then(|row| row.get("idx"))
                .and_then(Value::as_i64),
            Some(119)
        );

        let queue_path = receipt_binary_queue_path(&history_path);
        let queue_size = fs::metadata(&queue_path).expect("queue metadata").len();
        assert!(queue_size <= 1024);
        let queue_rows = decode_binary_rows(&queue_path);
        assert!(!queue_rows.is_empty());
        assert_eq!(
            queue_rows
                .last()
                .and_then(|row| row.get("idx"))
                .and_then(Value::as_i64),
            Some(119)
        );
    }

    #[test]
    fn append_jsonl_without_binary_queue_skips_binary_file() {
        let dir = tempdir().expect("tempdir");
        let history_path = dir.path().join("history.jsonl");
        append_jsonl_without_binary_queue(&history_path, &json!({"ok": true})).expect("append");

        let queue_path = receipt_binary_queue_path(&history_path);
        assert!(!queue_path.exists());
    }

    #[test]
    fn set_receipt_hash_matches_payload_without_receipt_hash_field() {
        let mut payload = json!({
            "ok": true,
            "type": "receipt_test",
            "receipt_hash": "stale",
            "nested": {"value": 1}
        });
        payload.set_receipt_hash();
        let receipt_hash = payload
            .get("receipt_hash")
            .and_then(Value::as_str)
            .expect("receipt hash")
            .to_string();
        let mut unhashed = payload.clone();
        unhashed
            .as_object_mut()
            .expect("receipt payload")
            .remove("receipt_hash");
        assert_eq!(deterministic_receipt_hash(&unhashed), receipt_hash);
    }
}

