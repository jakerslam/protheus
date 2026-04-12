const MEDIA_STORE_DIR_REL: &str = "client/runtime/local/state/web_conduit/stored_media";
const DEFAULT_MEDIA_STORE_TTL_MS: u64 = 120_000;
const MEDIA_STORE_FILE_MODE: u32 = 0o644;

fn media_store_dir_path(root: &Path, subdir: &str) -> PathBuf {
    root.join(MEDIA_STORE_DIR_REL).join(clean_text(subdir, 80))
}

fn media_store_error(error: &str, detail: &str) -> Value {
    json!({
        "ok": false,
        "error": clean_text(error, 120),
        "detail": clean_text(detail, 240)
    })
}

fn sanitize_saved_media_base(raw: &str) -> String {
    raw.trim()
        .chars()
        .map(|ch| {
            if ch.is_alphanumeric() || matches!(ch, '.' | '_' | '-') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
        .chars()
        .take(60)
        .collect::<String>()
}

fn media_saved_id(file_name: &str, content_type: &str) -> String {
    let base_seed = format!(
        "{}:{}:{}:{}",
        clean_text(file_name, 220),
        clean_text(content_type, 120),
        std::process::id(),
        Utc::now().timestamp_millis()
    );
    let hash = sha256_hex(&base_seed);
    let ext = Path::new(file_name)
        .extension()
        .and_then(|row| row.to_str())
        .map(|row| row.to_ascii_lowercase())
        .or_else(|| media_extension_for_content_type(content_type).map(|row| row.to_string()))
        .unwrap_or_else(|| "bin".to_string());
    let base_name = Path::new(file_name)
        .file_stem()
        .and_then(|row| row.to_str())
        .map(sanitize_saved_media_base)
        .unwrap_or_default();
    if base_name.is_empty() {
        format!("{}.{}", &hash[..24], ext)
    } else {
        format!("{base_name}---{}.{}", &hash[..24], ext)
    }
}

fn media_store_saved_row(
    path: &Path,
    id: &str,
    bytes: usize,
    content_type: &str,
    file_name: &str,
    subdir: &str,
) -> Value {
    json!({
        "id": clean_text(id, 220),
        "path": path.display().to_string(),
        "bytes": bytes,
        "content_type": normalize_media_content_type(content_type),
        "file_name": clean_text(file_name, 220),
        "subdir": clean_text(subdir, 80)
    })
}

fn write_media_store_bytes(path: &Path, bytes: &[u8]) -> Result<(), Value> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|err| {
            media_store_error("media_store_dir_create_failed", &err.to_string())
        })?;
    }
    let write_once = || fs::write(path, bytes);
    match write_once() {
        Ok(_) => {}
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent).map_err(|create_err| {
                    media_store_error("media_store_dir_create_failed", &create_err.to_string())
                })?;
            }
            write_once().map_err(|retry_err| {
                media_store_error("media_store_write_failed", &retry_err.to_string())
            })?;
        }
        Err(err) => {
            return Err(media_store_error("media_store_write_failed", &err.to_string()));
        }
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(path, fs::Permissions::from_mode(MEDIA_STORE_FILE_MODE));
    }
    Ok(())
}

fn store_media_artifact_copy(
    root: &Path,
    artifact_path: &Path,
    file_name: &str,
    content_type: &str,
    bytes: usize,
    subdir: &str,
) -> Result<Value, Value> {
    let dir = media_store_dir_path(root, subdir);
    fs::create_dir_all(&dir).map_err(|err| {
        media_store_error("media_store_dir_create_failed", &err.to_string())
    })?;
    let id = media_saved_id(file_name, content_type);
    let dest = dir.join(&id);
    fs::copy(artifact_path, &dest)
        .map_err(|err| media_store_error("media_store_copy_failed", &err.to_string()))?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let _ = fs::set_permissions(&dest, fs::Permissions::from_mode(MEDIA_STORE_FILE_MODE));
    }
    Ok(media_store_saved_row(
        &dest,
        &id,
        bytes,
        content_type,
        file_name,
        subdir,
    ))
}

fn save_media_buffer_to_store(
    root: &Path,
    buffer: &[u8],
    content_type: Option<&str>,
    subdir: &str,
    max_bytes: usize,
    original_file_name: Option<&str>,
) -> Result<Value, Value> {
    if buffer.len() > max_bytes {
        return Err(json!({
            "ok": false,
            "error": "too-large",
            "message": format!("Media exceeds {}MB limit", max_bytes / (1024 * 1024)),
            "bytes": buffer.len(),
            "max_bytes": max_bytes
        }));
    }
    let header_type = content_type.filter(|row| !row.trim().is_empty());
    let file_name = original_file_name
        .map(|row| clean_text(row, 220))
        .filter(|row| !row.is_empty())
        .unwrap_or_else(|| {
            media_extension_for_content_type(header_type.unwrap_or("application/octet-stream"))
                .map(|ext| format!("media.{ext}"))
                .unwrap_or_else(|| "media.bin".to_string())
        });
    let detected = media_guess_content_type(Some(&file_name), buffer, header_type);
    let id = media_saved_id(&file_name, &detected);
    let path = media_store_dir_path(root, subdir).join(&id);
    write_media_store_bytes(&path, buffer)?;
    Ok(media_store_saved_row(
        &path,
        &id,
        buffer.len(),
        &detected,
        &file_name,
        subdir,
    ))
}

