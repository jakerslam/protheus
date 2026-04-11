fn run_postprocess_table(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        TABLE_POSTPROCESS_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "table_postprocessing_pipeline_contract",
            "stages": ["detect_fake_table", "merge_simplify", "footnote_handle"],
            "default_max_rows": 5000,
            "default_max_cols": 64
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("table_postprocess_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "table_postprocessing_pipeline_contract"
    {
        errors.push("table_postprocess_contract_kind_invalid".to_string());
    }

    let (source_hint, table) = match parse_table_input(root, parsed) {
        Ok(ok) => ok,
        Err(err) => {
            errors.push(err);
            ("".to_string(), Vec::new())
        }
    };
    let max_rows = parse_u64(
        parsed.flags.get("max-rows"),
        contract
            .get("default_max_rows")
            .and_then(Value::as_u64)
            .unwrap_or(5000),
    )
    .clamp(1, 20_000) as usize;
    let max_cols = parse_u64(
        parsed.flags.get("max-cols"),
        contract
            .get("default_max_cols")
            .and_then(Value::as_u64)
            .unwrap_or(64),
    )
    .clamp(1, 512) as usize;

    if table.is_empty() {
        errors.push("table_required".to_string());
    }
    if strict && table.len() > max_rows {
        errors.push("table_rows_exceed_contract_limit".to_string());
    }
    if strict && table.iter().any(|row| row.len() > max_cols) {
        errors.push("table_cols_exceed_contract_limit".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "parse_plane_postprocess_table",
            "errors": errors
        });
    }

    let before = table.clone();
    let before_hash = sha256_hex_str(&canonical_json_string(&json!(before)));

    let mut fake_rows_removed = 0usize;
    let mut stage1 = Vec::<Vec<String>>::new();
    for row in &before {
        if is_fake_row(row) {
            fake_rows_removed += 1;
        } else {
            stage1.push(
                row.iter()
                    .map(|cell| clean(cell.trim(), 800))
                    .collect::<Vec<_>>(),
            );
        }
    }

    let mut merged_rows = 0usize;
    let mut stage2 = Vec::<Vec<String>>::new();
    for row in &stage1 {
        let first_empty = row
            .first()
            .map(|cell| cell.trim().is_empty())
            .unwrap_or(true);
        if first_empty && !stage2.is_empty() {
            let prev = stage2.last_mut().expect("prev");
            for (idx, cell) in row.iter().enumerate() {
                if cell.trim().is_empty() {
                    continue;
                }
                if idx >= prev.len() {
                    prev.push(clean(cell.trim(), 800));
                    continue;
                }
                if prev[idx].trim().is_empty() {
                    prev[idx] = clean(cell.trim(), 800);
                } else {
                    prev[idx] = clean(format!("{} {}", prev[idx], cell.trim()), 800);
                }
            }
            merged_rows += 1;
        } else {
            stage2.push(row.clone());
        }
    }

    let mut footnotes = Vec::<Value>::new();
    let mut stage3 = Vec::<Vec<String>>::new();
    for (row_idx, row) in stage2.iter().enumerate() {
        let mut rendered = Vec::<String>::new();
        for (col_idx, cell) in row.iter().enumerate() {
            let (cleaned, note) = strip_footnote(cell);
            if let Some(marker) = note {
                footnotes.push(json!({
                    "row": row_idx,
                    "col": col_idx,
                    "marker": marker
                }));
            }
            rendered.push(cleaned);
        }
        stage3.push(rendered);
    }

    let stage_receipts = vec![
        json!({
            "stage": "detect_fake_table",
            "before_rows": before.len(),
            "after_rows": stage1.len(),
            "fake_rows_removed": fake_rows_removed
        }),
        json!({
            "stage": "merge_simplify",
            "before_rows": stage1.len(),
            "after_rows": stage2.len(),
            "rows_merged": merged_rows
        }),
        json!({
            "stage": "footnote_handle",
            "footnotes_extracted": footnotes.len()
        }),
    ];

    if strict && stage3.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "parse_plane_postprocess_table",
            "errors": ["table_empty_after_postprocess"],
            "stage_receipts": stage_receipts
        });
    }

    let after_hash = sha256_hex_str(&canonical_json_string(&json!(stage3)));

    let artifact = json!({
        "source_hint": source_hint,
        "before": before,
        "after": stage3,
        "footnotes": footnotes,
        "limits": {"max_rows": max_rows, "max_cols": max_cols},
        "hashes": {"before_sha256": before_hash, "after_sha256": after_hash},
        "stage_receipts": stage_receipts
    });
    let artifact_path = state_root(root)
        .join("parse_postprocess")
        .join("latest.json");
    let _ = write_json(&artifact_path, &artifact);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "parse_plane_postprocess_table",
        "lane": "core/layer0/ops",
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&artifact.to_string())
        },
        "result": artifact,
        "claim_evidence": [
            {
                "id": "V6-PARSE-001.3",
                "claim": "advanced_table_postprocessing_pipeline_executes_fake_table_detection_merge_simplify_and_footnote_handling_with_before_after_evidence",
                "evidence": {
                    "fake_rows_removed": fake_rows_removed,
                    "rows_merged": merged_rows,
                    "footnotes_extracted": footnotes.len()
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn flatten_key(prefix: &str, segment: &str, format: &str) -> String {
    if prefix.is_empty() {
        return clean(segment, 200);
    }
    if format == "slash" {
        clean(format!("{prefix}/{segment}"), 300)
    } else {
        clean(format!("{prefix}.{segment}"), 300)
    }
}

fn is_scalar(value: &Value) -> bool {
    matches!(
        value,
        Value::Null | Value::Bool(_) | Value::Number(_) | Value::String(_)
    )
}

fn flatten_value(
    prefix: &str,
    value: &Value,
    depth: usize,
    max_depth: usize,
    format: &str,
    out: &mut Map<String, Value>,
) {
    if depth > max_depth {
        out.insert(prefix.to_string(), Value::String("[max_depth]".to_string()));
        return;
    }
    match value {
        Value::Object(map) => {
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            for key in keys {
                if let Some(next) = map.get(&key) {
                    let joined = flatten_key(prefix, &key, format);
                    flatten_value(&joined, next, depth + 1, max_depth, format, out);
                }
            }
        }
        Value::Array(rows) => {
            if rows.iter().all(is_scalar) {
                out.insert(prefix.to_string(), Value::Array(rows.clone()));
            } else {
                for (idx, next) in rows.iter().enumerate() {
                    let joined = flatten_key(prefix, &idx.to_string(), format);
                    flatten_value(&joined, next, depth + 1, max_depth, format, out);
                }
            }
        }
        _ => {
            out.insert(prefix.to_string(), value.clone());
        }
    }
}

fn collect_unnested_rows(
    prefix: &str,
    value: &Value,
    depth: usize,
    max_depth: usize,
    format: &str,
    out: &mut Vec<Value>,
) {
    if depth > max_depth {
        return;
    }
    match value {
        Value::Object(map) => {
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            for key in keys {
                if let Some(next) = map.get(&key) {
                    let joined = flatten_key(prefix, &key, format);
                    collect_unnested_rows(&joined, next, depth + 1, max_depth, format, out);
                }
            }
        }
        Value::Array(rows) => {
            for (idx, next) in rows.iter().enumerate() {
                if let Some(obj) = next.as_object() {
                    let mut row = Map::<String, Value>::new();
                    row.insert("__path".to_string(), Value::String(prefix.to_string()));
                    row.insert("__index".to_string(), json!(idx));
                    let mut keys = obj.keys().cloned().collect::<Vec<_>>();
                    keys.sort();
                    for key in keys {
                        let cell = obj.get(&key).cloned().unwrap_or(Value::Null);
                        if is_scalar(&cell) {
                            row.insert(key, cell);
                        } else {
                            row.insert(
                                key,
                                Value::String(clean(canonical_json_string(&cell), 400)),
                            );
                        }
                    }
                    out.push(Value::Object(row));
                }
                let joined = flatten_key(prefix, &idx.to_string(), format);
                collect_unnested_rows(&joined, next, depth + 1, max_depth, format, out);
            }
        }
        _ => {}
    }
}

fn parse_transform_input(
    root: &Path,
    parsed: &crate::ParsedArgs,
) -> Result<(String, Value), String> {
    if let Some(raw) = parsed.flags.get("json") {
        let value =
            serde_json::from_str::<Value>(raw).map_err(|_| "json_payload_invalid".to_string())?;
        return Ok(("json".to_string(), value));
    }
    if let Some(rel_or_abs) = parsed.flags.get("json-path") {
        let path = if Path::new(rel_or_abs).is_absolute() {
            PathBuf::from(rel_or_abs)
        } else {
            root.join(rel_or_abs)
        };
        let value =
            read_json(&path).ok_or_else(|| format!("json_path_not_found:{}", path.display()))?;
        return Ok((path.display().to_string(), value));
    }
    let from_path = parsed.flags.get("from-path").cloned().unwrap_or_else(|| {
        state_root(root)
            .join("parse_doc")
            .join("latest.json")
            .display()
            .to_string()
    });
    let path = if Path::new(&from_path).is_absolute() {
        PathBuf::from(&from_path)
    } else {
        root.join(&from_path)
    };
    let artifact =
        read_json(&path).ok_or_else(|| format!("from_path_not_found:{}", path.display()))?;
    let structured = artifact
        .get("pipeline")
        .and_then(|v| v.get("structured"))
        .cloned()
        .or_else(|| artifact.get("after").cloned())
        .or_else(|| artifact.get("flattened").cloned())
        .or_else(|| artifact.get("result").and_then(|v| v.get("after")).cloned())
        .or_else(|| artifact.get("result").and_then(|v| v.get("flattened")).cloned())
        .or_else(|| artifact.get("result").cloned())
        .unwrap_or(Value::Null);
    if structured.is_null() {
        return Err("structured_payload_missing".to_string());
    }
    Ok((path.display().to_string(), structured))
}
