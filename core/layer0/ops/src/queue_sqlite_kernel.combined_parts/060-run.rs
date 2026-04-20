
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
        "backpressure-policy" => {
            let queue_name = clean_text(payload.get("queue_name"), 160);
            backpressure_policy(
                &conn,
                if queue_name.is_empty() {
                    "default_queue"
                } else {
                    &queue_name
                },
                payload,
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
