
fn wrapper_candidate_path(wrapper_name: &str) -> String {
    let file_name = if cfg!(windows) {
        format!("{wrapper_name}.cmd")
    } else {
        wrapper_name.to_string()
    };
    if let Ok(exe) = env::current_exe() {
        if let Some(parent) = exe.parent() {
            return clean(parent.join(file_name).to_string_lossy().to_string(), 500);
        }
    }
    clean(file_name, 500)
}

fn resolve_executable_path(bin_name: &str) -> Option<String> {
    let path_var = env::var_os("PATH")?;
    let mut candidates = vec![bin_name.to_string()];
    if cfg!(windows) {
        for ext in [".exe", ".cmd", ".bat"] {
            candidates.push(format!("{bin_name}{ext}"));
        }
    }
    for dir in env::split_paths(&path_var) {
        for candidate_name in &candidates {
            let candidate = dir.join(candidate_name);
            if candidate.is_file() {
                return Some(clean(candidate.to_string_lossy().to_string(), 500));
            }
        }
    }
    None
}

fn runtime_manifest_status(root: &Path, runtime_mode: &str, missing_entrypoints_count: usize) -> Value {
    let manifest_path = root.join(INSTALL_RUNTIME_MANIFEST_REL);
    let manifest_raw = std::fs::read_to_string(&manifest_path).ok();
    let declared_entry_count = manifest_raw
        .as_deref()
        .map(|raw| {
            raw.lines()
                .map(str::trim)
                .filter(|line| !line.is_empty() && !line.starts_with('#'))
                .count()
        })
        .unwrap_or(0usize);
    json!({
        "manifest_rel": INSTALL_RUNTIME_MANIFEST_REL,
        "manifest_path": clean(manifest_path.to_string_lossy().to_string(), 500),
        "manifest_exists": manifest_path.is_file(),
        "runtime_mode": runtime_mode,
        "declared_entry_count": declared_entry_count,
        "missing_entrypoints_count": missing_entrypoints_count
    })
}

fn port_availability_status(host: &str, port: Option<u16>) -> Value {
    if let Some(port_value) = port {
        let bind_target = format!("{host}:{port_value}");
        match std::net::TcpListener::bind(bind_target.as_str()) {
            Ok(listener) => {
                drop(listener);
                json!({
                    "host": host,
                    "port": port_value,
                    "parse_ok": true,
                    "bind_available": true,
                    "status": "available"
                })
            }
            Err(err) => {
                let status = if err.kind() == std::io::ErrorKind::AddrInUse {
                    "in_use"
                } else {
                    "unavailable"
                };
                json!({
                    "host": host,
                    "port": port_value,
                    "parse_ok": true,
                    "bind_available": false,
                    "status": status,
                    "error_kind": format!("{:?}", err.kind()),
                    "error": clean(err.to_string(), 220)
                })
            }
        }
    } else {
        json!({
            "host": host,
            "parse_ok": false,
            "bind_available": false,
            "status": "invalid_port"
        })
    }
}
