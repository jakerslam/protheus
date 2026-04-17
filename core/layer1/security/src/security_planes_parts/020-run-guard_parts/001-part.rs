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

fn anti_sabotage_verify(
    repo_root: &Path,
    policy: &AntiSabotagePolicy,
    snapshot_ref: &str,
    strict: bool,
    auto_reset: bool,
) -> Result<(Value, i32), String> {
    let (_state_path, incident_path, snapshots_dir, _watcher_state_path) =
        anti_sabotage_paths(repo_root, policy);
    let snapshot_id = if snapshot_ref == "latest" || snapshot_ref.is_empty() {
        anti_sabotage_latest_snapshot_id(&snapshots_dir).unwrap_or_default()
    } else {
        clean(snapshot_ref, 120)
    };
    if snapshot_id.is_empty() {
        return Ok((
            json!({
                "ok": false,
                "type": "anti_sabotage_verify",
                "error": "snapshot_missing",
                "snapshot": snapshot_ref
            }),
            if strict { 1 } else { 0 },
        ));
    }
    let snapshot_dir = snapshots_dir.join(&snapshot_id);
    let manifest_path = snapshot_dir.join("manifest.json");
    let manifest = read_json_or(&manifest_path, Value::Null);
    let expected = manifest
        .get("hashes")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let monitored = anti_sabotage_walk_files(repo_root, policy);
    let mut current = BTreeMap::<String, String>::new();
    for (rel, abs) in monitored {
        if let Ok(hash) = sha256_hex_file(&abs) {
            current.insert(rel, hash);
        }
    }

    let mut mismatch = Vec::<Value>::new();
    let mut missing = Vec::<Value>::new();
    let mut extra = Vec::<Value>::new();

    for (rel, row) in &expected {
        let want = clean(
            row.get("hash")
                .or_else(|| row.get("sha256"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            160,
        );
        if want.is_empty() {
            continue;
        }
        match current.get(rel) {
            None => missing.push(json!({"file": rel})),
            Some(have) => {
                if have != &want {
                    mismatch.push(json!({"file": rel, "expected": want, "current": have}));
                }
            }
        }
    }
    for rel in current.keys() {
        if !expected.contains_key(rel) {
            extra.push(json!({"file": rel}));
        }
    }

    let violated = !mismatch.is_empty() || !missing.is_empty() || !extra.is_empty();
    let mut restored = Vec::<Value>::new();
    if violated && auto_reset {
        for row in mismatch.iter().chain(missing.iter()) {
            let rel = row.get("file").and_then(Value::as_str).unwrap_or("");
            if rel.is_empty() {
                continue;
            }
            let storage_rel = expected
                .get(rel)
                .and_then(|v| v.get("storage_path"))
                .and_then(Value::as_str)
                .unwrap_or("");
            if storage_rel.is_empty() {
                continue;
            }
            let src = snapshot_dir.join(storage_rel);
            let dst = runtime_root(repo_root).join(rel);
            if !src.exists() {
                continue;
            }
            if let Some(parent) = dst.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if fs::copy(&src, &dst).is_ok() {
                restored.push(json!({"file": rel, "restored": true}));
            }
        }
    }
    let rollback_plan_hash = sha256_hex_bytes(
        stable_json_string(&json!({
            "snapshot_id": snapshot_id,
            "mismatch": mismatch,
            "missing": missing
        }))
        .as_bytes(),
    );
    let incident = json!({
        "ts": now_iso(),
        "type": "anti_sabotage_verify",
        "snapshot_id": snapshot_id,
        "violated": violated,
        "mismatch_count": mismatch.len(),
        "missing_count": missing.len(),
        "extra_count": extra.len(),
        "restored_count": restored.len(),
        "rollback_plan": {
            "plan_hash": rollback_plan_hash
        }
    });
    let _ = append_jsonl(&incident_path, &incident);
    let code = if violated && strict { 1 } else { 0 };
    Ok((
        json!({
            "ok": !violated,
            "type": "anti_sabotage_verify",
            "snapshot_id": snapshot_id,
            "violated": violated,
            "strict": strict,
            "auto_reset": auto_reset,
            "mismatch": mismatch,
            "missing": missing,
            "extra": extra,
            "restored": restored,
            "rollback_plan_hash": rollback_plan_hash
        }),
        code,
    ))
}

fn anti_sabotage_status(repo_root: &Path, policy: &AntiSabotagePolicy) -> Value {
    let (state_path, incident_path, snapshots_dir, watcher_state_path) =
        anti_sabotage_paths(repo_root, policy);
    let latest_snapshot_id = anti_sabotage_latest_snapshot_id(&snapshots_dir);
    let latest_snapshot_manifest = latest_snapshot_id
        .as_ref()
        .map(|id| snapshots_dir.join(id).join("manifest.json"))
        .filter(|p| p.exists())
        .map(|p| normalize_rel(p.to_string_lossy()));

    let latest_incident_summary = fs::read_to_string(&incident_path)
        .ok()
        .and_then(|raw| parse_last_json_line(&raw))
        .map(|v| {
            json!({
                "incident_id": v.get("incident_id").cloned().unwrap_or(Value::Null),
                "ts": v.get("ts").cloned().unwrap_or(Value::Null),
                "snapshot_id": v.get("snapshot_id").cloned().unwrap_or(Value::Null),
                "violated": v.get("violated").cloned().unwrap_or(Value::Null),
                "mismatch_count": v.get("mismatch_count").cloned().unwrap_or(Value::Null),
                "missing_count": v.get("missing_count").cloned().unwrap_or(Value::Null),
                "extra_count": v.get("extra_count").cloned().unwrap_or(Value::Null),
                "rollback_plan_hash": v
                    .get("rollback_plan")
                    .and_then(|r| r.get("plan_hash"))
                    .cloned()
                    .unwrap_or(Value::Null)
            })
        })
        .unwrap_or(Value::Null);

    json!({
        "ok": true,
        "type": "anti_sabotage_status",
        "ts": now_iso(),
        "policy_version": policy.version,
        "latest_snapshot": latest_snapshot_id,
        "latest_snapshot_manifest": latest_snapshot_manifest,
        "latest_incident_summary": latest_incident_summary,
        "state_path": normalize_rel(state_path.to_string_lossy()),
        "incident_log": normalize_rel(incident_path.to_string_lossy()),
        "watcher_state_path": normalize_rel(watcher_state_path.to_string_lossy()),
        "watcher_state": read_json_or(&watcher_state_path, Value::Null)
    })
}

pub fn run_anti_sabotage_shield(repo_root: &Path, argv: &[String]) -> (Value, i32) {
    let parsed = parse_args(argv);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let policy = load_anti_sabotage_policy(repo_root, &parsed);
    match cmd.as_str() {
        "snapshot" => match anti_sabotage_snapshot(repo_root, &policy, flag(&parsed, "label")) {
            Ok(out) => (out, 0),
            Err(err) => (
                json!({"ok": false, "type":"anti_sabotage_snapshot", "error": clean(err, 220)}),
                1,
            ),
        },
        "verify" => {
            let strict = bool_flag(&parsed, "strict", policy.verify_strict_default);
            let auto_reset = bool_flag(&parsed, "auto-reset", policy.auto_reset_default);
            let snapshot_ref = flag(&parsed, "snapshot").unwrap_or("latest");
            match anti_sabotage_verify(repo_root, &policy, snapshot_ref, strict, auto_reset) {
                Ok(result) => result,
                Err(err) => (
                    json!({"ok": false, "type":"anti_sabotage_verify", "error": clean(err, 220)}),
                    1,
                ),
            }
        }
        "watch" => {
            let strict = bool_flag(&parsed, "strict", policy.watcher_strict_default);
            let auto_reset = bool_flag(&parsed, "auto-reset", policy.watcher_auto_reset_default);
            let interval_ms = flag(&parsed, "interval-ms")
                .and_then(|v| v.parse::<u64>().ok())
                .unwrap_or(policy.watcher_interval_ms.max(250) as u64)
                .clamp(250, 300_000);
            let iterations = flag(&parsed, "iterations")
                .and_then(|v| v.parse::<usize>().ok())
                .unwrap_or(1)
                .clamp(1, 1000);
            if bool_flag(&parsed, "bootstrap-snapshot", false) {
                let _ = anti_sabotage_snapshot(repo_root, &policy, Some("watch-bootstrap"));
            }
            let snapshot_ref = flag(&parsed, "snapshot").unwrap_or("latest").to_string();
            let mut last = json!({"ok": true, "type": "anti_sabotage_watch", "iterations": 0});
            let mut last_code = 0;
            for idx in 0..iterations {
                match anti_sabotage_verify(repo_root, &policy, &snapshot_ref, strict, auto_reset) {
                    Ok((verify, code)) => {
                        last = json!({
                            "ok": verify.get("ok").and_then(Value::as_bool).unwrap_or(false),
                            "type": "anti_sabotage_watch",
                            "iteration": idx + 1,
                            "iterations": iterations,
                            "verify": verify
                        });
                        last_code = code;
                    }
                    Err(err) => {
                        last = json!({"ok": false, "type":"anti_sabotage_watch", "error": clean(err, 220)});
                        last_code = 1;
                    }
                }
                if idx + 1 < iterations {
                    thread::sleep(Duration::from_millis(interval_ms));
                }
            }
            (last, last_code)
        }
        "status" => (anti_sabotage_status(repo_root, &policy), 0),
        _ => (
            json!({
                "ok": false,
                "type": "anti_sabotage_shield",
                "error": "unknown_command",
                "usage": [
                    "anti-sabotage-shield snapshot [--label=<id>]",
                    "anti-sabotage-shield verify [--snapshot=latest|<id>] [--strict=1|0] [--auto-reset=1|0]",
                    "anti-sabotage-shield watch [--snapshot=latest|<id>] [--strict=1|0] [--auto-reset=1|0] [--interval-ms=<n>] [--iterations=<n>]",
                    "anti-sabotage-shield status"
                ]
            }),
            2,
        ),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct ConstitutionPolicy {
    version: String,
    constitution_path: String,
    state_dir: String,
    veto_window_days: i64,
    min_approval_note_chars: usize,
    require_dual_approval: bool,
    enforce_inheritance_lock: bool,
    emergency_rollback_requires_approval: bool,
}

impl Default for ConstitutionPolicy {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            constitution_path: "docs/workspace/AGENT-CONSTITUTION.md".to_string(),
            state_dir: "local/state/security/constitution_guardian".to_string(),
            veto_window_days: 14,
            min_approval_note_chars: 12,
            require_dual_approval: true,
            enforce_inheritance_lock: true,
            emergency_rollback_requires_approval: true,
        }
    }
}

fn load_constitution_policy(repo_root: &Path, parsed: &ParsedArgs) -> ConstitutionPolicy {
    let policy_path = flag(parsed, "policy")
        .map(|v| resolve_runtime_or_state(repo_root, v))
        .unwrap_or_else(|| runtime_config_path(repo_root, "constitution_guardian_policy.json"));
    if !policy_path.exists() {
        return ConstitutionPolicy::default();
    }
    match fs::read_to_string(&policy_path) {
        Ok(raw) => serde_json::from_str::<ConstitutionPolicy>(&raw).unwrap_or_default(),
        Err(_) => ConstitutionPolicy::default(),
    }
}

#[derive(Debug, Clone)]
struct ConstitutionPaths {
    constitution: PathBuf,
    state_dir: PathBuf,
    genesis: PathBuf,
    proposals_dir: PathBuf,
    events: PathBuf,
    history_dir: PathBuf,
    active_state: PathBuf,
}

fn constitution_paths(repo_root: &Path, policy: &ConstitutionPolicy) -> ConstitutionPaths {
    let constitution = resolve_runtime_or_state(repo_root, &policy.constitution_path);
    let state_dir = resolve_runtime_or_state(repo_root, &policy.state_dir);
    ConstitutionPaths {
        constitution,
        genesis: state_dir.join("genesis.json"),
        proposals_dir: state_dir.join("proposals"),
        events: state_dir.join("events.jsonl"),
        history_dir: state_dir.join("history"),
        active_state: state_dir.join("active_state.json"),
        state_dir,
    }
}

fn proposal_path(paths: &ConstitutionPaths, proposal_id: &str) -> PathBuf {
    paths.proposals_dir.join(proposal_id).join("proposal.json")
