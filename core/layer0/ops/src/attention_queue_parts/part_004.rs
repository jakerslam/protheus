
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
    fn enqueue_orders_queue_by_band_then_priority_then_score() {
        let dir = tempdir().expect("tempdir");
        write_policy(dir.path(), 64, "critical");
        let low = json!({
            "ts": now_iso(),
            "source": "external_eyes",
            "source_type": "external_item",
            "severity": "info",
            "summary": "low priority item",
            "attention_key": "prio-low"
        });
        let mid = json!({
            "ts": now_iso(),
            "source": "memory_ambient",
            "source_type": "memory_event",
            "severity": "warn",
            "summary": "mid priority item",
            "attention_key": "prio-mid",
            "importance": {
                "score": 0.74
            }
        });
        let high = json!({
            "ts": now_iso(),
            "source": "spine",
            "source_type": "infra_outage_state",
            "severity": "critical",
            "summary": "conduit bridge timeout degraded",
            "attention_key": "prio-high"
        });
        assert_eq!(enqueue_event(dir.path(), &low), 0);
        assert_eq!(enqueue_event(dir.path(), &mid), 0);
        assert_eq!(enqueue_event(dir.path(), &high), 0);

        let queue = read_jsonl(&dir.path().join("local/state/attention/queue.jsonl"));
        assert_eq!(queue.len(), 3);
        assert_eq!(
            queue[0].get("attention_key").and_then(Value::as_str),
            Some("prio-high")
        );
        assert_eq!(
            queue[1].get("attention_key").and_then(Value::as_str),
            Some("prio-mid")
        );
        assert_eq!(
            queue[2].get("attention_key").and_then(Value::as_str),
            Some("prio-low")
        );
    }

    #[test]
    fn enqueue_assigns_tiered_queue_lanes() {
        let dir = tempdir().expect("tempdir");
        write_policy(dir.path(), 64, "critical");
        let background = json!({
            "ts": now_iso(),
            "source": "ops_logs",
            "source_type": "receipt_timeline",
            "severity": "info",
            "summary": "routine timeline heartbeat",
            "attention_key": "lane-background"
        });
        let critical = json!({
            "ts": now_iso(),
            "source": "security",
            "source_type": "integrity_fault",
            "severity": "critical",
            "summary": "critical policy failure",
            "attention_key": "lane-critical"
        });
        assert_eq!(enqueue_event(dir.path(), &background), 0);
        assert_eq!(enqueue_event(dir.path(), &critical), 0);
        let queue = read_jsonl(&dir.path().join("local/state/attention/queue.jsonl"));
        assert_eq!(queue.len(), 2);
        assert_eq!(
            queue[0].get("queue_lane").and_then(Value::as_str),
            Some("critical")
        );
        assert_eq!(
            queue[1].get("queue_lane").and_then(Value::as_str),
            Some("background")
        );
    }

    #[test]
    fn enqueue_attaches_importance_and_initiative_metadata() {
        let dir = tempdir().expect("tempdir");
        write_policy(dir.path(), 64, "critical");
        let event = json!({
            "ts": now_iso(),
            "source": "security",
            "source_type": "integrity_fault",
            "severity": "critical",
            "summary": "security_global_gate_failed conduit timeout",
            "attention_key": "importance-meta"
        });
        assert_eq!(enqueue_event(dir.path(), &event), 0);
        let queue = read_jsonl(&dir.path().join("local/state/attention/queue.jsonl"));
        assert_eq!(queue.len(), 1);
        let row = &queue[0];
        let score = row.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        assert!(score >= 0.95);
        assert_eq!(row.get("band").and_then(Value::as_str), Some("p0"));
        assert_eq!(
            row.get("initiative_action").and_then(Value::as_str),
            Some("persistent_until_ack")
        );
        assert!(row.get("importance").map(Value::is_object).unwrap_or(false));
    }

    #[test]
    fn next_ack_and_drain_progress_cursor() {
        let dir = tempdir().expect("tempdir");
        write_policy(dir.path(), 64, "critical");

        let first = json!({
            "ts": now_iso(),
            "source": "external_eyes",
            "source_type": "external_item",
            "severity": "warn",
            "summary": "first",
            "attention_key": "cursor-1"
        });
        let second = json!({
            "ts": now_iso(),
            "source": "external_eyes",
            "source_type": "eye_run_failed",
            "severity": "critical",
            "summary": "second",
            "attention_key": "cursor-2"
        });
        assert_eq!(enqueue_event(dir.path(), &first), 0);
        assert_eq!(enqueue_event(dir.path(), &second), 0);

        let next_code = run(
            dir.path(),
            &[
                "next".to_string(),
                "--consumer=cockpit".to_string(),
                "--limit=1".to_string(),
            ],
        );
        assert_eq!(next_code, 0);

        let contract = load_contract(dir.path());
        let queue = read_jsonl(&contract.queue_path);
        assert_eq!(queue.len(), 2);
        let token = cursor_token_for_event(&contract, "cockpit", 0, &queue[0]);

        let ack_code = run(
            dir.path(),
            &[
                "ack".to_string(),
                "--consumer=cockpit".to_string(),
                "--through-index=0".to_string(),
                format!("--cursor-token={token}"),
            ],
        );
        assert_eq!(ack_code, 0);

        let cursor_state = load_cursor_state(&contract.cursor_state_path);
        assert_eq!(read_consumer_offset(&cursor_state, "cockpit"), 1);

        let drain_code = run(
            dir.path(),
            &[
                "drain".to_string(),
                "--consumer=cockpit".to_string(),
                "--limit=10".to_string(),
            ],
        );
        assert_eq!(drain_code, 0);

        let cursor_state_after = load_cursor_state(&contract.cursor_state_path);
        assert_eq!(read_consumer_offset(&cursor_state_after, "cockpit"), 2);
    }

    #[test]
    fn compact_trims_acked_prefix_and_rebases_offsets() {
        let dir = tempdir().expect("tempdir");
        write_policy(dir.path(), 64, "critical");

        for idx in 0..5 {
            let event = json!({
                "ts": now_iso(),
                "source": "dashboard_chat",
                "source_type": "chat_turn",
                "severity": "info",
                "summary": format!("compact-event-{idx}"),
                "attention_key": format!("compact-key-{idx}")
            });
            assert_eq!(enqueue_event(dir.path(), &event), 0);
        }

        let contract = load_contract(dir.path());
        let (rows, _) = load_active_queue(&contract);
        assert_eq!(rows.len(), 5);
        let token = cursor_token_for_event(&contract, "dashboard-cockpit", 3, &rows[3]);

        assert_eq!(
            run(
                dir.path(),
                &[
                    "ack".to_string(),
                    "--consumer=dashboard-cockpit".to_string(),
                    "--through-index=3".to_string(),
                    format!("--cursor-token={token}"),
                ],
            ),
            0
        );

        assert_eq!(
            run(
                dir.path(),
                &[
                    "compact".to_string(),
                    "--retain=1".to_string(),
                    "--min-acked=2".to_string(),
                ],
            ),
            0
        );

        let queue = read_jsonl(&dir.path().join("local/state/attention/queue.jsonl"));
        assert_eq!(queue.len(), 2);

        let cursor_state =
            load_cursor_state(&dir.path().join("local/state/attention/cursor_state.json"));
        assert_eq!(read_consumer_offset(&cursor_state, "dashboard-cockpit"), 1);
    }

    #[test]
    fn ack_rejects_bad_token() {
        let dir = tempdir().expect("tempdir");
        write_policy(dir.path(), 64, "critical");
        let event = json!({
            "ts": now_iso(),
            "source": "external_eyes",
            "source_type": "external_item",
            "severity": "warn",
            "summary": "first",
            "attention_key": "cursor-bad-token"
        });
        assert_eq!(enqueue_event(dir.path(), &event), 0);
        let code = run(
            dir.path(),
            &[
                "ack".to_string(),
                "--consumer=cockpit".to_string(),
                "--through-index=0".to_string(),
                "--cursor-token=invalid".to_string(),
            ],
        );
        assert_eq!(code, 2);

        let contract = load_contract(dir.path());
        let cursor_state = load_cursor_state(&contract.cursor_state_path);
        assert_eq!(read_consumer_offset(&cursor_state, "cockpit"), 0);
    }

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 32,
            .. ProptestConfig::default()
        })]

        #[test]
        fn queue_depth_never_exceeds_contract_limit(
            max_depth in 1usize..12,
            info_events in 1usize..64
        ) {
            let dir = tempdir().expect("tempdir");
            write_policy(dir.path(), max_depth, "critical");
            for idx in 0..info_events {
                let event = json!({
                    "ts": now_iso(),
                    "source": "proptest",
                    "source_type": "queue_depth",
                    "severity": "info",
                    "summary": format!("queue-depth-{idx}"),
                    "attention_key": format!("prop-depth-{idx}")
                });
                let _ = enqueue_event(dir.path(), &event);
            }
            let queue = read_jsonl(&dir.path().join("local/state/attention/queue.jsonl"));
            prop_assert!(queue.len() <= max_depth);
        }

        #[test]
        fn critical_event_is_ordered_at_front_under_mixed_backlog(
            lead_count in 1usize..20,
            noisy in prop::collection::vec(prop_oneof![Just("info"), Just("warn")], 1..32)
        ) {
            let dir = tempdir().expect("tempdir");
            write_policy(dir.path(), 64, "critical");

            for idx in 0..lead_count {
                let sev = *noisy.get(idx % noisy.len()).expect("noise severity");
                let event = json!({
                    "ts": now_iso(),
                    "source": "proptest",
                    "source_type": "mixed_backlog",
                    "severity": sev,
                    "summary": format!("noise-{idx}"),
                    "attention_key": format!("noise-{idx}")
                });
                assert_eq!(enqueue_event(dir.path(), &event), 0);
            }

            let critical = json!({
                "ts": now_iso(),
                "source": "spine",
                "source_type": "infra_outage_state",
                "severity": "critical",
                "summary": "critical outage signal",
                "attention_key": "prop-critical-front"
            });
            assert_eq!(enqueue_event(dir.path(), &critical), 0);

            let queue = read_jsonl(&dir.path().join("local/state/attention/queue.jsonl"));
            prop_assert!(!queue.is_empty());
            prop_assert_eq!(
                queue[0].get("severity").and_then(Value::as_str),
                Some("critical")
            );
        }
    }
}


