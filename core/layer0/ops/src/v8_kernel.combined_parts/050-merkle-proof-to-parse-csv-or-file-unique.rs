
pub fn merkle_proof(leaves: &[String], index: usize) -> Vec<Value> {
    if leaves.is_empty() || index >= leaves.len() {
        return Vec::new();
    }
    let mut out = Vec::new();
    let mut idx = index;
    let mut level = leaves
        .iter()
        .map(|leaf| sha256_hex_str(&format!("leaf:{leaf}")))
        .collect::<Vec<_>>();

    while level.len() > 1 {
        let sibling_idx = if idx % 2 == 0 {
            idx + 1
        } else {
            idx.saturating_sub(1)
        };
        let sibling = if sibling_idx < level.len() {
            level[sibling_idx].clone()
        } else {
            level[idx].clone()
        };
        out.push(json!({
            "level_size": level.len(),
            "index": idx,
            "sibling_index": sibling_idx.min(level.len().saturating_sub(1)),
            "sibling_hash": sibling
        }));

        let mut next = Vec::new();
        let mut i = 0usize;
        while i < level.len() {
            let left = &level[i];
            let right = if i + 1 < level.len() {
                &level[i + 1]
            } else {
                &level[i]
            };
            next.push(sha256_hex_str(&format!("node:{left}:{right}")));
            i += 2;
        }
        idx /= 2;
        level = next;
    }

    out
}

pub fn write_receipt(
    root: &Path,
    env_key: &str,
    scope: &str,
    mut payload: Value,
) -> Result<Value, String> {
    let latest = latest_path(root, env_key, scope);
    let history = history_path(root, env_key, scope);
    payload["ts"] = Value::String(now_iso());
    payload.set_receipt_hash();
    write_json(&latest, &payload)?;
    append_jsonl(&history, &payload)?;
    Ok(payload)
}

pub fn emit_plane_receipt(
    root: &Path,
    env_key: &str,
    scope: &str,
    error_type: &str,
    payload: Value,
) -> i32 {
    match write_receipt(root, env_key, scope, payload) {
        Ok(out) => {
            print_json(&out);
            if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                0
            } else {
                1
            }
        }
        Err(err) => {
            print_json(&json!({
                "ok": false,
                "type": clean(error_type, 120),
                "error": clean(err, 240)
            }));
            1
        }
    }
}

pub fn emit_attached_plane_receipt(
    root: &Path,
    env_key: &str,
    scope: &str,
    strict: bool,
    payload: Value,
    conduit: Option<&Value>,
) -> i32 {
    let out = attach_conduit(payload, conduit);
    let _ = write_json(&latest_path(root, env_key, scope), &out);
    let _ = append_jsonl(&history_path(root, env_key, scope), &out);
    print_json(&out);
    if strict && !out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        1
    } else if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        0
    } else {
        1
    }
}

pub fn plane_status(root: &Path, env_key: &str, scope: &str, status_type: &str) -> Value {
    json!({
        "ok": true,
        "type": clean(status_type, 120),
        "lane": "core/layer0/ops",
        "latest_path": latest_path(root, env_key, scope).display().to_string(),
        "latest": read_json(&latest_path(root, env_key, scope))
    })
}

pub fn split_csv_clean(raw: &str, max_len: usize) -> Vec<String> {
    raw.split(',')
        .map(|row| clean(row, max_len))
        .filter(|row| !row.is_empty())
        .collect()
}

pub fn parse_csv_flag(flags: &HashMap<String, String>, key: &str, max_len: usize) -> Vec<String> {
    flags
        .get(key)
        .map(|v| split_csv_clean(v, max_len))
        .unwrap_or_default()
}

pub fn parse_csv_or_file(
    root: &Path,
    flags: &HashMap<String, String>,
    csv_key: &str,
    file_key: &str,
    max_len: usize,
) -> Vec<String> {
    let mut values = parse_csv_flag(flags, csv_key, max_len);
    let Some(rel_or_abs) = flags.get(file_key) else {
        return values;
    };
    let path = if Path::new(rel_or_abs).is_absolute() {
        PathBuf::from(rel_or_abs)
    } else {
        root.join(rel_or_abs)
    };
    let Ok(raw) = fs::read_to_string(path) else {
        return values;
    };
    if raw.trim_start().starts_with('[') {
        if let Ok(parsed_json) = serde_json::from_str::<Value>(&raw) {
            if let Some(rows) = parsed_json.as_array() {
                for row in rows {
                    if let Some(text) = row.as_str() {
                        let cleaned = clean(text, max_len);
                        if !cleaned.is_empty() {
                            values.push(cleaned);
                        }
                    }
                }
            }
        }
        return values;
    }
    values.extend(split_csv_clean(&raw.replace('\n', ","), max_len));
    values
}

pub fn parse_csv_or_file_unique(
    root: &Path,
    flags: &HashMap<String, String>,
    csv_key: &str,
    file_key: &str,
    max_len: usize,
) -> Vec<String> {
    let mut values = parse_csv_or_file(root, flags, csv_key, file_key, max_len);
    values.sort();
    values.dedup();
    values
}
