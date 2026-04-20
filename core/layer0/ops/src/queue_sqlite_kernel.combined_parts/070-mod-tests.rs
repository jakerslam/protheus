
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn cfg(dir: &Path) -> SqliteCfg {
        SqliteCfg {
            db_path: dir.join("queue.sqlite"),
            journal_mode: "WAL".to_string(),
            synchronous: "NORMAL".to_string(),
            busy_timeout_ms: 5_000,
        }
    }

    #[test]
    fn queue_sqlite_kernel_round_trip() {
        let temp = tempdir().expect("tempdir");
        let cfg = cfg(temp.path());
        let conn = open_connection(&cfg).expect("open");
        ensure_schema(&conn).expect("schema");

        let row = json!({
            "id": "bl-1",
            "class": "memory",
            "wave": "w1",
            "status": "queued",
            "title": "Ship durable queue",
            "dependencies": ["BL-0"]
        });
        let upserted =
            upsert_item(&conn, "backlog_queue_executor", &row, Some("queued")).expect("upsert");
        assert_eq!(
            upserted.get("lane_id").and_then(Value::as_str),
            Some("BL-1")
        );

        let event = append_event(
            &conn,
            "backlog_queue_executor",
            "BL-1",
            "queued",
            &json!({ "detail": "scheduled" }),
            None,
        )
        .expect("append event");
        assert!(event.get("event_id").and_then(Value::as_str).is_some());

        let receipt = insert_receipt(&conn, "BL-1", &json!({ "ok": true })).expect("receipt");
        assert!(receipt.get("receipt_id").and_then(Value::as_str).is_some());

        let stats = queue_stats(&conn, "backlog_queue_executor").expect("stats");
        assert_eq!(stats.get("items").and_then(Value::as_i64), Some(1));
        assert_eq!(stats.get("events").and_then(Value::as_i64), Some(1));
        assert_eq!(stats.get("receipts").and_then(Value::as_i64), Some(1));
    }

    #[test]
    fn queue_sqlite_kernel_migrates_history_once() {
        let temp = tempdir().expect("tempdir");
        let history_path = temp.path().join("history.jsonl");
        fs::write(
            &history_path,
            format!(
                "{}\n{}\n",
                json!({ "lane_id": "bl-1", "action": "queued", "ts": "2026-03-17T00:00:00Z" }),
                json!({ "lane_id": "bl-1", "action": "started", "ts": "2026-03-17T00:01:00Z" })
            ),
        )
        .expect("history");

        let cfg = cfg(temp.path());
        let mut conn = open_connection(&cfg).expect("open");
        let first =
            migrate_history(&mut conn, &history_path, "backlog_queue_executor").expect("first");
        assert_eq!(first.get("rows_migrated").and_then(Value::as_u64), Some(2));

        let second =
            migrate_history(&mut conn, &history_path, "backlog_queue_executor").expect("second");
        assert_eq!(second.get("skipped").and_then(Value::as_bool), Some(true));
        assert_eq!(
            second.get("reason").and_then(Value::as_str),
            Some("already_applied")
        );
    }

    #[test]
    fn queue_sqlite_kernel_backpressure_policy_is_deterministic() {
        let temp = tempdir().expect("tempdir");
        let cfg = cfg(temp.path());
        let conn = open_connection(&cfg).expect("open");
        ensure_schema(&conn).expect("schema");

        for idx in 0..5 {
            let row = json!({
                "id": format!("bl-{}", idx + 1),
                "status": "queued",
                "title": format!("Item {}", idx + 1),
                "dependencies": []
            });
            upsert_item(&conn, "ops_queue", &row, Some("queued")).expect("upsert item");
        }

        let mut payload = Map::new();
        payload.insert("soft_watermark".to_string(), json!(2));
        payload.insert("hard_watermark".to_string(), json!(4));
        payload.insert("quarantine_watermark".to_string(), json!(6));
        payload.insert("incoming_priority".to_string(), json!("low"));
        let decision = backpressure_policy(&conn, "ops_queue", &payload).expect("policy");
        assert_eq!(
            decision.get("pressure_state").and_then(Value::as_str),
            Some("shed")
        );
        assert_eq!(
            decision.get("incoming_decision").and_then(Value::as_str),
            Some("drop")
        );
    }
}
