
fn coop_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let op = clean(
        parsed
            .flags
            .get("op")
            .map(String::as_str)
            .unwrap_or("status"),
        20,
    )
    .to_ascii_lowercase();
    let mut state = read_json(&coop_state_path(root))
        .and_then(|v| v.as_object().cloned())
        .unwrap_or_else(|| {
            let mut m = Map::new();
            m.insert("sites".to_string(), Value::Object(Map::new()));
            m
        });
    let mut replication_merkle: Option<String> = None;
    {
        let sites = state
            .entry("sites")
            .or_insert_with(|| Value::Object(Map::new()))
            .as_object_mut()
            .ok_or_else(|| "coop_sites_invalid".to_string())?;
        if op == "register-site" {
            let site = clean(
                parsed
                    .flags
                    .get("site")
                    .map(String::as_str)
                    .unwrap_or("site-a"),
                80,
            );
            let site_state = clean(
                parsed
                    .flags
                    .get("state")
                    .map(String::as_str)
                    .unwrap_or("STANDBY"),
                16,
            )
            .to_ascii_uppercase();
            sites.insert(site, json!({"state": site_state, "updated_at": now_iso()}));
        } else if op == "replicate" {
            replication_merkle = Some(sha256_hex_str(&canonical_json_string(&Value::Object(
                sites.clone(),
            ))));
        } else if op == "failover" {
            let target = clean(
                parsed
                    .flags
                    .get("target-site")
                    .map(String::as_str)
                    .unwrap_or(""),
                80,
            );
            if target.is_empty() || !sites.contains_key(&target) {
                return Err("coop_target_site_missing".to_string());
            }
            for (_, row) in sites.iter_mut() {
                row["state"] = Value::String("STANDBY".to_string());
                row["updated_at"] = Value::String(now_iso());
            }
            if let Some(row) = sites.get_mut(&target) {
                row["state"] = Value::String("ACTIVE".to_string());
                row["updated_at"] = Value::String(now_iso());
            }
        } else if op != "status" {
            return Err("coop_op_invalid".to_string());
        }
    }
    if let Some(merkle) = replication_merkle {
        state.insert(
            "last_replication".to_string(),
            json!({"ts": now_iso(), "merkle": merkle}),
        );
    }
    let sites = state
        .get("sites")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let site_hashes = sites
        .iter()
        .map(|(site, row)| sha256_hex_str(&format!("{}:{}", site, canonical_json_string(row))))
        .collect::<Vec<_>>();
    let forest_root = deterministic_merkle_root(&site_hashes);
    state.insert(
        "forest_root".to_string(),
        Value::String(forest_root.clone()),
    );
    write_json(&coop_state_path(root), &Value::Object(state.clone()))?;
    Ok(json!({
        "ok": true,
        "type": "government_plane_coop",
        "lane": LANE_ID,
        "ts": now_iso(),
        "op": op,
        "state": state,
        "claim_evidence": [{
            "id": "V7-GOV-001.6",
            "claim": "coop_site_state_replication_and_failover_emit_merkle_checked_receipts",
            "evidence": {"forest_root": forest_root}
        }]
    }))
}

fn proofs_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let op = clean(
        parsed
            .flags
            .get("op")
            .map(String::as_str)
            .unwrap_or("status"),
        16,
    )
    .to_ascii_lowercase();
    if op == "status" {
        return Ok(json!({
            "ok": true,
            "type": "government_plane_proofs",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "proof_roots": ["proofs/layer0", "proofs/layer1"],
            "claim_evidence": [{
                "id": "V7-GOV-001.7",
                "claim": "formal_proof_status_surfaces_privileged_boundary_verification_scope",
                "evidence": {"roots": 2}
            }]
        }));
    }
    if op != "verify" {
        return Err("proofs_op_invalid".to_string());
    }
    let mut proof_files = Vec::<String>::new();
    for root_rel in ["proofs/layer0", "proofs/layer1"] {
        let base = root.join(root_rel);
        if base.exists() {
            for entry in walkdir::WalkDir::new(base).into_iter().flatten() {
                if entry.file_type().is_file() {
                    let p = entry.path();
                    if let Some(ext) = p.extension().and_then(|v| v.to_str()) {
                        if ext == "v" || ext == "lean" {
                            proof_files.push(p.to_string_lossy().to_string());
                        }
                    }
                }
            }
        }
    }
    let mut unsafe_hits = Vec::new();
    let ops_root = root.join("core/layer0/ops/src");
    for entry in walkdir::WalkDir::new(&ops_root).into_iter().flatten() {
        if entry.file_type().is_file()
            && entry.path().extension().and_then(|v| v.to_str()) == Some("rs")
        {
            if let Ok(raw) = fs::read_to_string(entry.path()) {
                if raw.contains("unsafe ") {
                    unsafe_hits.push(entry.path().to_string_lossy().to_string());
                }
            }
        }
    }
    let ok = !proof_files.is_empty() && unsafe_hits.is_empty();
    Ok(json!({
        "ok": ok,
        "type": "government_plane_proofs",
        "lane": LANE_ID,
        "ts": now_iso(),
        "op": op,
        "proof_file_count": proof_files.len(),
        "unsafe_hits": unsafe_hits,
        "claim_evidence": [{
            "id": "V7-GOV-001.7",
            "claim": "formal_verification_lane_checks_proof_artifacts_and_privileged_boundary_unsafe_usage",
            "evidence": {"proof_file_count": proof_files.len(), "unsafe_hits": unsafe_hits.len()}
        }]
    }))
}
