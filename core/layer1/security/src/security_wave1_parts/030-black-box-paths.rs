
fn strip_invisible_path_chars(raw: &str) -> String {
    raw.chars()
        .filter(|ch| {
            !ch.is_control()
                && !matches!(
                    ch,
                    '\u{200B}'
                        | '\u{200C}'
                        | '\u{200D}'
                        | '\u{200E}'
                        | '\u{200F}'
                        | '\u{202A}'
                        | '\u{202B}'
                        | '\u{202C}'
                        | '\u{202D}'
                        | '\u{202E}'
                        | '\u{2060}'
                        | '\u{FEFF}'
                )
        })
        .collect::<String>()
}

fn resolve_runtime_subdir_override(
    raw: Option<String>,
    runtime_root: &Path,
    fallback: PathBuf,
) -> PathBuf {
    let Some(raw_value) = raw else { return fallback };
    let cleaned = strip_invisible_path_chars(raw_value.trim()).trim().to_string();
    if cleaned.is_empty() || cleaned.contains("://") {
        return fallback;
    }
    let candidate = PathBuf::from(cleaned);
    if candidate
        .components()
        .any(|part| matches!(part, std::path::Component::ParentDir))
    {
        return fallback;
    }
    if candidate.is_absolute() {
        if candidate.starts_with(runtime_root) {
            candidate
        } else {
            fallback
        }
    } else {
        runtime_root.join(candidate)
    }
}

fn black_box_paths(repo_root: &Path) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let runtime_root = runtime_state_root(repo_root);
    let ledger_fallback = runtime_root.join("security").join("black_box_ledger");
    let ledger_dir = resolve_runtime_subdir_override(
        std::env::var("BLACK_BOX_LEDGER_DIR").ok(),
        &runtime_root,
        ledger_fallback.clone(),
    );
    let spine_runs = resolve_runtime_subdir_override(
        std::env::var("BLACK_BOX_SPINE_RUNS_DIR").ok(),
        &runtime_root,
        runtime_root.join("spine").join("runs"),
    );
    let autonomy_runs = resolve_runtime_subdir_override(
        std::env::var("BLACK_BOX_AUTONOMY_RUNS_DIR").ok(),
        &runtime_root,
        runtime_root.join("autonomy").join("runs"),
    );
    let attest_dir = resolve_runtime_subdir_override(
        std::env::var("BLACK_BOX_EXTERNAL_ATTESTATION_DIR").ok(),
        &runtime_root,
        ledger_dir.join("attestations"),
    );
    (ledger_dir, spine_runs, autonomy_runs, attest_dir)
}

fn date_arg_or_today(v: Option<&String>) -> String {
    if let Some(raw) = v {
        let txt = clean_text(raw, 32);
        if txt.chars().count() == 10
            && txt.chars().nth(4) == Some('-')
            && txt.chars().nth(7) == Some('-')
        {
            return txt;
        }
    }
    now_iso().chars().take(10).collect::<String>()
}

fn allowed_spine_type(v: &str) -> bool {
    v == "spine_run_started"
        || v == "spine_run_completed"
        || v.contains("spine_trit_shadow")
        || v.contains("spine_alignment_oracle")
        || v.contains("spine_suggestion_lane")
        || v.contains("spine_self_documentation")
        || v.contains("spine_router_budget_calibration")
        || v.contains("spine_ops_dashboard")
        || v.contains("spine_integrity")
        || v.contains("spine_state_backup")
        || v.contains("spine_backup_integrity")
}

fn allowed_autonomy_type(v: &str) -> bool {
    v == "autonomy_run" || v == "autonomy_candidate_audit"
}

fn allowed_attestation_type(v: &str) -> bool {
    let lower = v.to_ascii_lowercase();
    matches!(
        lower.as_str(),
        "external_boundary_attestation"
            | "boundary_attestation"
            | "cross_runtime_attestation"
            | "cross_service_attestation"
    )
}

