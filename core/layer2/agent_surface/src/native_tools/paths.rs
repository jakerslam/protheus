use serde_json::Value;
use std::path::PathBuf;

pub fn required_abs_path(args: &Value) -> Result<PathBuf, String> {
    let path = args
        .get("path")
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "path_required".to_string())?;
    let path = PathBuf::from(path);
    if !path.is_absolute() {
        return Err("absolute_path_required".to_string());
    }
    Ok(path)
}
