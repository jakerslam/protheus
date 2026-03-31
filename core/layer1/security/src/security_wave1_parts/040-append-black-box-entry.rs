
fn append_black_box_entry(repo_root: &Path, args: &CliArgs, ledger_dir: &Path) -> (Value, i32) {
    let actor = clean_text(
        args.flags
            .get("actor")
            .cloned()
            .unwrap_or_else(|| "unknown_actor".to_string()),
        120,
    );
    let action = clean_text(
        args.flags
            .get("action")
            .cloned()
            .unwrap_or_else(|| "unknown_action".to_string()),
        160,
    );
    let source = clean_text(
        args.flags
            .get("source")
            .cloned()
            .unwrap_or_else(|| "security_plane".to_string()),
        120,
    );
    let details = args
        .flags
        .get("details-json")
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .unwrap_or_else(|| json!({}));

    let conn = match black_box_open_db(ledger_dir) {
        Ok(conn) => conn,
        Err(err) => {
            return (
                json!({
                    "ok": false,
                    "type": "black_box_ledger_append",
                    "error": err
                }),
                1,
            )
        }
    };
    let key = match black_box_key(ledger_dir) {
        Ok(key) => key,
        Err(err) => {
            return (
                json!({
                    "ok": false,
                    "type": "black_box_ledger_append",
                    "error": err
                }),
                1,
            )
        }
    };
    let (nonce, ciphertext) = match encrypt_ledger_details(&key, &details) {
        Ok(v) => v,
        Err(err) => {
            return (
                json!({
                    "ok": false,
                    "type": "black_box_ledger_append",
                    "error": err
                }),
                1,
            )
        }
    };

    let prev_hash = conn
        .query_row(
            "SELECT entry_hash FROM ledger_entries ORDER BY seq DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .unwrap_or_else(|_| "GENESIS".to_string());
    let ts = now_iso();
    let ciphertext_digest = sha256_hex(&hex::encode(&ciphertext));
    let signature = sha256_hex(&format!(
        "{ts}|{actor}|{action}|{source}|{prev_hash}|{ciphertext_digest}"
    ));
    let entry_hash = sha256_hex(&stable_json_string(&json!({
        "ts": ts,
        "actor": actor,
        "action": action,
        "source": source,
        "prev_hash": prev_hash,
        "signature": signature,
        "ciphertext_digest": ciphertext_digest
    })));

    if let Err(err) = conn.execute(
        "INSERT INTO ledger_entries (ts, actor, action, source, prev_hash, entry_hash, signature, details_nonce, details_ciphertext)
         VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
        params![
            ts,
            actor,
            action,
            source,
            prev_hash,
            entry_hash,
            signature,
            nonce,
            ciphertext
        ],
    ) {
        return (
            json!({
                "ok": false,
                "type": "black_box_ledger_append",
                "error": format!("sqlite_insert_failed:{err}")
            }),
            1,
        );
    }

    let seq = conn.last_insert_rowid();
    let latest_published = conn
        .query_row(
            "SELECT published_at FROM published_roots ORDER BY root_seq DESC LIMIT 1",
            [],
            |row| row.get::<_, String>(0),
        )
        .ok();
    let should_publish = latest_published
        .as_deref()
        .and_then(|raw| chrono::DateTime::parse_from_rfc3339(raw).ok())
        .map(|dt| {
            Utc::now()
                .signed_duration_since(dt.with_timezone(&Utc))
                .num_seconds()
                >= 60
        })
        .unwrap_or(true);
    if should_publish {
        let root_hash = conn
            .query_row(
                "SELECT entry_hash FROM ledger_entries ORDER BY seq DESC LIMIT 1",
                [],
                |row| row.get::<_, String>(0),
            )
            .unwrap_or_else(|_| "GENESIS".to_string());
        let published_at = now_iso();
        let _ = conn.execute(
            "INSERT OR REPLACE INTO published_roots (root_seq, root_hash, published_at) VALUES (?1, ?2, ?3)",
            params![seq, root_hash, published_at],
        );
    }

    (
        json!({
            "ok": true,
            "type": "black_box_ledger_append",
            "seq": seq,
            "actor": args.flags.get("actor").cloned().unwrap_or_else(|| "unknown_actor".to_string()),
            "action": args.flags.get("action").cloned().unwrap_or_else(|| "unknown_action".to_string()),
            "entry_hash": conn.query_row("SELECT entry_hash FROM ledger_entries WHERE seq = ?1", params![seq], |row| row.get::<_, String>(0)).unwrap_or_default(),
            "sqlite_path": normalize_rel_path(black_box_sqlite_path(ledger_dir).to_string_lossy()),
            "encrypted_at_rest": true,
            "claim_evidence": [{
                "id": "V6-SEC-LEDGER-001",
                "claim": "black_box_ledger_appends_tamper_evident_encrypted_sqlite_entries_with_hash_chain_signatures_and_published_roots",
                "evidence": {
                    "lane": "append",
                    "runtime": "security_plane_black_box_ledger"
                }
            }],
            "receipt_hash": sha256_hex(&stable_json_string(&json!({
                "repo_root": normalize_rel_path(repo_root.display()),
                "seq": seq,
                "type": "black_box_ledger_append"
            }))),
        }),
        0,
    )
}

fn verify_black_box_sqlite(ledger_dir: &Path) -> Result<Value, String> {
    let conn = black_box_open_db(ledger_dir)?;
    let count: i64 = conn
        .query_row("SELECT COUNT(*) FROM ledger_entries", [], |row| row.get(0))
        .map_err(|err| format!("sqlite_count_failed:{err}"))?;
    if count == 0 {
        return Err("sqlite_chain_empty".to_string());
    }

    let mut stmt = conn
        .prepare(
            "SELECT seq, ts, actor, action, source, prev_hash, entry_hash, signature, details_nonce, details_ciphertext
             FROM ledger_entries ORDER BY seq ASC",
        )
        .map_err(|err| format!("sqlite_prepare_failed:{err}"))?;
    let mut rows = stmt
        .query([])
        .map_err(|err| format!("sqlite_query_failed:{err}"))?;
    let mut prev_hash = "GENESIS".to_string();
    let key = black_box_key(ledger_dir)?;
    let mut last_hash = "GENESIS".to_string();
    let mut last_seq = 0i64;
    while let Some(row) = rows
        .next()
        .map_err(|err| format!("sqlite_row_failed:{err}"))?
    {
        let seq: i64 = row
            .get(0)
            .map_err(|err| format!("sqlite_seq_failed:{err}"))?;
        let ts: String = row
            .get(1)
            .map_err(|err| format!("sqlite_ts_failed:{err}"))?;
        let actor: String = row
            .get(2)
            .map_err(|err| format!("sqlite_actor_failed:{err}"))?;
        let action: String = row
            .get(3)
            .map_err(|err| format!("sqlite_action_failed:{err}"))?;
        let source: String = row
            .get(4)
            .map_err(|err| format!("sqlite_source_failed:{err}"))?;
        let stored_prev: String = row
            .get(5)
            .map_err(|err| format!("sqlite_prev_failed:{err}"))?;
        let entry_hash: String = row
            .get(6)
            .map_err(|err| format!("sqlite_hash_failed:{err}"))?;
        let signature: String = row
            .get(7)
            .map_err(|err| format!("sqlite_signature_failed:{err}"))?;
        let nonce: Vec<u8> = row
            .get(8)
            .map_err(|err| format!("sqlite_nonce_failed:{err}"))?;
        let ciphertext: Vec<u8> = row
            .get(9)
            .map_err(|err| format!("sqlite_ciphertext_failed:{err}"))?;

        if stored_prev != prev_hash {
            return Err(format!("sqlite_prev_hash_mismatch:seq={seq}"));
        }
        let _details = decrypt_ledger_details(&key, &nonce, &ciphertext)?;
        let ciphertext_digest = sha256_hex(&hex::encode(&ciphertext));
        let calc_signature = sha256_hex(&format!(
            "{ts}|{actor}|{action}|{source}|{stored_prev}|{ciphertext_digest}"
        ));
        if calc_signature != signature {
            return Err(format!("sqlite_signature_mismatch:seq={seq}"));
        }
        let calc_hash = sha256_hex(&stable_json_string(&json!({
            "ts": ts,
            "actor": actor,
            "action": action,
            "source": source,
            "prev_hash": stored_prev,
            "signature": signature,
            "ciphertext_digest": ciphertext_digest
        })));
        if calc_hash != entry_hash {
            return Err(format!("sqlite_hash_mismatch:seq={seq}"));
        }
        prev_hash = entry_hash.clone();
        last_hash = entry_hash;
        last_seq = seq;
    }

    let published_root = conn
        .query_row(
            "SELECT root_seq, root_hash, published_at FROM published_roots ORDER BY root_seq DESC LIMIT 1",
            [],
            |row| {
                Ok(json!({
                    "root_seq": row.get::<_, i64>(0)?,
                    "root_hash": row.get::<_, String>(1)?,
                    "published_at": row.get::<_, String>(2)?,
                }))
            },
        )
        .ok();
    if let Some(root) = published_root.as_ref() {
        if root.get("root_hash").and_then(Value::as_str) != Some(last_hash.as_str()) {
            return Err("published_root_hash_mismatch".to_string());
        }
    }

    Ok(json!({
        "ok": true,
        "type": "black_box_ledger_verify",
        "valid": true,
        "sqlite_chain_length": count,
        "last_seq": last_seq,
        "last_hash": last_hash,
        "published_root": published_root,
    }))
}

fn export_black_box_sqlite(ledger_dir: &Path, export_path: &Path) -> (Value, i32) {
    let conn = match black_box_open_db(ledger_dir) {
        Ok(conn) => conn,
        Err(err) => {
            return (
                json!({
                    "ok": false,
                    "type": "black_box_ledger_export",
                    "error": err
                }),
                1,
            )
        }
    };
    let key = match black_box_key(ledger_dir) {
        Ok(key) => key,
        Err(err) => {
            return (
                json!({
                    "ok": false,
                    "type": "black_box_ledger_export",
                    "error": err
                }),
                1,
            )
        }
    };
    let mut stmt = match conn.prepare(
        "SELECT seq, ts, actor, action, source, prev_hash, entry_hash, signature, details_nonce, details_ciphertext
         FROM ledger_entries ORDER BY seq ASC",
    ) {
        Ok(stmt) => stmt,
        Err(err) => {
            return (
                json!({
                    "ok": false,
                    "type": "black_box_ledger_export",
                    "error": format!("sqlite_prepare_failed:{err}")
                }),
                1,
            )
        }
    };
    let mapped = match stmt.query_map([], |row| {
        Ok((
            row.get::<_, i64>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, String>(3)?,
            row.get::<_, String>(4)?,
            row.get::<_, String>(5)?,
            row.get::<_, String>(6)?,
            row.get::<_, String>(7)?,
            row.get::<_, Vec<u8>>(8)?,
            row.get::<_, Vec<u8>>(9)?,
        ))
    }) {
        Ok(rows) => rows,
        Err(err) => {
            return (
                json!({
                    "ok": false,
                    "type": "black_box_ledger_export",
                    "error": format!("sqlite_query_failed:{err}")
                }),
                1,
            )
        }
    };
    let mut entries = Vec::new();
    for row in mapped {
        let (seq, ts, actor, action, source, prev_hash, entry_hash, signature, nonce, ciphertext) =
            match row {
                Ok(v) => v,
                Err(err) => {
                    return (
                        json!({
                            "ok": false,
                            "type": "black_box_ledger_export",
                            "error": format!("sqlite_row_failed:{err}")
                        }),
                        1,
                    )
                }
            };
        let details = match decrypt_ledger_details(&key, &nonce, &ciphertext) {
            Ok(details) => details,
            Err(err) => {
                return (
                    json!({
                        "ok": false,
                        "type": "black_box_ledger_export",
                        "error": err
                    }),
                    1,
                )
            }
        };
        entries.push(json!({
            "seq": seq,
            "ts": ts,
            "actor": actor,
            "action": action,
            "source": source,
            "prev_hash": prev_hash,
            "entry_hash": entry_hash,
            "signature": signature,
            "ciphertext_digest": sha256_hex(&hex::encode(&ciphertext)),
            "details": details,
        }));
    }
    let published_roots = match conn.prepare(
        "SELECT root_seq, root_hash, published_at FROM published_roots ORDER BY root_seq ASC",
    ) {
        Ok(mut stmt) => match stmt.query_map([], |row| {
            Ok(json!({
                "root_seq": row.get::<_, i64>(0)?,
                "root_hash": row.get::<_, String>(1)?,
                "published_at": row.get::<_, String>(2)?,
            }))
        }) {
            Ok(rows) => rows.filter_map(Result::ok).collect::<Vec<_>>(),
            Err(_) => Vec::new(),
        },
        Err(_) => Vec::new(),
    };
    let export = json!({
        "type": "black_box_ledger_offline_export",
        "version": "v1",
        "generated_at": now_iso(),
        "entries": entries,
        "published_roots": published_roots,
    });
    match write_json_atomic(export_path, &export) {
        Ok(()) => (
            json!({
                "ok": true,
                "type": "black_box_ledger_export",
                "export_path": normalize_rel_path(export_path.to_string_lossy()),
                "entry_count": export.get("entries").and_then(Value::as_array).map(|rows| rows.len()).unwrap_or(0),
            }),
            0,
        ),
        Err(err) => (
            json!({
                "ok": false,
                "type": "black_box_ledger_export",
                "error": err
            }),
            1,
        ),
    }
}
