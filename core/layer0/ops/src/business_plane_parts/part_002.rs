fn parse_date_range(range: &str) -> (Option<String>, Option<String>) {
    let parts = range
        .split(':')
        .map(|s| s.trim().to_string())
        .collect::<Vec<_>>();
    if parts.len() != 2 {
        return (None, None);
    }
    let start = if parts[0].is_empty() {
        None
    } else {
        Some(parts[0].clone())
    };
    let end = if parts[1].is_empty() {
        None
    } else {
        Some(parts[1].clone())
    };
    (start, end)
}

fn archive_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let op = clean(
        parsed
            .flags
            .get("op")
            .map(String::as_str)
            .unwrap_or("status"),
        16,
    )
    .to_ascii_lowercase();
    if op == "record" {
        let row = parse_json_or_empty(parsed.flags.get("entry-json"));
        append_archive(root, &row)?;
        return Ok(json!({
            "ok": true,
            "type": "business_plane_archive",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "recorded": true,
            "claim_evidence": [{
                "id": "V7-BUSINESS-001.8",
                "claim": "business_receipt_archive_is_append_only_with_daily_merkle_anchor_and_audit_export_support",
                "evidence": {"recorded": true}
            }]
        }));
    }
    let rows = read_jsonl(&archive_path(root));
    let business_scope = clean(
        parsed
            .flags
            .get("business-context")
            .map(String::as_str)
            .unwrap_or("ALL"),
        80,
    );
    let date_range = clean(
        parsed
            .flags
            .get("date-range")
            .map(String::as_str)
            .unwrap_or(":"),
        64,
    );
    let (start, end) = parse_date_range(&date_range);
    let filtered = rows
        .iter()
        .filter(|row| {
            if business_scope != "ALL"
                && row.get("business_context").and_then(Value::as_str)
                    != Some(business_scope.as_str())
            {
                return false;
            }
            let ts = row.get("ts").and_then(Value::as_str).unwrap_or("");
            if let Some(s) = &start {
                if ts < s {
                    return false;
                }
            }
            if let Some(e) = &end {
                if ts > e {
                    return false;
                }
            }
            true
        })
        .cloned()
        .collect::<Vec<_>>();
    if op == "export" {
        let export_path = lane_root(root).join("audit_export.json");
        write_json(
            &export_path,
            &json!({
                "generated_at": now_iso(),
                "business_context": business_scope,
                "date_range": date_range,
                "rows": filtered
            }),
        )?;
        return Ok(json!({
            "ok": true,
            "type": "business_plane_archive",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "export_path": export_path.to_string_lossy().to_string(),
            "row_count": filtered.len(),
            "daily_roots": read_json(&archive_anchor_path(root)).unwrap_or_else(|| json!({})),
            "claim_evidence": [{
                "id": "V7-BUSINESS-001.8",
                "claim": "business_receipt_archive_is_append_only_with_daily_merkle_anchor_and_audit_export_support",
                "evidence": {"op": op, "row_count": filtered.len()}
            }]
        }));
    }
    if op != "query" && op != "status" {
        return Err("archive_op_invalid".to_string());
    }
    Ok(json!({
        "ok": true,
        "type": "business_plane_archive",
        "lane": LANE_ID,
        "ts": now_iso(),
        "op": op,
        "business_context": business_scope,
        "date_range": date_range,
        "row_count": filtered.len(),
        "rows": filtered,
        "daily_roots": read_json(&archive_anchor_path(root)).unwrap_or_else(|| json!({})),
        "claim_evidence": [{
            "id": "V7-BUSINESS-001.8",
            "claim": "business_receipt_archive_is_append_only_with_daily_merkle_anchor_and_audit_export_support",
            "evidence": {"op": op, "row_count": filtered.len()}
        }]
    }))
}

struct WalkCount;
impl WalkCount {
    fn count_json_files(path: &Path) -> usize {
        if !path.exists() {
            return 0;
        }
        let mut count = 0usize;
        let mut stack = vec![path.to_path_buf()];
        while let Some(cur) = stack.pop() {
            if let Ok(read_dir) = fs::read_dir(cur) {
                for entry in read_dir.flatten() {
                    let p = entry.path();
                    if p.is_dir() {
                        stack.push(p);
                    } else if p.extension().and_then(|v| v.to_str()) == Some("json") {
                        count += 1;
                    }
                }
            }
        }
        count
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let strict = parse_bool(parsed.flags.get("strict"), true);
    let bypass = conduit_bypass_requested(&parsed.flags);
    let conduit = build_conduit_enforcement(
        root,
        ENV_KEY,
        LANE_ID,
        strict,
        &command,
        "business_plane_conduit_enforcement",
        "client/protheusctl -> core/business-plane",
        bypass,
        vec![json!({
            "id": "V7-BUSINESS-001.5",
            "claim": "business_plane_operations_are_conduit_routed_fail_closed_and_business_scoped",
            "evidence": {"command": command, "bypass_requested": bypass}
        })],
    );
    if strict && !conduit.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        let payload = json!({
            "ok": false,
            "type": "business_plane",
            "lane": LANE_ID,
            "ts": now_iso(),
            "command": command,
            "error": "conduit_bypass_rejected"
        });
        return emit(root, &command, strict, payload, Some(&conduit));
    }
    let result = match command.as_str() {
        "taxonomy" => taxonomy_command(root, &parsed),
        "persona" => persona_command(root, &parsed),
        "continuity" => continuity_command(root, &parsed),
        "alerts" => alerts_command(root, &parsed),
        "switchboard" => switchboard_command(root, &parsed),
        "external-sync" | "external_sync" => external_sync_command(root, &parsed),
        "continuity-audit" | "continuity_audit" => continuity_audit_command(root, &parsed),
        "archive" | "audit" => archive_command(root, &parsed),
        "status" => Ok(json!({
            "ok": true,
            "type": "business_plane_status",
            "lane": LANE_ID,
            "ts": now_iso(),
            "state_root": lane_root(root).to_string_lossy().to_string(),
            "latest_path": latest_path(root, ENV_KEY, LANE_ID).to_string_lossy().to_string(),
            "history_path": history_path(root, ENV_KEY, LANE_ID).to_string_lossy().to_string(),
            "business_registry_path": business_registry_path(root).to_string_lossy().to_string(),
            "claim_evidence": [{
                "id": "V7-BUSINESS-001.1",
                "claim": "business_plane_status_surfaces_authoritative_memory_and_continuity_paths",
                "evidence": {"state_root": lane_root(root).to_string_lossy().to_string()}
            }]
        })),
        _ => Err("unknown_business_command".to_string()),
    };
    match result {
        Ok(payload) => emit(root, &command, strict, payload, Some(&conduit)),
        Err(error) => emit(
            root,
            &command,
            strict,
            json!({
                "ok": false,
                "type": "business_plane",
                "lane": LANE_ID,
                "ts": now_iso(),
                "command": command,
                "error": error
            }),
            Some(&conduit),
        ),
    }
}

