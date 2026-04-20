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
