
#[cfg(test)]
mod agent_ingress_tests {
    use super::*;
    use base64::Engine;
    use tempfile::tempdir;

    const SRS_ID: &str = "V12-ATTENTION-INGRESS-001";

    fn write_policy(root: &Path) {
        let policy = json!({
            "enabled": true,
            "eyes": {
                "push_attention_queue": true,
                "attention_queue_path": "local/state/attention/queue.jsonl",
                "receipts_path": "local/state/attention/receipts.jsonl",
                "latest_path": "local/state/attention/latest.json",
                "attention_contract": {
                    "max_queue_depth": 32,
                    "ttl_hours": 12,
                    "dedupe_window_hours": 24,
                    "backpressure_drop_below": "critical",
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
    fn agent_attention_ingress_drops_passive_memory_by_default() {
        assert_eq!(SRS_ID, "V12-ATTENTION-INGRESS-001");
        let dir = tempdir().expect("tempdir");
        write_policy(dir.path());
        let event = json!({
            "ts": now_iso(),
            "source": "agent:misty",
            "source_type": "passive_memory_turn",
            "severity": "info",
            "summary": "Misty chatted about an external framework",
            "attention_key": "agent:misty:passive:one"
        });

        assert_eq!(enqueue_event(dir.path(), &event), 0);
        let queue = read_jsonl(&dir.path().join("local/state/attention/queue.jsonl"));
        assert_eq!(queue.len(), 0);
        let receipts = read_jsonl(&dir.path().join("local/state/attention/receipts.jsonl"));
        let last = receipts.last().expect("ingress receipt");
        assert_eq!(
            last.get("decision").and_then(Value::as_str),
            Some("dropped_ingress_policy")
        );
        assert_eq!(
            last.get("attention_ingress_reason").and_then(Value::as_str),
            Some("agent_scoped_event_not_owned_actionable")
        );
    }

    #[test]
    fn agent_attention_ingress_allows_owned_actionable_events() {
        let dir = tempdir().expect("tempdir");
        write_policy(dir.path());
        for (source_type, key) in [
            ("tool_result", "agent:misty:tool:one"),
            ("eval_issue_feedback", "agent:misty:eval:one"),
            ("cron_job_output", "agent:misty:cron:one"),
        ] {
            let event = json!({
                "ts": now_iso(),
                "source": "agent:misty",
                "source_type": source_type,
                "severity": "info",
                "summary": format!("{source_type} for Misty"),
                "attention_key": key
            });
            assert_eq!(enqueue_event(dir.path(), &event), 0);
        }
        let queue = read_jsonl(&dir.path().join("local/state/attention/queue.jsonl"));
        assert_eq!(queue.len(), 3);
    }

    #[test]
    fn agent_attention_ingress_explicit_opt_in_allows_context_event() {
        let dir = tempdir().expect("tempdir");
        write_policy(dir.path());
        let event = json!({
            "ts": now_iso(),
            "source": "agent:misty",
            "source_type": "passive_memory_turn",
            "severity": "info",
            "summary": "explicitly visible memory event",
            "attention_key": "agent:misty:passive:visible",
            "attention_ingress": {
                "agent_visible": true
            }
        });

        assert_eq!(enqueue_event(dir.path(), &event), 0);
        let queue = read_jsonl(&dir.path().join("local/state/attention/queue.jsonl"));
        assert_eq!(queue.len(), 1);
        assert_eq!(
            queue[0].get("attention_key").and_then(Value::as_str),
            Some("agent:misty:passive:visible")
        );
    }
}
