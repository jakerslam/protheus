
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
