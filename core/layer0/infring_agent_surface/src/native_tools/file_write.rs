use crate::native_tools::hashing::sha256_hex;
use crate::native_tools::paths::required_abs_path;
use serde_json::{json, Value};
use std::fs;

pub fn file_write(args: &Value) -> Result<Value, String> {
    let path = required_abs_path(args)?;
    let content = args
        .get("content")
        .and_then(Value::as_str)
        .ok_or_else(|| "content_required".to_string())?;
    let overwrite = args
        .get("overwrite")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let existed = path.exists();
    if existed && !overwrite {
        return Err("overwrite_permission_required".to_string());
    }
    let previous_hash = if existed {
        Some(sha256_hex(
            &fs::read(&path).map_err(|error| format!("pre_write_snapshot_failed:{error}"))?,
        ))
    } else {
        None
    };
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|error| format!("parent_create_failed:{error}"))?;
    }
    fs::write(&path, content).map_err(|error| format!("file_write_failed:{error}"))?;
    let bytes = fs::read(&path).map_err(|error| format!("post_write_read_failed:{error}"))?;
    Ok(json!({
        "path": path.display().to_string(),
        "created": !existed,
        "overwritten": existed,
        "previous_content_hash": previous_hash,
        "new_content_hash": sha256_hex(&bytes),
        "bytes_written": bytes.len(),
    }))
}
