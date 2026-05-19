use crate::native_tools::export_guard::ensure_no_export_removal;
use crate::native_tools::hashing::sha256_hex;
use crate::native_tools::paths::required_abs_path;
use serde_json::{json, Value};
use std::fs;

pub fn file_patch(args: &Value) -> Result<Value, String> {
    let path = required_abs_path(args)?;
    let old = args
        .get("old")
        .or_else(|| args.get("find"))
        .or_else(|| args.get("search"))
        .or_else(|| args.get("before"))
        .or_else(|| args.get("original"))
        .and_then(Value::as_str)
        .ok_or_else(|| "old_required".to_string())?;
    let new = args
        .get("new")
        .or_else(|| args.get("replace"))
        .or_else(|| args.get("replacement"))
        .or_else(|| args.get("after"))
        .or_else(|| args.get("updated"))
        .and_then(Value::as_str)
        .ok_or_else(|| "new_required".to_string())?;
    let allow_multiple = args
        .get("allow_multiple")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let content =
        fs::read_to_string(&path).map_err(|error| format!("file_patch_read_failed:{error}"))?;
    let count = content.matches(old).count();
    if count == 0 {
        return Err("patch_old_text_not_found".to_string());
    }
    if count > 1 && !allow_multiple {
        return Err("patch_old_text_not_unique".to_string());
    }
    let previous_hash = sha256_hex(content.as_bytes());
    let patched = if allow_multiple {
        content.replace(old, new)
    } else {
        content.replacen(old, new, 1)
    };
    ensure_no_export_removal(&path, &content, &patched, args)?;
    fs::write(&path, &patched).map_err(|error| format!("file_patch_write_failed:{error}"))?;
    Ok(json!({
        "path": path.display().to_string(),
        "replacement_count": if allow_multiple { count } else { 1 },
        "previous_content_hash": previous_hash,
        "new_content_hash": sha256_hex(patched.as_bytes()),
    }))
}
