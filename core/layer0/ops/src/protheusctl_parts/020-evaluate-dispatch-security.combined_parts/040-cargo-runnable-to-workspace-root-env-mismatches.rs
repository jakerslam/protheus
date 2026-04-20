
fn cargo_runnable() -> bool {
    Command::new("cargo")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn rustup_detected() -> bool {
    Command::new("rustup")
        .arg("--version")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

fn rustup_default_toolchain_configured() -> bool {
    Command::new("rustup")
        .arg("default")
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn canonical_path_string(path: &Path) -> String {
    fs::canonicalize(path)
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_string()
}

fn workspace_root_env_mismatches(root: &Path) -> Vec<Value> {
    let active = canonical_path_string(root);
    let mut rows = Vec::<Value>::new();
    for key in ["INFRING_WORKSPACE_ROOT", "PROTHEUS_WORKSPACE_ROOT"] {
        let Ok(raw) = env::var(key) else {
            continue;
        };
        let trimmed = raw.trim();
        if trimmed.is_empty() {
            continue;
        }
        let configured = Path::new(trimmed);
        let resolved = canonical_path_string(configured);
        if resolved != active {
            rows.push(json!({
                "env": key,
                "configured": clean(trimmed.to_string(), 500),
                "resolved": clean(resolved, 500),
                "active_workspace_root": clean(active.clone(), 500)
            }));
        }
    }
    rows
}
