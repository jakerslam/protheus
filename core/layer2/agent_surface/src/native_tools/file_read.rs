use crate::native_tools::hashing::sha256_hex;
use crate::native_tools::paths::required_abs_path;
use serde_json::{json, Value};
use std::fs;

pub fn file_read(args: &Value) -> Result<Value, String> {
    let path = required_abs_path(args)?;
    let bytes = fs::read(&path).map_err(|error| format!("file_read_failed:{error}"))?;
    if bytes.contains(&0) {
        return Err("binary_text_read_rejected".to_string());
    }
    let full_hash = sha256_hex(&bytes);
    let content = String::from_utf8(bytes).map_err(|_| "utf8_text_read_required".to_string())?;
    let lines = content.lines().collect::<Vec<_>>();
    let total_lines = lines.len();
    let start_line = args
        .get("start_line")
        .and_then(Value::as_u64)
        .unwrap_or(1)
        .max(1) as usize;
    let end_line = args
        .get("end_line")
        .and_then(Value::as_u64)
        .map(|value| value.max(1) as usize)
        .unwrap_or(total_lines);
    if start_line > end_line.saturating_add(1) {
        return Err("invalid_line_range".to_string());
    }
    let selected = lines
        .iter()
        .enumerate()
        .filter_map(|(idx, line)| {
            let line_no = idx + 1;
            if line_no >= start_line && line_no <= end_line {
                Some(*line)
            } else {
                None
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(json!({
        "path": path.display().to_string(),
        "content": selected,
        "start_line": start_line,
        "end_line": end_line.min(total_lines),
        "total_lines": total_lines,
        "content_hash": full_hash,
    }))
}

pub fn file_read_many(args: &Value) -> Result<Value, String> {
    let paths = args
        .get("paths")
        .and_then(Value::as_array)
        .ok_or_else(|| "paths_required".to_string())?;
    let mut results = Vec::new();
    for path in paths.iter().take(20) {
        let path = path
            .as_str()
            .ok_or_else(|| "paths_must_be_strings".to_string())?;
        results.push(file_read(&json!({"path": path}))?);
    }
    Ok(json!({ "files": results }))
}
