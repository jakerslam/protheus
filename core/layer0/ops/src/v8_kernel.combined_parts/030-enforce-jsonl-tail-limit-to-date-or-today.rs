
fn enforce_jsonl_tail_limit(path: &Path, max_bytes: u64) -> Result<bool, String> {
    if max_bytes == u64::MAX {
        return Ok(false);
    }
    let current = fs::metadata(path)
        .map(|meta| meta.len())
        .map_err(|err| format!("jsonl_metadata_failed:{}:{err}", path.display()))?;
    if current <= max_bytes {
        return Ok(false);
    }

    let mut file = fs::File::open(path)
        .map_err(|err| format!("open_jsonl_failed:{}:{err}", path.display()))?;
    let read_len = current.min(max_bytes.saturating_add(RETENTION_TAIL_SLACK_BYTES));
    if current > read_len {
        file.seek(SeekFrom::End(-(read_len as i64)))
            .map_err(|err| format!("seek_jsonl_failed:{}:{err}", path.display()))?;
    }
    let mut buffer = Vec::<u8>::new();
    file.read_to_end(&mut buffer)
        .map_err(|err| format!("read_jsonl_failed:{}:{err}", path.display()))?;

    let mut start = 0usize;
    if current > read_len {
        if let Some(pos) = buffer.iter().position(|byte| *byte == b'\n') {
            start = pos.saturating_add(1);
        }
    }
    let retained = if start < buffer.len() {
        &buffer[start..]
    } else {
        &[][..]
    };

    atomic_write_bytes(path, retained)?;
    Ok(true)
}

fn enforce_binary_queue_limit(
    history_jsonl_path: &Path,
    queue_path: &Path,
    max_bytes: u64,
    force_rebuild: bool,
) -> Result<(), String> {
    let queue_too_large = if max_bytes == u64::MAX {
        false
    } else {
        fs::metadata(queue_path)
            .map(|meta| meta.len() > max_bytes)
            .unwrap_or(false)
    };
    if !force_rebuild && !queue_too_large {
        return Ok(());
    }
    rebuild_binary_queue_from_jsonl(history_jsonl_path, queue_path, max_bytes)
}

fn rebuild_binary_queue_from_jsonl(
    history_jsonl_path: &Path,
    queue_path: &Path,
    max_bytes: u64,
) -> Result<(), String> {
    let rows = read_jsonl(history_jsonl_path);
    if rows.is_empty() {
        if queue_path.exists() {
            fs::remove_file(queue_path).map_err(|err| {
                format!("remove_binary_queue_failed:{}:{err}", queue_path.display())
            })?;
        }
        return Ok(());
    }

    let mut frames = Vec::<Vec<u8>>::with_capacity(rows.len());
    let mut total = 0u64;
    for row in rows {
        let encoded = serde_json::to_vec(&row).map_err(|err| {
            format!(
                "encode_binary_receipt_failed:{}:{err}",
                queue_path.display()
            )
        })?;
        let mut frame = Vec::<u8>::with_capacity(4 + encoded.len());
        frame.extend_from_slice(&(encoded.len() as u32).to_le_bytes());
        frame.extend_from_slice(&encoded);
        total = total.saturating_add(frame.len() as u64);
        frames.push(frame);
    }

    let mut keep_from = 0usize;
    if max_bytes != u64::MAX && total > max_bytes {
        let mut running = 0u64;
        keep_from = frames.len().saturating_sub(1);
        for idx in (0..frames.len()).rev() {
            let frame_len = frames[idx].len() as u64;
            if running == 0 || running.saturating_add(frame_len) <= max_bytes {
                running = running.saturating_add(frame_len);
                keep_from = idx;
            } else {
                break;
            }
        }
    }

    let mut payload = Vec::<u8>::new();
    for frame in frames.into_iter().skip(keep_from) {
        payload.extend_from_slice(&frame);
    }
    atomic_write_bytes(queue_path, &payload)
}

fn atomic_write_bytes(path: &Path, payload: &[u8]) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create_dir_failed:{}:{err}", parent.display()))?;
    }
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    fs::write(&tmp, payload).map_err(|err| format!("write_tmp_failed:{}:{err}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|err| {
        format!(
            "rename_tmp_failed:{}:{}:{err}",
            tmp.display(),
            path.display()
        )
    })
}

pub fn print_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

pub trait ReceiptJsonExt {
    fn with_receipt_hash(self) -> Value;
    fn set_receipt_hash(&mut self);
}

impl ReceiptJsonExt for Value {
    fn with_receipt_hash(mut self) -> Value {
        self.set_receipt_hash();
        self
    }

    fn set_receipt_hash(&mut self) {
        let mut unhashed = self.clone();
        if let Some(obj) = unhashed.as_object_mut() {
            obj.remove("receipt_hash");
        }
        self["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&unhashed));
    }
}

pub fn parse_bool(raw: Option<&String>, fallback: bool) -> bool {
    match raw.map(|v| v.trim().to_ascii_lowercase()) {
        Some(v) if matches!(v.as_str(), "1" | "true" | "yes" | "on") => true,
        Some(v) if matches!(v.as_str(), "0" | "false" | "no" | "off") => false,
        _ => fallback,
    }
}

pub fn parse_bool_str(raw: Option<&str>, fallback: bool) -> bool {
    match raw.map(|v| v.trim().to_ascii_lowercase()) {
        Some(v) if matches!(v.as_str(), "1" | "true" | "yes" | "on") => true,
        Some(v) if matches!(v.as_str(), "0" | "false" | "no" | "off") => false,
        _ => fallback,
    }
}

pub fn parse_f64(raw: Option<&String>, fallback: f64) -> f64 {
    raw.and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(fallback)
}

pub fn parse_f64_str(raw: Option<&str>, fallback: f64) -> f64 {
    raw.and_then(|v| v.trim().parse::<f64>().ok())
        .unwrap_or(fallback)
}

pub fn parse_u64(raw: Option<&String>, fallback: u64) -> u64 {
    raw.and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(fallback)
}

pub fn parse_u64_str(raw: Option<&str>, fallback: u64) -> u64 {
    raw.and_then(|v| v.trim().parse::<u64>().ok())
        .unwrap_or(fallback)
}

pub fn parse_i64(raw: Option<&String>, fallback: i64) -> i64 {
    raw.and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(fallback)
}

pub fn parse_i64_clamped(raw: Option<&String>, fallback: i64, lo: i64, hi: i64) -> i64 {
    parse_i64(raw, fallback).clamp(lo, hi)
}

pub fn parse_json_or_empty(raw: Option<&String>) -> Value {
    raw.and_then(|s| serde_json::from_str::<Value>(s).ok())
        .unwrap_or_else(|| json!({}))
}

pub fn date_or_today(raw: Option<&String>) -> String {
    let candidate = raw.map(|v| v.trim().to_string()).unwrap_or_default();
    if !candidate.is_empty() && chrono::NaiveDate::parse_from_str(&candidate, "%Y-%m-%d").is_ok() {
        return candidate;
    }
    now_iso().chars().take(10).collect()
}
