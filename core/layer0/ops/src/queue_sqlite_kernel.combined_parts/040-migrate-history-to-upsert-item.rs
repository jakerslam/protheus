
fn migrate_history(
    conn: &mut Connection,
    history_path: &Path,
    queue_name: &str,

) -> Result<Value, String> {
    ensure_schema(conn)?;
    let migration_id = format!(
        "jsonl_history_to_sqlite:{}",
        history_path
            .canonicalize()
            .unwrap_or_else(|_| history_path.to_path_buf())
            .display()
    );
    if !history_path.exists() {
        return Ok(json!({
            "ok": true,
            "applied": false,
            "skipped": true,
            "reason": "history_path_missing",
            "rows_migrated": 0,
            "migration_id": migration_id
        }));
    }
    if migration_already_applied(conn, &migration_id)? {
        return Ok(json!({
            "ok": true,
            "applied": false,
            "skipped": true,
            "reason": "already_applied",
            "rows_migrated": 0,
            "migration_id": migration_id
        }));
    }

    let rows = read_jsonl_rows(history_path);
    if rows.is_empty() {
        mark_migration_applied(
            conn,
            &migration_id,
            &json!({ "source_path": history_path.to_string_lossy(), "rows_migrated": 0 }),
        )?;
        return Ok(json!({
            "ok": true,
            "applied": true,
            "skipped": false,
            "reason": "empty_source",
            "rows_migrated": 0,
            "migration_id": migration_id
        }));
    }

    let tx = conn
        .transaction()
        .map_err(|err| format!("queue_sqlite_kernel_tx_failed:{err}"))?;
    let mut migrated = 0u64;
    {
        let mut insert = tx
            .prepare(
                "INSERT OR IGNORE INTO backlog_queue_events (event_id, queue_name, lane_id, event_type, payload_json, ts) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .map_err(|err| format!("queue_sqlite_kernel_prepare_failed:{err}"))?;
        for row in &rows {
            let payload_json = canonical_json(row);
            let event_id = sha256_hex(&payload_json);
            let lane_id = clean_lane_id(&clean_text(
                row.get("lane_id").or_else(|| row.get("id")),
                120,
            ));
            let event_type = clean_text(row.get("action"), 80);
            let ts = clean_text(row.get("ts").or_else(|| row.get("timestamp")), 120);
            let changes = insert
                .execute(params![
                    event_id,
                    normalize_queue_name(queue_name),
                    if lane_id.is_empty() {
                        None::<String>
                    } else {
                        Some(lane_id)
                    },
                    if event_type.is_empty() {
                        "history_import".to_string()
                    } else {
                        event_type
                    },
                    payload_json,
                    if ts.is_empty() { now_iso() } else { ts }
                ])
                .map_err(|err| format!("queue_sqlite_kernel_insert_failed:{err}"))?;
            if changes > 0 {
                migrated += changes as u64;
            }
        }
    }
    tx.commit()
        .map_err(|err| format!("queue_sqlite_kernel_tx_commit_failed:{err}"))?;
    mark_migration_applied(
        conn,
        &migration_id,
        &json!({
            "source_path": history_path.to_string_lossy(),
            "rows_seen": rows.len(),
            "rows_migrated": migrated
        }),
    )?;
    Ok(json!({
        "ok": true,
        "applied": true,
        "skipped": false,
        "reason": "ok",
        "rows_seen": rows.len(),
        "rows_migrated": migrated,
        "migration_id": migration_id
    }))
}

fn upsert_item(
    conn: &Connection,
    queue_name: &str,
    row: &Value,
    status: Option<&str>,
) -> Result<Value, String> {
    ensure_schema(conn)?;
    let lane_id = clean_lane_id(&clean_text(row.get("id"), 120));
    if lane_id.is_empty() {
        return Err("queue_sqlite_lane_id_missing".to_string());
    }
    let payload_json = canonical_json(row);
    let updated_at = now_iso();
    let dependencies = row
        .get("dependencies")
        .cloned()
        .filter(|value| value.is_array())
        .unwrap_or_else(|| json!([]));
    conn.execute(
        r#"
        INSERT INTO backlog_queue_items (
          lane_id, queue_name, class, wave, status, title, dependencies_json, payload_json, updated_at
        ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)
        ON CONFLICT(lane_id) DO UPDATE SET
          queue_name=excluded.queue_name,
          class=excluded.class,
          wave=excluded.wave,
          status=excluded.status,
          title=excluded.title,
          dependencies_json=excluded.dependencies_json,
          payload_json=excluded.payload_json,
          updated_at=excluded.updated_at
        "#,
        params![
            lane_id,
            normalize_queue_name(queue_name),
            {
                let value = clean_text(row.get("class"), 120);
                if value.is_empty() { None::<String> } else { Some(value) }
            },
            {
                let value = clean_text(row.get("wave"), 80);
                if value.is_empty() { None::<String> } else { Some(value) }
            },
            {
                let raw = status.unwrap_or_else(|| row.get("status").and_then(Value::as_str).unwrap_or("queued"));
                clean_text(Some(&Value::String(raw.to_string())), 40).to_ascii_lowercase()
            },
            {
                let value = clean_text(row.get("title"), 400);
                if value.is_empty() { None::<String> } else { Some(value) }
            },
            canonical_json(&dependencies),
            payload_json,
            updated_at
        ],
    )
    .map_err(|err| format!("queue_sqlite_kernel_insert_failed:{err}"))?;
    Ok(json!({
        "ok": true,
        "lane_id": clean_lane_id(&clean_text(row.get("id"), 120)),
        "updated_at": updated_at
    }))
}
