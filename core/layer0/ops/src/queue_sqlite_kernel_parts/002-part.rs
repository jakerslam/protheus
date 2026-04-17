pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|value| value.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "open".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload = match payload_json(&argv[1..]) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("queue_sqlite_kernel_error", &err));
            return 1;
        }
    };
    let payload = payload_obj(&payload);
    let cfg = match sqlite_cfg_from_payload(root, payload) {
        Ok(cfg) => cfg,
        Err(err) => {
            print_json_line(&cli_error("queue_sqlite_kernel_error", &err));
            return 1;
        }
    };
    let mut conn = match open_connection(&cfg) {
        Ok(conn) => conn,
        Err(err) => {
            print_json_line(&cli_error("queue_sqlite_kernel_error", &err));
            return 1;
        }
    };

    let result = match command.as_str() {
        "open" => Ok(json!({
            "ok": true,
            "sqlite_cfg": cfg_to_value(&cfg),
            "db_path": cfg.db_path.to_string_lossy(),
            "db_exists": cfg.db_path.exists()
        })),
        "ensure-schema" => ensure_schema(&conn).map(|_| {
            json!({
                "ok": true,
                "sqlite_cfg": cfg_to_value(&cfg),
                "schema_ready": true
            })
        }),
        "migrate-history" => {
            let history_path_raw = clean_text(payload.get("history_path"), 520);
            if history_path_raw.is_empty() {
                Err("queue_sqlite_history_path_required".to_string())
            } else {
                let history_path = {
                    let candidate = PathBuf::from(&history_path_raw);
                    if candidate.is_absolute() {
                        candidate
                    } else {
                        root.join(candidate)
                    }
                };
                let queue_name = clean_text(payload.get("queue_name"), 160);
                migrate_history(
                    &mut conn,
                    &history_path,
                    if queue_name.is_empty() {
                        "backlog_queue_executor"
                    } else {
                        &queue_name
                    },
                )
            }
        }
        "upsert-item" => {
            let row = payload
                .get("row")
                .cloned()
                .unwrap_or_else(|| Value::Object(Map::new()));
            let queue_name = clean_text(payload.get("queue_name"), 160);
            let status = clean_text(payload.get("status"), 40);
            upsert_item(
                &conn,
                if queue_name.is_empty() {
                    "default_queue"
                } else {
                    &queue_name
                },
                &row,
                if status.is_empty() {
                    None
                } else {
                    Some(status.as_str())
                },
            )
        }
        "append-event" => {
            let queue_name = clean_text(payload.get("queue_name"), 160);
            let lane_id = clean_text(payload.get("lane_id"), 120);
            let event_type = clean_text(payload.get("event_type"), 80);
            let event_payload = payload
                .get("payload")
                .cloned()
                .unwrap_or_else(|| Value::Object(Map::new()));
            let ts = clean_text(payload.get("ts"), 120);
            append_event(
                &conn,
                if queue_name.is_empty() {
                    "default_queue"
                } else {
                    &queue_name
                },
                &lane_id,
                &event_type,
                &event_payload,
                if ts.is_empty() {
                    None
                } else {
                    Some(ts.as_str())
                },
            )
        }
        "insert-receipt" => {
            let lane_id = clean_text(payload.get("lane_id"), 120);
            if lane_id.is_empty() {
                Err("queue_sqlite_lane_id_missing".to_string())
            } else {
                let receipt = payload
                    .get("receipt")
                    .cloned()
                    .unwrap_or_else(|| Value::Object(Map::new()));
                insert_receipt(&conn, &lane_id, &receipt)
            }
        }
        "queue-stats" => {
            let queue_name = clean_text(payload.get("queue_name"), 160);
            queue_stats(
                &conn,
                if queue_name.is_empty() {
                    "default_queue"
                } else {
                    &queue_name
                },
            )
        }
        _ => Err(format!("queue_sqlite_kernel_unknown_command:{command}")),
    };

    match result {
        Ok(payload) => {
            print_json_line(&cli_receipt(
                &format!("queue_sqlite_kernel_{}", command.replace('-', "_")),
                payload,
            ));
            0
        }
        Err(err) => {
            print_json_line(&cli_error(
                &format!("queue_sqlite_kernel_{}", command.replace('-', "_")),
                &err,
            ));
            1
        }
    }
}

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
}
