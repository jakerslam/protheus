
#[cfg(test)]
mod tests {
    use super::*;
    use base64::Engine;
    use proptest::prelude::*;
    use tempfile::tempdir;

    fn write_policy(root: &Path, max_queue_depth: usize, drop_below: &str) {
        let policy = json!({
            "enabled": true,
            "eyes": {
                "push_attention_queue": true,
                "attention_queue_path": "local/state/attention/queue.jsonl",
                "receipts_path": "local/state/attention/receipts.jsonl",
                "latest_path": "local/state/attention/latest.json",
                "attention_contract": {
                    "max_queue_depth": max_queue_depth,
                    "ttl_hours": 12,
                    "dedupe_window_hours": 24,
                    "backpressure_drop_below": drop_below,
                    "escalate_levels": ["critical"],
                    "priority_map": {
                        "critical": 100,
                        "warn": 60,
                        "info": 20
                    }
                }
            }
        });
        let path = root.join("config").join("mech_suit_mode_policy.json");
        if let Some(parent) = path.parent() {
            let _ = fs::create_dir_all(parent);
        }
        write_json(&path, &policy);
    }

    fn enqueue_event(root: &Path, event: &Value) -> i32 {
        let payload = base64::engine::general_purpose::STANDARD
            .encode(serde_json::to_string(event).expect("encode event"));
        run(
            root,
            &[
                "enqueue".to_string(),
                format!("--event-json-base64={payload}"),
            ],
        )
    }

    #[test]
    fn enqueue_dedupes_within_window() {
        let dir = tempdir().expect("tempdir");
        write_policy(dir.path(), 32, "critical");
        let event = json!({
            "ts": now_iso(),
            "source": "external_eyes",
            "source_type": "external_item",
            "severity": "warn",
            "summary": "item one",
            "attention_key": "dup-key"
        });
        let payload = base64::engine::general_purpose::STANDARD
            .encode(serde_json::to_string(&event).expect("encode event"));
        let code_a = run(
            dir.path(),
            &[
                "enqueue".to_string(),
                format!("--event-json-base64={payload}"),
            ],
        );
        assert_eq!(code_a, 0);

        let code_b = run(
            dir.path(),
            &[
                "enqueue".to_string(),
                format!("--event-json-base64={payload}"),
            ],
        );
        assert_eq!(code_b, 0);

        let queue = read_jsonl(&dir.path().join("local/state/attention/queue.jsonl"));
        assert_eq!(queue.len(), 1);
    }

    #[test]
    fn enqueue_drops_info_on_backpressure() {
        let dir = tempdir().expect("tempdir");
        write_policy(dir.path(), 1, "critical");

        let warn_event = json!({
            "ts": now_iso(),
            "source": "external_eyes",
            "source_type": "eye_run_failed",
            "severity": "critical",
            "summary": "critical event",
            "attention_key": "first"
        });
        let warn_payload = base64::engine::general_purpose::STANDARD
            .encode(serde_json::to_string(&warn_event).expect("encode event"));
        let code_a = run(
            dir.path(),
            &[
                "enqueue".to_string(),
                format!("--event-json-base64={warn_payload}"),
            ],
        );
        assert_eq!(code_a, 0);

        let info_event = json!({
            "ts": now_iso(),
            "source": "external_eyes",
            "source_type": "external_item",
            "severity": "info",
            "summary": "informational event",
            "attention_key": "second"
        });
        let info_payload = base64::engine::general_purpose::STANDARD
            .encode(serde_json::to_string(&info_event).expect("encode event"));
        let code_b = run(
            dir.path(),
            &[
                "enqueue".to_string(),
                format!("--event-json-base64={info_payload}"),
            ],
        );
        assert_eq!(code_b, 2);

        let queue = read_jsonl(&dir.path().join("local/state/attention/queue.jsonl"));
        assert_eq!(queue.len(), 1);
        assert_eq!(
            queue[0].get("severity").and_then(Value::as_str),
            Some("critical")
        );
    }

    #[test]
    fn enqueue_receipt_reports_backpressure_thresholds_and_policy_action() {
        let dir = tempdir().expect("tempdir");
        write_policy(dir.path(), 1, "critical");

        let critical_event = json!({
            "ts": now_iso(),
            "source": "external_eyes",
            "source_type": "eye_run_failed",
            "severity": "critical",
            "summary": "critical event",
            "attention_key": "bp-threshold-critical"
        });
        assert_eq!(enqueue_event(dir.path(), &critical_event), 0);

        let info_event = json!({
            "ts": now_iso(),
            "source": "external_eyes",
            "source_type": "external_item",
            "severity": "info",
            "summary": "informational event",
            "attention_key": "bp-threshold-info"
        });
        assert_eq!(enqueue_event(dir.path(), &info_event), 2);