fn compact_event(row: &Value, source: &str) -> Value {
    json!({
        "ts": row.get("ts").and_then(Value::as_str).map(|v| clean_text(v, 64)),
        "source": source,
        "type": row.get("type").and_then(Value::as_str).map(|v| clean_text(v, 120)),
        "proposal_id": row.get("proposal_id").and_then(Value::as_str).map(|v| clean_text(v, 120)),
        "result": row.get("result").and_then(Value::as_str).map(|v| clean_text(v, 120)),
        "outcome": row.get("outcome").and_then(Value::as_str).map(|v| clean_text(v, 120)),
        "objective_id": row
            .get("objective_id")
            .or_else(|| row.get("directive_pulse").and_then(|v| v.get("objective_id")))
            .or_else(|| row.get("objective_binding").and_then(|v| v.get("objective_id")))
            .and_then(Value::as_str)
            .map(|v| clean_text(v, 120)),
        "risk": row.get("risk").and_then(Value::as_str).map(|v| clean_text(v, 80)),
        "ok": row.get("ok").and_then(Value::as_bool),
        "reason": row.get("reason").and_then(Value::as_str).map(|v| clean_text(v, 220))
    })
}

fn compact_attestation(row: &Value) -> Value {
    json!({
        "ts": row.get("ts").or_else(|| row.get("timestamp")).and_then(Value::as_str).map(|v| clean_text(v, 64)),
        "source": "boundary_attestation",
        "type": "external_boundary_attestation",
        "proposal_id": Value::Null,
        "result": Value::Null,
        "outcome": Value::Null,
        "objective_id": row.get("objective_id").and_then(Value::as_str).map(|v| clean_text(v, 120)),
        "risk": Value::Null,
        "ok": row.get("ok").and_then(Value::as_bool),
        "reason": row.get("boundary").or_else(|| row.get("scope")).and_then(Value::as_str).map(|v| clean_text(v, 120)),
        "external_attestation": {
            "system": row.get("system").or_else(|| row.get("source_system")).or_else(|| row.get("attestor")).and_then(Value::as_str).map(|v| clean_text(v, 120)),
            "boundary": row.get("boundary").or_else(|| row.get("scope")).and_then(Value::as_str).map(|v| clean_text(v, 120)),
            "chain_hash": row.get("chain_hash").or_else(|| row.get("receipt_hash")).or_else(|| row.get("hash")).and_then(Value::as_str).map(|v| clean_text(v, 180)),
            "signature": row.get("signature").or_else(|| row.get("sig")).and_then(Value::as_str).map(|v| clean_text(v, 220)),
            "signer": row.get("signer").or_else(|| row.get("attestor")).and_then(Value::as_str).map(|v| clean_text(v, 120))
        }
    })
}

fn load_critical_events(
    date: &str,
    spine_dir: &Path,
    autonomy_dir: &Path,
    attest_dir: &Path,
) -> (Vec<Value>, usize, usize, usize) {
    let spine_rows = read_jsonl(&spine_dir.join(format!("{date}.jsonl")))
        .into_iter()
        .filter(|row| {
            row.get("type")
                .and_then(Value::as_str)
                .map(allowed_spine_type)
                .unwrap_or(false)
        })
        .map(|row| compact_event(&row, "spine"))
        .collect::<Vec<_>>();
    let autonomy_rows = read_jsonl(&autonomy_dir.join(format!("{date}.jsonl")))
        .into_iter()
        .filter(|row| {
            row.get("type")
                .and_then(Value::as_str)
                .map(allowed_autonomy_type)
                .unwrap_or(false)
        })
        .map(|row| compact_event(&row, "autonomy"))
        .collect::<Vec<_>>();
    let att_rows = read_jsonl(&attest_dir.join(format!("{date}.jsonl")))
        .into_iter()
        .filter(|row| {
            row.get("type")
                .and_then(Value::as_str)
                .map(allowed_attestation_type)
                .unwrap_or(false)
        })
        .map(|row| compact_attestation(&row))
        .filter(|row| {
            row.get("external_attestation")
                .and_then(|v| v.get("chain_hash"))
                .and_then(Value::as_str)
                .map(|v| !v.is_empty())
                .unwrap_or(false)
        })
        .collect::<Vec<_>>();

    let mut all = Vec::new();
    all.extend(spine_rows.clone());
    all.extend(autonomy_rows.clone());
    all.extend(att_rows.clone());
    all.sort_by(|a, b| {
        let ta = a
            .get("ts")
            .and_then(Value::as_str)
            .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
            .map(|v| v.timestamp_millis())
            .unwrap_or(0);
        let tb = b
            .get("ts")
            .and_then(Value::as_str)
            .and_then(|v| chrono::DateTime::parse_from_rfc3339(v).ok())
            .map(|v| v.timestamp_millis())
            .unwrap_or(0);
        ta.cmp(&tb)
    });
    (all, spine_rows.len(), autonomy_rows.len(), att_rows.len())
}

