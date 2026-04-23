
const DEFAULT_RECEIPT_HISTORY_MAX_BYTES: u64 = 2 * 1024 * 1024;
const DEFAULT_RECEIPT_BINARY_MAX_BYTES: u64 = 2 * 1024 * 1024;
const RETENTION_MAX_BYTES_CAP: u64 = 1024 * 1024 * 1024;
const RETENTION_TAIL_SLACK_BYTES: u64 = 8 * 1024;

fn env_nonempty_path(env_key: &str) -> Option<PathBuf> {
    std::env::var(env_key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .map(PathBuf::from)
}

pub fn scoped_state_root(root: &Path, env_key: &str, scope: &str) -> PathBuf {
    if let Some(path) = env_nonempty_path(env_key) {
        return path;
    }
    crate::core_state_root(root).join("ops").join(scope)
}

pub fn state_root_from_env_or(root: &Path, env_key: &str, default_rel: &[&str]) -> PathBuf {
    if let Some(path) = env_nonempty_path(env_key) {
        return path;
    }
    default_rel
        .iter()
        .fold(root.to_path_buf(), |path, segment| path.join(segment))
}

pub fn latest_path(root: &Path, env_key: &str, scope: &str) -> PathBuf {
    scoped_state_root(root, env_key, scope).join("latest.json")
}

pub fn history_path(root: &Path, env_key: &str, scope: &str) -> PathBuf {
    scoped_state_root(root, env_key, scope).join("history.jsonl")
}

pub fn read_json(path: &Path) -> Option<Value> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str::<Value>(&raw).ok()
}

pub fn read_jsonl(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .ok()
        .map(|raw| {
            raw.lines()
                .filter_map(|line| serde_json::from_str::<Value>(line).ok())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

pub fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create_dir_failed:{}:{err}", parent.display()))?;
    }
    let payload = serde_json::to_string_pretty(value)
        .map_err(|err| format!("encode_json_failed:{}:{err}", path.display()))?;
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    fs::write(&tmp, format!("{payload}\n"))
        .map_err(|err| format!("write_tmp_failed:{}:{err}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|err| {
        format!(
            "rename_tmp_failed:{}:{}:{err}",
            tmp.display(),
            path.display()
        )
    })
}

pub fn append_jsonl(path: &Path, value: &Value) -> Result<(), String> {
    append_jsonl_with_limits(
        path,
        value,
        receipt_history_max_bytes(),
        receipt_binary_queue_enabled(),
        receipt_binary_queue_max_bytes(),
    )
}

pub fn append_jsonl_without_binary_queue(path: &Path, value: &Value) -> Result<(), String> {
    append_jsonl_with_limits(path, value, receipt_history_max_bytes(), false, 0)
}

pub fn append_jsonl_with_limits(
    path: &Path,
    value: &Value,
    history_max_bytes: u64,
    binary_queue_enabled: bool,
    binary_max_bytes: u64,
) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create_dir_failed:{}:{err}", parent.display()))?;
    }
    let line = serde_json::to_string(value)
        .map_err(|err| format!("encode_jsonl_failed:{}:{err}", path.display()))?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("open_jsonl_failed:{}:{err}", path.display()))?;
    writeln!(file, "{line}")
        .map_err(|err| format!("append_jsonl_failed:{}:{err}", path.display()))?;

    let queue_path = if binary_queue_enabled {
        let queue = receipt_binary_queue_path(path);
        append_binary_queue(&queue, value)?;
        Some(queue)
    } else {
        None
    };

    let history_trimmed = enforce_jsonl_tail_limit(path, history_max_bytes)?;
    if let Some(queue) = queue_path {
        enforce_binary_queue_limit(path, &queue, binary_max_bytes, history_trimmed)?;
    }
    Ok(())
}

fn receipt_binary_queue_enabled() -> bool {
    match std::env::var("INFRING_RECEIPT_BINARY_QUEUE") {
        Ok(raw) => !matches!(
            raw.trim().to_ascii_lowercase().as_str(),
            "0" | "false" | "off" | "no"
        ),
        Err(_) => true,
    }
}

fn parse_retention_max_bytes_env(name: &str, fallback: u64) -> u64 {
    match std::env::var(name) {
        Ok(raw) => match raw.trim().parse::<u64>() {
            Ok(0) => u64::MAX,
            Ok(v) => v.min(RETENTION_MAX_BYTES_CAP),
            Err(_) => fallback,
        },
        Err(_) => fallback,
    }
}

fn receipt_history_max_bytes() -> u64 {
    parse_retention_max_bytes_env(
        "INFRING_RECEIPT_HISTORY_MAX_BYTES",
        DEFAULT_RECEIPT_HISTORY_MAX_BYTES,
    )
}

fn receipt_binary_queue_max_bytes() -> u64 {
    parse_retention_max_bytes_env(
        "INFRING_RECEIPT_BINARY_QUEUE_MAX_BYTES",
        DEFAULT_RECEIPT_BINARY_MAX_BYTES,
    )
}

pub fn receipt_binary_queue_path(history_jsonl_path: &Path) -> PathBuf {
    let parent = history_jsonl_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_default();
    let stem = history_jsonl_path
        .file_stem()
        .and_then(|v| v.to_str())
        .unwrap_or("history");
    parent.join(format!("{stem}.bin"))
}

pub fn append_binary_queue(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("create_dir_failed:{}:{err}", parent.display()))?;
    }
    let encoded = serde_json::to_vec(value)
        .map_err(|err| format!("encode_binary_receipt_failed:{}:{err}", path.display()))?;
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("open_binary_receipt_failed:{}:{err}", path.display()))?;
    let len = (encoded.len() as u32).to_le_bytes();
    file.write_all(&len)
        .and_then(|_| file.write_all(&encoded))
        .map_err(|err| format!("append_binary_receipt_failed:{}:{err}", path.display()))
}