fn media_store_extract_original_filename(file_path: &str) -> String {
    let basename = Path::new(file_path)
        .file_name()
        .and_then(|row| row.to_str())
        .unwrap_or("")
        .trim();
    if basename.is_empty() {
        return "file.bin".to_string();
    }
    let ext = Path::new(basename)
        .extension()
        .and_then(|row| row.to_str())
        .map(|row| format!(".{row}"))
        .unwrap_or_default();
    let stem = Path::new(basename)
        .file_stem()
        .and_then(|row| row.to_str())
        .unwrap_or("");
    static SAVED_NAME_RE: OnceLock<Regex> = OnceLock::new();
    let re = SAVED_NAME_RE.get_or_init(|| {
        Regex::new(r"^(?P<base>.+)---(?P<id>([a-f0-9]{24}|[a-f0-9]{8}(?:-[a-f0-9]{4}){3}-[a-f0-9]{12}))$")
            .expect("saved name regex")
    });
    if let Some(caps) = re.captures(stem) {
        return format!(
            "{}{}",
            caps.name("base").map(|row| row.as_str()).unwrap_or("file"),
            ext
        );
    }
    basename.to_string()
}

fn media_store_id_is_safe(id: &str) -> bool {
    !id.is_empty() && !id.contains('/') && !id.contains('\\') && !id.contains('\0') && id != ".."
}

fn resolve_saved_media_path(root: &Path, id: &str, subdir: &str) -> Result<PathBuf, Value> {
    let clean_id = clean_text(id, 220);
    if !media_store_id_is_safe(&clean_id) {
        return Err(json!({
            "ok": false,
            "error": "invalid-path",
            "message": format!("unsafe stored media id: {:?}", clean_id)
        }));
    }
    let dir = media_store_dir_path(root, subdir);
    let resolved = dir.join(&clean_id);
    if !resolved.starts_with(&dir) {
        return Err(json!({
            "ok": false,
            "error": "invalid-path",
            "message": format!("stored media id escapes directory: {:?}", clean_id)
        }));
    }
    let stat = fs::symlink_metadata(&resolved).map_err(|_| {
        json!({
            "ok": false,
            "error": "not-found",
            "message": "Stored media path does not exist"
        })
    })?;
    if stat.file_type().is_symlink() {
        return Err(json!({
            "ok": false,
            "error": "invalid-path",
            "message": format!("refusing to follow symlink for stored media id: {:?}", clean_id)
        }));
    }
    if !stat.is_file() {
        return Err(json!({
            "ok": false,
            "error": "not-file",
            "message": format!("stored media id does not resolve to a file: {:?}", clean_id)
        }));
    }
    Ok(resolved)
}

fn delete_saved_media(root: &Path, id: &str, subdir: &str) -> Result<Value, Value> {
    let resolved = resolve_saved_media_path(root, id, subdir)?;
    fs::remove_file(&resolved)
        .map_err(|err| media_store_error("media_store_delete_failed", &err.to_string()))?;
    Ok(json!({
        "ok": true,
        "id": clean_text(id, 220),
        "subdir": clean_text(subdir, 80),
        "deleted_path": resolved.display().to_string()
    }))
}

fn clean_old_media_store(root: &Path, ttl_ms: u64, recursive: bool, prune_empty_dirs: bool) -> Result<(), Value> {
    let dir = media_store_dir_path(root, "");
    fs::create_dir_all(&dir).map_err(|err| media_store_error("media_store_dir_create_failed", &err.to_string()))?;
    let now = std::time::SystemTime::now();

    fn remove_expired(dir: &Path, now: std::time::SystemTime, ttl_ms: u64, recursive: bool, prune_empty_dirs: bool) {
        let entries = match fs::read_dir(dir) {
            Ok(rows) => rows,
            Err(_) => return,
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let stat = match fs::symlink_metadata(&path) {
                Ok(row) => row,
                Err(_) => continue,
            };
            if stat.file_type().is_symlink() {
                continue;
            }
            if stat.is_dir() {
                if recursive {
                    remove_expired(&path, now, ttl_ms, recursive, prune_empty_dirs);
                    if prune_empty_dirs {
                        let empty = fs::read_dir(&path)
                            .ok()
                            .and_then(|mut rows| rows.next())
                            .is_none();
                        if empty {
                            let _ = fs::remove_dir(&path);
                        }
                    }
                }
                continue;
            }
            if !stat.is_file() {
                continue;
            }
            let expired = stat
                .modified()
                .ok()
                .and_then(|modified| now.duration_since(modified).ok())
                .map(|age| age.as_millis() as u64 > ttl_ms)
                .unwrap_or(false);
            if expired {
                let _ = fs::remove_file(&path);
            }
        }
    }

    remove_expired(&dir, now, ttl_ms, recursive, prune_empty_dirs);
    Ok(())
}

fn map_media_store_source_error(raw_source: &str, error_payload: &Value) -> Value {
    let code = clean_text(
        error_payload.get("error").and_then(Value::as_str).unwrap_or("media_store_failed"),
        120,
    );
    if raw_source.starts_with("http://") || raw_source.starts_with("https://") {
        return error_payload.clone();
    }
    match code.as_str() {
        "path-not-allowed" | "invalid-path" => json!({
            "ok": false,
            "error": "invalid-path",
            "message": "Media path is outside workspace root"
        }),
        "not-found" => json!({
            "ok": false,
            "error": "not-found",
            "message": "Media path does not exist"
        }),
        _ => error_payload.clone(),
    }
}

fn media_store_contract() -> Value {
    json!({
        "root": MEDIA_STORE_DIR_REL,
        "default_ttl_ms": DEFAULT_MEDIA_STORE_TTL_MS,
        "saved_id_shape": "<sanitized-base>---<hash24>.<ext>",
        "resolve_rejects": ["slash", "backslash", "null_byte", "double_dot", "symlink", "non_file"],
        "subdirs": ["outbound", "inbound"]
    })
}
