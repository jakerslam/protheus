
fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("queue_sqlite_kernel_create_dir_failed:{err}"))?;
    }
    Ok(())
}

fn open_connection(cfg: &SqliteCfg) -> Result<Connection, String> {
    ensure_parent(&cfg.db_path)?;
    let conn = Connection::open(&cfg.db_path)
        .map_err(|err| format!("queue_sqlite_kernel_open_failed:{err}"))?;
    conn.execute_batch(&format!(
        "PRAGMA busy_timeout={};PRAGMA journal_mode={};PRAGMA synchronous={};PRAGMA foreign_keys=ON;",
        cfg.busy_timeout_ms, cfg.journal_mode, cfg.synchronous
    ))
    .map_err(|err| format!("queue_sqlite_kernel_pragma_failed:{err}"))?;
    Ok(conn)
}

fn execute_batch_with_retry(conn: &Connection, sql: &str) -> Result<(), String> {
    let mut attempt = 0u32;
    loop {
        match conn.execute_batch(sql) {
            Ok(()) => return Ok(()),
            Err(err) => {
                let msg = err.to_string().to_ascii_lowercase();
                if !msg.contains("database is locked") || attempt >= 6 {
                    return Err(format!("queue_sqlite_kernel_exec_failed:{err}"));
                }
                thread::sleep(Duration::from_millis(20 * 2u64.pow(attempt)));
                attempt += 1;
            }
        }
    }
}

fn ensure_schema(conn: &Connection) -> Result<(), String> {
    execute_batch_with_retry(
        conn,
        r#"
        CREATE TABLE IF NOT EXISTS queue_schema_migrations (
          migration_id TEXT PRIMARY KEY,
          applied_at TEXT NOT NULL,
          detail_json TEXT NOT NULL
        );
        CREATE TABLE IF NOT EXISTS backlog_queue_items (
          lane_id TEXT PRIMARY KEY,
          queue_name TEXT NOT NULL,
          class TEXT,
          wave TEXT,
          status TEXT NOT NULL,
          title TEXT,
          dependencies_json TEXT NOT NULL,
          payload_json TEXT NOT NULL,
          updated_at TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_backlog_queue_lookup
          ON backlog_queue_items(queue_name, status, updated_at DESC);
        CREATE TABLE IF NOT EXISTS backlog_queue_events (
          event_id TEXT PRIMARY KEY,
          queue_name TEXT NOT NULL,
          lane_id TEXT,
          event_type TEXT NOT NULL,
          payload_json TEXT NOT NULL,
          ts TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_backlog_queue_events_lookup
          ON backlog_queue_events(queue_name, ts DESC);
        CREATE TABLE IF NOT EXISTS backlog_queue_receipts (
          receipt_id TEXT PRIMARY KEY,
          lane_id TEXT NOT NULL,
          receipt_json TEXT NOT NULL,
          ts TEXT NOT NULL
        );
        CREATE INDEX IF NOT EXISTS idx_backlog_queue_receipts_lane
          ON backlog_queue_receipts(lane_id, ts DESC);
        "#,
    )
}

fn migration_already_applied(conn: &Connection, migration_id: &str) -> Result<bool, String> {
    let mut stmt = conn
        .prepare("SELECT migration_id FROM queue_schema_migrations WHERE migration_id = ?1 LIMIT 1")
        .map_err(|err| format!("queue_sqlite_kernel_prepare_failed:{err}"))?;
    let mut rows = stmt
        .query(params![migration_id])
        .map_err(|err| format!("queue_sqlite_kernel_query_failed:{err}"))?;
    rows.next()
        .map_err(|err| format!("queue_sqlite_kernel_query_failed:{err}"))
        .map(|row| row.is_some())
}

fn mark_migration_applied(
    conn: &Connection,
    migration_id: &str,
    detail: &Value,
) -> Result<(), String> {
    conn.execute(
        "INSERT OR REPLACE INTO queue_schema_migrations (migration_id, applied_at, detail_json) VALUES (?1, ?2, ?3)",
        params![migration_id, now_iso(), canonical_json(detail)],
    )
    .map_err(|err| format!("queue_sqlite_kernel_insert_failed:{err}"))?;
    Ok(())
}

fn read_jsonl_rows(path: &Path) -> Vec<Value> {
    let raw = match fs::read_to_string(path) {
        Ok(value) => value,
        Err(_) => return Vec::new(),
    };
    raw.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect()
}
