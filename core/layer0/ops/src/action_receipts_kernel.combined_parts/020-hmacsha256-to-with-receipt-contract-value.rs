
type HmacSha256 = Hmac<Sha256>;

fn usage() {
    println!("action-receipts-kernel commands:");
    println!("  infring-ops action-receipts-kernel now-iso");
    println!("  infring-ops action-receipts-kernel append-jsonl --payload-base64=<json>");
    println!("  infring-ops action-receipts-kernel with-receipt-contract --payload-base64=<json>");
    println!("  infring-ops action-receipts-kernel write-contract-receipt --payload-base64=<json>");
    println!(
        "  infring-ops action-receipts-kernel replay-task-lineage --task-id=<id> [--trace-id=<id>] [--limit=<n>] [--scan-root=<path>] [--sources=<csv_paths>]"
    );
    println!(
        "  infring-ops action-receipts-kernel query-task-lineage --task-id=<id> [--trace-id=<id>] [--limit=<n>] [--scan-root=<path>] [--sources=<csv_paths>]"
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
            .map_err(|err| format!("action_receipts_kernel_payload_decode_failed:{err}"));
    }
    if let Some(raw_b64) = lane_utils::parse_flag(argv, "payload-base64", false) {
        let bytes = BASE64_STANDARD
            .decode(raw_b64.as_bytes())
            .map_err(|err| format!("action_receipts_kernel_payload_base64_decode_failed:{err}"))?;
        let text = String::from_utf8(bytes)
            .map_err(|err| format!("action_receipts_kernel_payload_utf8_decode_failed:{err}"))?;
        return serde_json::from_str::<Value>(&text)
            .map_err(|err| format!("action_receipts_kernel_payload_decode_failed:{err}"));
    }
    Ok(json!({}))
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    value.as_object().unwrap_or_else(|| {
        static EMPTY: std::sync::OnceLock<Map<String, Value>> = std::sync::OnceLock::new();
        EMPTY.get_or_init(Map::new)
    })
}

fn as_object<'a>(value: Option<&'a Value>) -> Option<&'a Map<String, Value>> {
    value.and_then(Value::as_object)
}

fn as_str(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(v)) => v.trim().to_string(),
        Some(Value::Null) | None => String::new(),
        Some(v) => v.to_string().trim_matches('"').trim().to_string(),
    }
}

fn resolve_file_path(root: &Path, raw: &str) -> PathBuf {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return root.join("local").join("state").join("receipts.jsonl");
    }
    let candidate = PathBuf::from(trimmed);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("action_receipts_kernel_create_dir_failed:{err}"))?;
    }
    Ok(())
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("action_receipts_kernel_append_open_failed:{err}"))?;
    file.write_all(
        format!(
            "{}\n",
            serde_json::to_string(row).unwrap_or_else(|_| "null".to_string())
        )
        .as_bytes(),
    )
    .map_err(|err| format!("action_receipts_kernel_append_failed:{err}"))
}

fn chain_state_path(file_path: &Path) -> PathBuf {
    PathBuf::from(format!("{}.chain.json", file_path.to_string_lossy()))
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let digest = hasher.finalize();
    let mut out = String::with_capacity(digest.len() * 2);
    for byte in digest {
        out.push_str(&format!("{byte:02x}"));
    }
    out
}

fn optional_hmac(hash: &str) -> Result<Option<String>, String> {
    let key = std::env::var("RECEIPT_CHAIN_HMAC_KEY").unwrap_or_default();
    let key = key.trim();
    if key.is_empty() {
        return Ok(None);
    }
    let mut mac = HmacSha256::new_from_slice(key.as_bytes())
        .map_err(|err| format!("action_receipts_kernel_hmac_init_failed:{err}"))?;
    mac.update(hash.as_bytes());
    Ok(Some(hex::encode(mac.finalize().into_bytes())))
}

fn read_chain_state(file_path: &Path) -> (u64, Option<String>) {
    let state_path = chain_state_path(file_path);
    let parsed = fs::read_to_string(state_path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| json!({}));
    let seq = parsed.get("seq").and_then(Value::as_u64).unwrap_or(0);
    let hash = parsed
        .get("hash")
        .and_then(Value::as_str)
        .map(|row| row.to_string());
    (seq, hash)
}

fn write_chain_state(file_path: &Path, seq: u64, hash: Option<&str>) -> Result<(), String> {
    let state_path = chain_state_path(file_path);
    ensure_parent(&state_path)?;
    let tmp_path = PathBuf::from(format!(
        "{}.tmp-{}",
        state_path.to_string_lossy(),
        std::process::id()
    ));
    fs::write(
        &tmp_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&json!({
                "seq": seq,
                "hash": hash,
                "ts": now_iso(),
            }))
            .map_err(|err| format!("action_receipts_kernel_encode_failed:{err}"))?
        ),
    )
    .map_err(|err| format!("action_receipts_kernel_write_failed:{err}"))?;
    fs::rename(&tmp_path, &state_path)
        .map_err(|err| format!("action_receipts_kernel_rename_failed:{err}"))
}

fn with_receipt_contract_value(record: &Value, attempted: bool, verified: bool) -> Value {
    let src = as_object(Some(record)).cloned().unwrap_or_default();
    let mut receipt_contract = as_object(src.get("receipt_contract"))
        .cloned()
        .unwrap_or_default();
    receipt_contract.insert("version".to_string(), Value::String("1.0".to_string()));
    receipt_contract.insert("attempted".to_string(), Value::Bool(attempted));
    receipt_contract.insert("verified".to_string(), Value::Bool(verified));
    receipt_contract.insert("recorded".to_string(), Value::Bool(true));
    let mut out = src;
    out.insert(
        "receipt_contract".to_string(),
        Value::Object(receipt_contract),
    );
    Value::Object(out)
}
