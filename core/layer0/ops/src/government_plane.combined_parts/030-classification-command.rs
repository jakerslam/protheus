
fn classification_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let op = clean(
        parsed
            .flags
            .get("op")
            .map(String::as_str)
            .unwrap_or("status"),
        20,
    )
    .to_ascii_lowercase();
    let principal = clean(
        parsed
            .flags
            .get("principal")
            .map(String::as_str)
            .unwrap_or("operator"),
        120,
    );
    let mut clr = clearances(root);
    if op == "set-clearance" {
        let level = clean(
            parsed
                .flags
                .get("clearance")
                .map(String::as_str)
                .unwrap_or("unclassified"),
            32,
        )
        .to_ascii_lowercase();
        if level_rank(&level) < 0 {
            return Err("clearance_invalid".to_string());
        }
        clr.insert(principal.clone(), Value::String(level.clone()));
        write_json(&clearances_path(root), &Value::Object(clr))?;
        return Ok(json!({
            "ok": true,
            "type": "government_plane_classification",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "principal": principal,
            "clearance": level,
            "claim_evidence": [{
                "id": "V7-GOV-001.2",
                "claim": "classification_plane_persists_clearance_and_enforces_namespace_isolation",
                "evidence": {"principal": principal}
            }]
        }));
    }
    let principal_level = clr
        .get(&principal)
        .and_then(Value::as_str)
        .unwrap_or("unclassified")
        .to_string();
    let level = clean(
        parsed
            .flags
            .get("level")
            .map(String::as_str)
            .unwrap_or("unclassified"),
        32,
    )
    .to_ascii_lowercase();
    if level_rank(&level) < 0 {
        return Err("classification_level_invalid".to_string());
    }
    if op == "status" {
        return Ok(json!({
            "ok": true,
            "type": "government_plane_classification",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "principal": principal,
            "principal_clearance": principal_level,
            "clearance_path": clearances_path(root).to_string_lossy().to_string(),
            "claim_evidence": [{
                "id": "V7-GOV-001.2",
                "claim": "classification_plane_status_surfaces_principal_clearance_and_namespace_paths",
                "evidence": {"principal_clearance": principal_level}
            }]
        }));
    }
    if op == "transfer" {
        let from = clean(
            parsed
                .flags
                .get("from")
                .map(String::as_str)
                .unwrap_or("secret"),
            32,
        )
        .to_ascii_lowercase();
        let to = clean(
            parsed
                .flags
                .get("to")
                .map(String::as_str)
                .unwrap_or("unclassified"),
            32,
        )
        .to_ascii_lowercase();
        let via_cds = parse_bool(parsed.flags.get("via-cds"), false);
        let allowed = level_rank(&from) >= level_rank(&to) && via_cds;
        return Ok(json!({
            "ok": allowed,
            "type": "government_plane_classification",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "principal": principal,
            "from": from,
            "to": to,
            "via_cds": via_cds,
            "claim_evidence": [{
                "id": "V7-GOV-001.2",
                "claim": "classification_transfers_require_explicit_cross_domain_guard_path",
                "evidence": {"allowed": allowed}
            }]
        }));
    }
    let id = clean(
        parsed
            .flags
            .get("id")
            .map(String::as_str)
            .unwrap_or("object"),
        140,
    );
    let object_path = classification_root(root)
        .join(level.clone())
        .join(format!("{}.json", id));
    if level_rank(&principal_level) < level_rank(&level) {
        return Ok(json!({
            "ok": false,
            "type": "government_plane_classification",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "principal": principal,
            "principal_clearance": principal_level,
            "target_level": level,
            "error": "clearance_insufficient",
            "claim_evidence": [{
                "id": "V7-GOV-001.2",
                "claim": "classification_access_fails_closed_above_effective_clearance",
                "evidence": {"principal_clearance": principal_level}
            }]
        }));
    }
    if op == "write" {
        let payload = parse_json_or_empty(parsed.flags.get("payload-json"));
        write_json(
            &object_path,
            &json!({"principal": principal, "level": level, "payload": payload, "ts": now_iso()}),
        )?;
    } else if op != "read" {
        return Err("classification_op_invalid".to_string());
    }
    Ok(json!({
        "ok": true,
        "type": "government_plane_classification",
        "lane": LANE_ID,
        "ts": now_iso(),
        "op": op,
        "principal": principal,
        "principal_clearance": principal_level,
        "target_level": level,
        "object_path": object_path.to_string_lossy().to_string(),
        "object": read_json(&object_path).unwrap_or_else(|| json!({})),
        "claim_evidence": [{
            "id": "V7-GOV-001.2",
            "claim": "classification_plane_persists_isolated_level_scoped_objects",
            "evidence": {"op": op, "level": level}
        }]
    }))
}
