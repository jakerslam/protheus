
fn run_topology(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        TOPOLOGY_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "agency_division_topology_contract",
            "required_fields": ["divisions", "handoffs"],
            "default_divisions": ["frontend", "security", "research"]
        }),
    );

    let mut errors = Vec::<String>::new();
    validate_contract(
        &contract,
        "agency_division_topology_contract",
        "agency_topology_contract_version_must_be_v1",
        "agency_topology_contract_kind_invalid",
        &mut errors,
    );

    let manifest = parsed
        .flags
        .get("manifest-json")
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .unwrap_or_else(|| {
            let divisions = contract
                .get("default_divisions")
                .and_then(Value::as_array)
                .cloned()
                .unwrap_or_else(|| vec![json!("frontend"), json!("security")]);
            json!({
                "divisions": divisions,
                "handoffs": [
                    {"from": "frontend", "to": "security"},
                    {"from": "security", "to": "research"}
                ]
            })
        });

    let divisions = manifest
        .get("divisions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let handoffs = manifest
        .get("handoffs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    if strict && divisions.is_empty() {
        errors.push("agency_topology_divisions_required".to_string());
    }
    if strict && handoffs.is_empty() {
        errors.push("agency_topology_handoffs_required".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "agency_plane_topology",
            "errors": errors
        });
    }

    let handoff_receipts = handoffs
        .iter()
        .enumerate()
        .map(|(idx, row)| {
            let from = clean(
                row.get("from").and_then(Value::as_str).unwrap_or_default(),
                80,
            );
            let to = clean(
                row.get("to").and_then(Value::as_str).unwrap_or_default(),
                80,
            );
            json!({
                "index": idx + 1,
                "from": from,
                "to": to,
                "handoff_hash": sha256_hex_str(&format!("{}:{}:{}", idx + 1, from, to))
            })
        })
        .collect::<Vec<_>>();

    let topology = json!({
        "version": "v1",
        "divisions": divisions,
        "handoffs": handoffs,
        "handoff_receipts": handoff_receipts,
        "generated_at": crate::now_iso()
    });

    let artifact_path = state_root(root).join("topology").join("latest.json");
    let _ = write_json(&artifact_path, &topology);

    json!({
        "ok": true,
        "strict": strict,
        "type": "agency_plane_topology",
        "lane": "core/layer0/ops",
        "topology": topology,
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&topology.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-AGENCY-001.2",
                "claim": "division_based_topology_emits_orchestration_manifest_with_deterministic_handoff_receipts",
                "evidence": {
                    "divisions": manifest
                        .get("divisions")
                        .and_then(Value::as_array)
                        .map(|rows| rows.len())
                        .unwrap_or(0),
                    "handoffs": handoff_receipts.len()
                }
            }
        ]
    })
    .with_receipt_hash()
}
