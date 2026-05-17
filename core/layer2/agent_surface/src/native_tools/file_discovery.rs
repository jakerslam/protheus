use crate::native_tools::paths::required_abs_path;
use serde_json::{json, Value};
use std::fs;
use std::io::ErrorKind;
use std::path::{Path, PathBuf};

pub fn file_stat(args: &Value) -> Result<Value, String> {
    let path = required_abs_path(args)?;
    let parent_exists = path.parent().map(Path::exists).unwrap_or(false);
    match fs::symlink_metadata(&path) {
        Ok(metadata) => Ok(json!({
            "path": path.display().to_string(),
            "exists": true,
            "kind": metadata_kind(&metadata),
            "size_bytes": metadata.len(),
            "readonly": metadata.permissions().readonly(),
            "parent_exists": parent_exists,
        })),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(json!({
            "path": path.display().to_string(),
            "exists": false,
            "kind": "missing",
            "size_bytes": null,
            "readonly": null,
            "parent_exists": parent_exists,
        })),
        Err(error) => Err(format!("file_stat_failed:{error}")),
    }
}

pub fn file_list(args: &Value) -> Result<Value, String> {
    let root = required_abs_path(args)?;
    let recursive = args
        .get("recursive")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let include_hidden = args
        .get("include_hidden")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let max_entries = args
        .get("max_entries")
        .and_then(Value::as_u64)
        .unwrap_or(200)
        .clamp(1, 1000) as usize;
    let max_depth = if recursive {
        args.get("max_depth")
            .and_then(Value::as_u64)
            .unwrap_or(3)
            .clamp(1, 8) as usize
    } else {
        1
    };

    let root_metadata =
        fs::symlink_metadata(&root).map_err(|error| format!("file_list_failed:{error}"))?;
    if !root_metadata.is_dir() {
        return Err("file_list_path_must_be_directory".to_string());
    }

    let mut entries = Vec::<Value>::new();
    let mut dirs = vec![(root.clone(), 0usize)];
    while let Some((dir, depth)) = dirs.pop() {
        if entries.len() >= max_entries || depth >= max_depth {
            continue;
        }
        let mut children = fs::read_dir(&dir)
            .map_err(|error| format!("file_list_read_dir_failed:{error}"))?
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .collect::<Vec<PathBuf>>();
        children.sort();
        for path in children {
            if entries.len() >= max_entries {
                break;
            }
            if !include_hidden && is_hidden(&path) {
                continue;
            }
            let metadata = match fs::symlink_metadata(&path) {
                Ok(metadata) => metadata,
                Err(_) => continue,
            };
            let kind = metadata_kind(&metadata);
            let relative_path = path
                .strip_prefix(&root)
                .map(|value| value.display().to_string())
                .unwrap_or_else(|_| path.display().to_string());
            let size_bytes = if metadata.is_file() {
                json!(metadata.len())
            } else {
                Value::Null
            };
            entries.push(json!({
                "path": path.display().to_string(),
                "relative_path": relative_path,
                "kind": kind,
                "size_bytes": size_bytes,
                "readonly": metadata.permissions().readonly(),
            }));
            if recursive && metadata.is_dir() && !metadata.file_type().is_symlink() {
                dirs.push((path, depth + 1));
            }
        }
    }

    Ok(json!({
        "path": root.display().to_string(),
        "recursive": recursive,
        "max_depth": max_depth,
        "max_entries": max_entries,
        "truncated": entries.len() >= max_entries,
        "entries": entries,
    }))
}

fn is_hidden(path: &Path) -> bool {
    path.file_name()
        .and_then(|value| value.to_str())
        .map(|name| name.starts_with('.'))
        .unwrap_or(false)
}

fn metadata_kind(metadata: &fs::Metadata) -> &'static str {
    let file_type = metadata.file_type();
    if file_type.is_symlink() {
        "symlink"
    } else if metadata.is_file() {
        "file"
    } else if metadata.is_dir() {
        "directory"
    } else {
        "other"
    }
}
