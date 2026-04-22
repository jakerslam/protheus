
#[derive(Clone, Debug)]
struct SqliteCfg {
    db_path: PathBuf,
    journal_mode: String,
    synchronous: String,
    busy_timeout_ms: u64,
}

fn usage() {
    println!("queue-sqlite-kernel commands:");
    println!(
        "  protheus-ops queue-sqlite-kernel <open|ensure-schema|migrate-history|upsert-item|append-event|insert-receipt|queue-stats|backpressure-policy> [--payload-base64=<base64_json>]"
    );
}

fn with_receipt_hash(mut value: Value) -> Value {
    value["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&value));
    value
}

fn cli_receipt(kind: &str, payload: Value) -> Value {
    crate::contract_lane_utils::cli_receipt(kind, payload)
}

fn cli_error(kind: &str, error: &str) -> Value {
    crate::contract_lane_utils::cli_error(kind, error)
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn payload_json(argv: &[String]) -> Result<Value, String> {
    if let Some(raw) = lane_utils::parse_flag(argv, "payload", false) {
        return serde_json::from_str::<Value>(&raw)
            .map_err(|err| format!("queue_sqlite_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("queue_sqlite_kernel_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("queue_sqlite_kernel_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("queue_sqlite_kernel_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn as_string(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(v)) => v.trim().to_string(),
        Some(Value::Null) | None => String::new(),
        Some(v) => v.to_string().trim_matches('"').trim().to_string(),
    }
}

fn as_i64(value: Option<&Value>, fallback: i64) -> i64 {
    match value {
        Some(Value::Number(v)) => v.as_i64().unwrap_or(fallback),
        Some(Value::String(v)) => v.trim().parse::<i64>().unwrap_or(fallback),
        _ => fallback,
    }
}

fn clean_text(value: Option<&Value>, max_len: usize) -> String {
    let mut out = as_string(value)
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ");
    if out.len() > max_len {
        out.truncate(max_len);
    }
    out
}

fn canonical_json(value: &Value) -> String {
    match value {
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_) => {
            serde_json::to_string(value).unwrap_or_else(|_| "null".to_string())
        }
        Value::Array(items) => {
            let mut out = String::from("[");
            for (idx, item) in items.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                out.push_str(&canonical_json(item));
            }
            out.push(']');
            out
        }
        Value::Object(map) => {
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            let mut out = String::from("{");
            for (idx, key) in keys.iter().enumerate() {
                if idx > 0 {
                    out.push(',');
                }
                out.push_str(&serde_json::to_string(key).unwrap_or_else(|_| "\"\"".to_string()));
                out.push(':');
                out.push_str(&canonical_json(map.get(key).unwrap_or(&Value::Null)));
            }
            out.push('}');
            out
        }
    }
}

fn sha256_hex(text: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(text.as_bytes());
    hex::encode(hasher.finalize())
}

fn normalize_queue_name(raw: &str) -> String {
    let lowered = raw.trim().to_ascii_lowercase();
    let mut out = String::new();
    let mut prev_underscore = false;
    for ch in lowered.chars() {
        let mapped = if ch.is_ascii_alphanumeric() || matches!(ch, '.' | ':' | '-') {
            prev_underscore = false;
            ch
        } else if prev_underscore {
            continue;
        } else {
            prev_underscore = true;
            '_'
        };
        out.push(mapped);
    }
    let trimmed = out.trim_matches('_').to_string();
    if trimmed.is_empty() {
        "default_queue".to_string()
    } else {
        trimmed
    }
}

fn clean_lane_id(raw: &str) -> String {
    raw.trim().to_ascii_uppercase()
}

fn sanitize_journal_mode(raw: &str) -> String {
    match raw.trim().to_ascii_uppercase().as_str() {
        "DELETE" => "DELETE".to_string(),
        "TRUNCATE" => "TRUNCATE".to_string(),
        "PERSIST" => "PERSIST".to_string(),
        "MEMORY" => "MEMORY".to_string(),
        "WAL" => "WAL".to_string(),
        "OFF" => "OFF".to_string(),
        _ => "WAL".to_string(),
    }
}

fn sanitize_synchronous(raw: &str) -> String {
    match raw.trim().to_ascii_uppercase().as_str() {
        "OFF" => "OFF".to_string(),
        "NORMAL" => "NORMAL".to_string(),
        "FULL" => "FULL".to_string(),
        "EXTRA" => "EXTRA".to_string(),
        _ => "NORMAL".to_string(),
    }
}

fn sqlite_cfg_from_payload(root: &Path, payload: &Map<String, Value>) -> Result<SqliteCfg, String> {
    let source = payload
        .get("sqlite_cfg")
        .and_then(Value::as_object)
        .or_else(|| {
            payload
                .get("db")
                .and_then(Value::as_object)
                .and_then(|db| db.get("sqlite_cfg"))
                .and_then(Value::as_object)
        })
        .unwrap_or(payload);
    let db_path_raw = clean_text(source.get("db_path"), 520);
    if db_path_raw.is_empty() {
        return Err("queue_sqlite_db_path_required".to_string());
    }
    let db_path = {
        let candidate = PathBuf::from(&db_path_raw);
        if candidate.is_absolute() {
            candidate
        } else {
            root.join(candidate)
        }
    };
    Ok(SqliteCfg {
        db_path,
        journal_mode: sanitize_journal_mode(&clean_text(source.get("journal_mode"), 24)),
        synchronous: sanitize_synchronous(&clean_text(source.get("synchronous"), 24)),
        busy_timeout_ms: as_i64(source.get("busy_timeout_ms"), 5000).clamp(100, 120_000) as u64,
    })
}

fn cfg_to_value(cfg: &SqliteCfg) -> Value {
    json!({
        "db_path": cfg.db_path.to_string_lossy(),
        "journal_mode": cfg.journal_mode,
        "synchronous": cfg.synchronous,
        "busy_timeout_ms": cfg.busy_timeout_ms
    })
}
