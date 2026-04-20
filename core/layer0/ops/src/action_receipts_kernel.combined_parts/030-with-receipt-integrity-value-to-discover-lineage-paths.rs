
fn with_receipt_integrity_value(file_path: &Path, record: &Value) -> Result<Value, String> {
    let src = as_object(Some(record)).cloned().unwrap_or_default();
    let (prev_seq, prev_hash) = read_chain_state(file_path);
    let seq = prev_seq.saturating_add(1);
    let payload_hash = sha256_hex(
        &serde_json::to_string(&Value::Object(src.clone())).unwrap_or_else(|_| "{}".to_string()),
    );
    let link_hash = sha256_hex(&format!(
        "{seq}:{}:{payload_hash}",
        prev_hash.clone().unwrap_or_default()
    ));
    let hmac = optional_hmac(&link_hash)?;

    let mut receipt_contract = as_object(src.get("receipt_contract"))
        .cloned()
        .unwrap_or_default();
    receipt_contract.insert(
        "integrity".to_string(),
        json!({
            "version": "1.0",
            "seq": seq,
            "prev_hash": prev_hash,
            "payload_hash": payload_hash,
            "hash": link_hash,
            "hmac": hmac,
            "ts": now_iso(),
        }),
    );
    let mut out = src;
    out.insert(
        "receipt_contract".to_string(),
        Value::Object(receipt_contract),
    );
    let out_value = Value::Object(out);
    let current_hash = out_value
        .get("receipt_contract")
        .and_then(Value::as_object)
        .and_then(|row| row.get("integrity"))
        .and_then(Value::as_object)
        .and_then(|row| row.get("hash"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    write_chain_state(file_path, seq, Some(&current_hash))?;
    Ok(out_value)
}

fn parse_attempted(payload: &Map<String, Value>) -> bool {
    payload
        .get("attempted")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn parse_verified(payload: &Map<String, Value>) -> bool {
    payload
        .get("verified")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn write_contract_receipt_value(
    root: &Path,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let file_path = resolve_file_path(root, &as_str(payload.get("file_path")));
    let record = payload.get("record").cloned().unwrap_or_else(|| json!({}));
    let with_contract =
        with_receipt_contract_value(&record, parse_attempted(payload), parse_verified(payload));
    let with_integrity = with_receipt_integrity_value(&file_path, &with_contract)?;
    append_jsonl(&file_path, &with_integrity)?;
    Ok(json!({
        "ok": true,
        "file_path": file_path.to_string_lossy(),
        "record": with_integrity,
    }))
}

fn parse_lineage_limit(payload: &Map<String, Value>) -> usize {
    payload
        .get("limit")
        .and_then(Value::as_u64)
        .map(|v| v as usize)
        .filter(|v| *v > 0)
        .unwrap_or(4000)
        .min(50_000)
}

fn parse_scan_root(root: &Path, payload: &Map<String, Value>) -> PathBuf {
    let raw = as_str(payload.get("scan_root"));
    if raw.is_empty() {
        return root.to_path_buf();
    }
    let candidate = PathBuf::from(raw);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn source_paths_from_payload(root: &Path, payload: &Map<String, Value>) -> Vec<PathBuf> {
    let explicit = as_str(payload.get("sources"));
    if explicit.trim().is_empty() {
        return Vec::new();
    }
    explicit
        .split(',')
        .map(|row| resolve_file_path(root, row))
        .filter(|path| path.exists())
        .collect::<Vec<_>>()
}

fn known_lineage_paths(scan_root: &Path) -> Vec<PathBuf> {
    let mut out = vec![
        scan_root
            .join("local")
            .join("state")
            .join("runtime")
            .join("task_runtime")
            .join("verity_receipts.jsonl"),
        scan_root
            .join("local")
            .join("state")
            .join("runtime")
            .join("task_runtime")
            .join("conduit_messages.jsonl"),
        scan_root
            .join("client")
            .join("runtime")
            .join("local")
            .join("state")
            .join("ui")
            .join("infring_dashboard")
            .join("actions")
            .join("history.jsonl"),
        scan_root
            .join("local")
            .join("state")
            .join("attention")
            .join("receipts.jsonl"),
        scan_root
            .join("local")
            .join("state")
            .join("stomach")
            .join("receipts.jsonl"),
        scan_root
            .join("local")
            .join("state")
            .join("ops")
            .join("verity")
            .join("receipts.jsonl"),
    ];
    out.retain(|path| path.exists());
    out
}

fn is_replay_candidate_name(name: &str) -> bool {
    matches!(
        name,
        "history.jsonl"
            | "receipts.jsonl"
            | "verity_receipts.jsonl"
            | "conduit_messages.jsonl"
            | "protocol_step_receipts.jsonl"
            | "protocol_history.jsonl"
    )
}

fn should_skip_replay_path(path: &Path) -> bool {
    let lowered = path.to_string_lossy().to_ascii_lowercase();
    lowered.contains("/assimilation/isolated/")
        || lowered.contains("/assimilation/burned/")
        || lowered.contains("/node_modules/")
        || lowered.contains("/.git/")
        || lowered.contains("/target/")
}

fn discover_lineage_paths(scan_root: &Path) -> Vec<PathBuf> {
    let roots = [
        scan_root.join("local").join("state"),
        scan_root.join("core").join("local").join("state"),
        scan_root
            .join("client")
            .join("runtime")
            .join("local")
            .join("state"),
    ];
    let mut out = BTreeSet::<PathBuf>::new();
    for root in roots {
        if !root.exists() {
            continue;
        }
        for entry in WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
        {
            let path = entry.path();
            if !path.is_file() || should_skip_replay_path(path) {
                continue;
            }
            let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
                continue;
            };
            if is_replay_candidate_name(name) {
                out.insert(path.to_path_buf());
            }
        }
    }
    out.into_iter().collect::<Vec<_>>()
}
