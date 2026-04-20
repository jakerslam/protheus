
fn truncate_utf8_lossy(bytes: &[u8], max_bytes: usize) -> (String, bool) {
    if bytes.len() <= max_bytes {
        return (String::from_utf8_lossy(bytes).to_string(), false);
    }
    let mut end = max_bytes;
    while end > 0 && !std::str::from_utf8(&bytes[..end]).is_ok() {
        end -= 1;
    }
    let slice = if end == 0 {
        &bytes[..max_bytes]
    } else {
        &bytes[..end]
    };
    (String::from_utf8_lossy(slice).to_string(), true)
}

fn bytes_look_binary(bytes: &[u8]) -> bool {
    if bytes.is_empty() {
        return false;
    }
    let probe_len = bytes.len().min(4096);
    let sample = &bytes[..probe_len];
    if sample.iter().any(|byte| *byte == 0) {
        return true;
    }
    let control_count = sample
        .iter()
        .filter(|byte| {
            let b = **byte;
            b < 9 || (b > 13 && b < 32)
        })
        .count();
    let control_ratio = control_count as f64 / probe_len as f64;
    if control_ratio > 0.12 {
        return true;
    }
    std::str::from_utf8(sample).is_err() && control_ratio > 0.04
}

fn guess_mime_type_for_file(path: &Path, bytes: &[u8]) -> String {
    let ext = path
        .extension()
        .and_then(|row| row.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();
    let known = match ext.as_str() {
        "md" => "text/markdown; charset=utf-8",
        "txt" | "log" | "toml" | "yaml" | "yml" | "json" | "jsonl" | "csv" | "tsv" => {
            "text/plain; charset=utf-8"
        }
        "rs" | "ts" | "tsx" | "py" | "sh" | "zsh" | "bash" | "js" | "cjs" | "mjs" | "c" | "h"
        | "cpp" | "hpp" | "go" | "java" | "kt" | "swift" | "sql" | "css" | "html" | "xml" => {
            "text/plain; charset=utf-8"
        }
        "pdf" => "application/pdf",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "webp" => "image/webp",
        "mp3" => "audio/mpeg",
        "wav" => "audio/wav",
        "mp4" => "video/mp4",
        "zip" => "application/zip",
        "gz" => "application/gzip",
        "tar" => "application/x-tar",
        _ => "",
    };
    if !known.is_empty() {
        return known.to_string();
    }
    if bytes_look_binary(bytes) {
        "application/octet-stream".to_string()
    } else {
        "text/plain; charset=utf-8".to_string()
    }
}

fn attention_policy_path(root: &Path) -> PathBuf {
    let from_env = std::env::var("MECH_SUIT_MODE_POLICY_PATH")
        .ok()
        .map(PathBuf::from);
    if let Some(path) = from_env {
        if path.is_absolute() {
            return path;
        }
        return root.join(path);
    }
    let default_root = root.join("config").join("mech_suit_mode_policy.json");
    if default_root.exists() {
        return default_root;
    }
    root.join("client/runtime/config/mech_suit_mode_policy.json")
}

fn attention_queue_path_for_dashboard(root: &Path) -> PathBuf {
    let fallback = root.join("client/runtime/local/state/attention/queue.jsonl");
    let policy = read_json_loose(&attention_policy_path(root)).unwrap_or_else(|| json!({}));
    let from_policy = clean_text(
        policy
            .pointer("/eyes/attention_queue_path")
            .and_then(Value::as_str)
            .unwrap_or(""),
        4000,
    );
    if from_policy.is_empty() {
        return fallback;
    }
    let raw = PathBuf::from(from_policy);
    if raw.is_absolute() {
        raw
    } else {
        root.join(raw)
    }
}