fn black_box_chain_path(ledger_dir: &Path) -> PathBuf {
    ledger_dir.join("chain.jsonl")
}

fn black_box_detail_path(ledger_dir: &Path, date: &str, seq: usize) -> PathBuf {
    if seq <= 1 {
        ledger_dir.join(format!("{date}.jsonl"))
    } else {
        ledger_dir.join(format!("{date}.{seq}.jsonl"))
    }
}

fn next_rollup_seq(chain_rows: &[Value], date: &str) -> usize {
    let mut max_seq = 0usize;
    for row in chain_rows {
        if row.get("date").and_then(Value::as_str).unwrap_or("") != date {
            continue;
        }
        let seq = row.get("rollup_seq").and_then(Value::as_u64).unwrap_or(1) as usize;
        max_seq = max_seq.max(seq);
    }
    max_seq.saturating_add(1)
}

fn write_jsonl_rows(path: &Path, rows: &[Value]) -> Result<(), String> {
    ensure_parent(path)?;
    let mut body = String::new();
    for row in rows {
        let encoded = serde_json::to_string(row)
            .map_err(|err| format!("encode_jsonl_failed:{}:{err}", path.display()))?;
        body.push_str(&encoded);
        body.push('\n');
    }
    fs::write(path, body).map_err(|err| format!("write_jsonl_failed:{}:{err}", path.display()))
}

fn black_box_write_detail(
    date: &str,
    events: &[Value],
    detail_path: &Path,
) -> Result<(Vec<Value>, String), String> {
    let mut rows = Vec::new();
    let mut prev_hash = "GENESIS".to_string();
    for (idx, event) in events.iter().enumerate() {
        let payload = json!({
            "schema_id": "black_box_event",
            "schema_version": "1.0.0",
            "date": date,
            "index": idx,
            "event": event,
            "prev_hash": prev_hash
        });
        let hash = sha256_hex(&stable_json_string(&payload));
        let mut row = payload;
        row["hash"] = Value::String(hash.clone());
        prev_hash = hash;
        rows.push(row);
    }
    write_jsonl_rows(detail_path, &rows)?;
    let digest = rows
        .last()
        .and_then(|row| row.get("hash"))
        .and_then(Value::as_str)
        .map(|v| v.to_string())
        .unwrap_or_else(|| sha256_hex(&stable_json_string(&json!({"date": date, "empty": true}))));
    Ok((rows, digest))
}

fn black_box_sqlite_path(ledger_dir: &Path) -> PathBuf {
    ledger_dir.join("ledger.sqlite")
}

fn black_box_export_path(ledger_dir: &Path) -> PathBuf {
    ledger_dir.join("export_latest.json")
}

fn black_box_key_path(ledger_dir: &Path) -> PathBuf {
    ledger_dir.join("encryption_key.hex")
}

