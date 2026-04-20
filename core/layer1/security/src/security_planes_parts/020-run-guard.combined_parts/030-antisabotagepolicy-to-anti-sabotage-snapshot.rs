
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct AntiSabotagePolicy {
    version: String,
    protected_roots: Vec<String>,
    extensions: Vec<String>,
    state_dir: String,
    quarantine_dir: String,
    snapshots_dir: String,
    incident_log: String,
    state_file: String,
    watcher_state_file: String,
    watcher_interval_ms: i64,
    max_snapshots: usize,
    verify_strict_default: bool,
    auto_reset_default: bool,
    watcher_strict_default: bool,
    watcher_auto_reset_default: bool,
}

impl Default for AntiSabotagePolicy {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            protected_roots: vec![
                "systems".to_string(),
                "config".to_string(),
                "lib".to_string(),
                "adaptive".to_string(),
            ],
            extensions: vec![
                ".js".to_string(),
                ".ts".to_string(),
                ".json".to_string(),
                ".yaml".to_string(),
                ".yml".to_string(),
            ],
            state_dir: "local/state/security/anti_sabotage".to_string(),
            quarantine_dir: "local/state/security/anti_sabotage/quarantine".to_string(),
            snapshots_dir: "local/state/security/anti_sabotage/snapshots".to_string(),
            incident_log: "local/state/security/anti_sabotage/incidents.jsonl".to_string(),
            state_file: "local/state/security/anti_sabotage/state.json".to_string(),
            watcher_state_file: "local/state/security/anti_sabotage/watcher_state.json".to_string(),
            watcher_interval_ms: 30_000,
            max_snapshots: 20,
            verify_strict_default: true,
            auto_reset_default: true,
            watcher_strict_default: false,
            watcher_auto_reset_default: true,
        }
    }
}

fn load_anti_sabotage_policy(repo_root: &Path, parsed: &ParsedArgs) -> AntiSabotagePolicy {
    let policy_path = flag(parsed, "policy")
        .map(|v| resolve_runtime_or_state(repo_root, v))
        .unwrap_or_else(|| runtime_config_path(repo_root, "anti_sabotage_policy.json"));
    if !policy_path.exists() {
        return AntiSabotagePolicy::default();
    }
    match fs::read_to_string(&policy_path) {
        Ok(raw) => serde_json::from_str::<AntiSabotagePolicy>(&raw).unwrap_or_default(),
        Err(_) => AntiSabotagePolicy::default(),
    }
}

fn anti_sabotage_walk_files(
    repo_root: &Path,
    policy: &AntiSabotagePolicy,
) -> Vec<(String, PathBuf)> {
    let runtime = runtime_root(repo_root);
    let ext_set = policy
        .extensions
        .iter()
        .map(|v| {
            let c = clean(v, 16).to_ascii_lowercase();
            if c.starts_with('.') {
                c
            } else {
                format!(".{c}")
            }
        })
        .collect::<BTreeSet<_>>();
    let mut out = Vec::<(String, PathBuf)>::new();
    for rel_root in &policy.protected_roots {
        let root = runtime.join(normalize_rel(rel_root));
        if !root.exists() {
            continue;
        }
        for entry in WalkDir::new(&root)
            .follow_links(false)
            .into_iter()
            .filter_map(Result::ok)
        {
            if !entry.file_type().is_file() {
                continue;
            }
            let path = entry.path();
            let ext = path
                .extension()
                .and_then(|v| v.to_str())
                .map(|v| format!(".{}", v.to_ascii_lowercase()))
                .unwrap_or_default();
            if !ext_set.is_empty() && !ext_set.contains(&ext) {
                continue;
            }
            let rel = path
                .strip_prefix(&runtime)
                .ok()
                .map(|v| normalize_rel(v.to_string_lossy()))
                .unwrap_or_else(|| normalize_rel(path.to_string_lossy()));
            out.push((rel, path.to_path_buf()));
        }
    }
    out.sort_by(|a, b| a.0.cmp(&b.0));
    out
}

fn anti_sabotage_paths(
    repo_root: &Path,
    policy: &AntiSabotagePolicy,
) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    (
        resolve_runtime_or_state(repo_root, &policy.state_file),
        resolve_runtime_or_state(repo_root, &policy.incident_log),
        resolve_runtime_or_state(repo_root, &policy.snapshots_dir),
        resolve_runtime_or_state(repo_root, &policy.watcher_state_file),
    )
}

fn anti_sabotage_latest_snapshot_id(snapshots_dir: &Path) -> Option<String> {
    let entries = fs::read_dir(snapshots_dir).ok()?;
    let mut names = entries
        .filter_map(Result::ok)
        .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
        .filter_map(|e| e.file_name().into_string().ok())
        .collect::<Vec<_>>();
    names.sort();
    names.pop()
}

fn anti_sabotage_snapshot(
    repo_root: &Path,
    policy: &AntiSabotagePolicy,
    label: Option<&str>,
) -> Result<Value, String> {
    let (_state_path, _incident_path, snapshots_dir, _watcher_state_path) =
        anti_sabotage_paths(repo_root, policy);
    fs::create_dir_all(&snapshots_dir).map_err(|err| {
        format!(
            "create_snapshots_dir_failed:{}:{err}",
            snapshots_dir.display()
        )
    })?;
    let ts = Utc::now().format("%Y%m%d%H%M%S").to_string();
    let suffix = label
        .map(|v| clean(v, 40).replace(' ', "_"))
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| "manual".to_string());
    let snapshot_id = format!("{ts}_{suffix}");
    let snapshot_dir = snapshots_dir.join(&snapshot_id);
    let files_dir = snapshot_dir.join("files");
    fs::create_dir_all(&files_dir)
        .map_err(|err| format!("create_snapshot_files_failed:{}:{err}", files_dir.display()))?;
    let monitored = anti_sabotage_walk_files(repo_root, policy);
    let mut hashes = Map::<String, Value>::new();
    for (rel, abs) in monitored {
        let digest = sha256_hex_file(&abs)?;
        let storage = files_dir.join(&rel);
        if let Some(parent) = storage.parent() {
            fs::create_dir_all(parent).map_err(|err| {
                format!("create_snapshot_parent_failed:{}:{err}", parent.display())
            })?;
        }
        fs::copy(&abs, &storage)
            .map_err(|err| format!("copy_snapshot_file_failed:{}:{err}", abs.display()))?;
        hashes.insert(
            rel.clone(),
            json!({
                "hash": digest,
                "storage_path": normalize_rel(storage.strip_prefix(&snapshot_dir).unwrap_or(&storage).to_string_lossy())
            }),
        );
    }
    let manifest = json!({
        "version": policy.version,
        "snapshot_id": snapshot_id,
        "created_at": now_iso(),
        "hashes": hashes
    });
    write_json_atomic(&snapshot_dir.join("manifest.json"), &manifest)?;
    Ok(json!({
        "ok": true,
        "type": "anti_sabotage_snapshot",
        "snapshot_id": manifest.get("snapshot_id").cloned().unwrap_or(Value::Null),
        "manifest_path": snapshot_dir.join("manifest.json").to_string_lossy(),
        "files": manifest.get("hashes").and_then(Value::as_object).map(|m| m.len()).unwrap_or(0)
    }))
}
