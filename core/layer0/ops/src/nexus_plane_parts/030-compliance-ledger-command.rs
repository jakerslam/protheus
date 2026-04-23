
fn compliance_ledger_command(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    let op = parse_op(parsed);
    if op == "status" {
        let rows = read_jsonl(&lane_file(root, "compliance_ledger.jsonl"));
        return Ok(json!({
            "ok": true,
            "type": "nexus_plane_compliance_ledger",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "count": rows.len(),
            "rows": rows,
            "claim_evidence": [{
                "id": "V7-NEXUS-001.7",
                "claim": "compliance_ledger_status_surfaces_cross_domain_chain_history",
                "evidence": {"count": rows.len()}
            }]
        }));
    }
    if op == "append" {
        let entry = parse_json_or_empty(parsed.flags.get("entry-json"));
        let chain_id = clean(
            parsed
                .flags
                .get("chain-id")
                .map(String::as_str)
                .unwrap_or("chain"),
            120,
        );
        let row = json!({
            "ts": now_iso(),
            "chain_id": chain_id,
            "entry": entry,
            "lineage_hash": sha256_hex_str(&canonical_json_string(&entry))
        });
        append_jsonl(&lane_file(root, "compliance_ledger.jsonl"), &row)?;
        return Ok(json!({
            "ok": true,
            "type": "nexus_plane_compliance_ledger",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "row": row,
            "claim_evidence": [{
                "id": "V7-NEXUS-001.7",
                "claim": "unified_compliance_ledger_links_cross_domain_actions_with_single_chain_id_lineage",
                "evidence": {"chain_id": chain_id}
            }]
        }));
    }
    if op == "query" {
        let chain_id = clean(
            parsed
                .flags
                .get("chain-id")
                .map(String::as_str)
                .unwrap_or(""),
            120,
        );
        let rows = read_jsonl(&lane_file(root, "compliance_ledger.jsonl"))
            .into_iter()
            .filter(|row| {
                chain_id.is_empty()
                    || row.get("chain_id").and_then(Value::as_str) == Some(chain_id.as_str())
            })
            .collect::<Vec<_>>();
        return Ok(json!({
            "ok": true,
            "type": "nexus_plane_compliance_ledger",
            "lane": LANE_ID,
            "ts": now_iso(),
            "op": op,
            "chain_id": chain_id,
            "rows": rows,
            "claim_evidence": [{
                "id": "V7-NEXUS-001.7",
                "claim": "compliance_ledger_query_exports_chain_scoped_audit_material",
                "evidence": {"chain_id_filter": chain_id}
            }]
        }));
    }
    Err("compliance_ledger_op_invalid".to_string())
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
        "nexus_plane_conduit_enforcement",
        "client/infringctl -> core/nexus-plane",
        bypass,
        vec![json!({
            "id": "V7-NEXUS-001.2",
            "claim": "nexus_plane_commands_require_conduit_only_fail_closed_execution",
            "evidence": {"command": command, "bypass_requested": bypass}
        })],
    );
    if strict && !conduit.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        let payload = command_error_payload(&command, "conduit_bypass_rejected");
        return emit(root, &command, strict, payload, Some(&conduit));
    }
    let result = match command.as_str() {
        "package-domain" | "package_domain" => package_domain_command(root, &parsed),
        "bridge" => bridge_command(root, &parsed),
        "insurance" => insurance_command(root, &parsed),
        "human-boundary" | "human_boundary" => human_boundary_command(root, &parsed),
        "receipt-v2" | "receipt_v2" => receipt_v2_command(root, &parsed),
        "merkle-forest" | "merkle_forest" => merkle_forest_command(root, &parsed),
        "compliance-ledger" | "compliance_ledger" => compliance_ledger_command(root, &parsed),
        "status" => Ok(json!({
            "ok": true,
            "type": "nexus_plane_status",
            "lane": LANE_ID,
            "ts": now_iso(),
            "state_root": lane_root(root).to_string_lossy().to_string(),
            "latest_path": latest_path(root, ENV_KEY, LANE_ID).to_string_lossy().to_string(),
            "history_path": history_path(root, ENV_KEY, LANE_ID).to_string_lossy().to_string()
        })),
        _ => Err("unknown_nexus_command".to_string()),
    };
    match result {
        Ok(payload) => emit(root, &command, strict, payload, Some(&conduit)),
        Err(error) => emit(
            root,
            &command,
            strict,
            command_error_payload(&command, &error),
            Some(&conduit),
        ),
    }
}