fn random_bytes(len: usize) -> Result<Vec<u8>, String> {
    let mut out = vec![0u8; len];
    match fs::File::open("/dev/urandom") {
        Ok(mut file) => file
            .read_exact(&mut out)
            .map_err(|err| format!("urandom_read_failed:{err}"))?,
        Err(_) => {
            let fallback = sha256_hex(&format!("{}:{}:{}", now_iso(), std::process::id(), len));
            let decoded = hex::decode(fallback)
                .map_err(|err| format!("fallback_random_decode_failed:{err}"))?;
            for (idx, byte) in out.iter_mut().enumerate() {
                *byte = decoded[idx % decoded.len()];
            }
        }
    }
    Ok(out)
}

fn black_box_key(ledger_dir: &Path) -> Result<Vec<u8>, String> {
    if let Ok(raw) = std::env::var("BLACK_BOX_LEDGER_KEY_HEX") {
        let key = hex::decode(raw.trim()).map_err(|err| format!("ledger_key_hex_invalid:{err}"))?;
        if key.len() != 32 {
            return Err("ledger_key_hex_must_be_32_bytes".to_string());
        }
        return Ok(key);
    }

    let key_path = black_box_key_path(ledger_dir);
    if let Ok(raw) = fs::read_to_string(&key_path) {
        let key =
            hex::decode(raw.trim()).map_err(|err| format!("ledger_key_file_invalid:{err}"))?;
        if key.len() != 32 {
            return Err("ledger_key_file_must_be_32_bytes".to_string());
        }
        return Ok(key);
    }

    let key = random_bytes(32)?;
    ensure_parent(&key_path)?;
    fs::write(&key_path, format!("{}\n", hex::encode(&key)))
        .map_err(|err| format!("ledger_key_write_failed:{}:{err}", key_path.display()))?;
    Ok(key)
}

fn black_box_open_db(ledger_dir: &Path) -> Result<Connection, String> {
    fs::create_dir_all(ledger_dir).map_err(|err| {
        format!(
            "black_box_ledger_dir_create_failed:{}:{err}",
            ledger_dir.display()
        )
    })?;
    let db_path = black_box_sqlite_path(ledger_dir);
    let conn =
        Connection::open(&db_path).map_err(|err| format!("black_box_sqlite_open_failed:{err}"))?;
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS ledger_entries(
            seq INTEGER PRIMARY KEY AUTOINCREMENT,
            ts TEXT NOT NULL,
            actor TEXT NOT NULL,
            action TEXT NOT NULL,
            source TEXT NOT NULL,
            prev_hash TEXT NOT NULL,
            entry_hash TEXT NOT NULL UNIQUE,
            signature TEXT NOT NULL,
            details_nonce BLOB NOT NULL,
            details_ciphertext BLOB NOT NULL
        );
        CREATE TABLE IF NOT EXISTS published_roots(
            root_seq INTEGER PRIMARY KEY,
            root_hash TEXT NOT NULL,
            published_at TEXT NOT NULL
        );",
    )
    .map_err(|err| format!("black_box_sqlite_schema_failed:{err}"))?;
    Ok(conn)
}

fn encrypt_ledger_details(key: &[u8], details: &Value) -> Result<(Vec<u8>, Vec<u8>), String> {
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|err| format!("ledger_cipher_init_failed:{err}"))?;
    let nonce = random_bytes(12)?;
    let plaintext = stable_json_string(details);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce), plaintext.as_bytes())
        .map_err(|err| format!("ledger_encrypt_failed:{err}"))?;
    Ok((nonce, ciphertext))
}

fn decrypt_ledger_details(key: &[u8], nonce: &[u8], ciphertext: &[u8]) -> Result<Value, String> {
    let cipher =
        Aes256Gcm::new_from_slice(key).map_err(|err| format!("ledger_cipher_init_failed:{err}"))?;
    let plaintext = cipher
        .decrypt(Nonce::from_slice(nonce), ciphertext)
        .map_err(|err| format!("ledger_decrypt_failed:{err}"))?;
    serde_json::from_slice::<Value>(&plaintext)
        .map_err(|err| format!("ledger_details_json_invalid:{err}"))
}
