
fn execute_contract_with_options(
    root: &Path,
    id: &str,
    dispatch_enabled: bool,
    dispatch_strict: bool,
) -> Result<Value, String> {
    let normalized_id = id.trim().to_ascii_uppercase();
    if normalized_id.is_empty() {
        return Err("missing_id".to_string());
    }

    let cpath = contract_path(root, &normalized_id);
    if !cpath.exists() {
        return Err("contract_not_found".to_string());
    }

    let contract = read_json(&cpath)?;
    validate_contract_shape(&normalized_id, &contract)?;

    let contract_bytes =
        serde_json::to_vec(&contract).map_err(|e| format!("contract_encode_failed:{e}"))?;
    let mut hasher = Sha256::new();
    hasher.update(contract_bytes);
    let contract_digest = format!("sha256:{}", hex::encode(hasher.finalize()));
    let now_ms = now_epoch_ms();
    let dispatch_targets = if dispatch_enabled {
        runtime_lane_targets(&contract)
    } else {
        Vec::new()
    };
    let dispatch_bin = std::env::var("INFRING_SRS_DISPATCH_BIN")
        .ok()
        .filter(|row| !row.trim().is_empty())
        .or_else(|| {
            std::env::current_exe()
                .ok()
                .and_then(|path| path.to_str().map(|v| v.to_string()))
        })
        .unwrap_or_else(|| "infring-ops".to_string());
    let mut dispatch_results = Vec::<Value>::new();
    let mut dispatch_failed = 0usize;
    for target in &dispatch_targets {
        let row = run_dispatch_target(root, target, dispatch_strict, &dispatch_bin);
        if row.get("ok").and_then(Value::as_bool) != Some(true) {
            dispatch_failed += 1;
        }
        dispatch_results.push(row);
    }
    let dispatch_ok = dispatch_failed == 0;
    let receipt_ok = if dispatch_strict { dispatch_ok } else { true };

    let receipt = with_hash(json!({
        "ok": receipt_ok,
        "type": "srs_contract_runtime_receipt",
        "lane": "srs_contract_runtime",
        "id": normalized_id,
        "ts_epoch_ms": now_ms,
        "contract_path": cpath.to_string_lossy(),
        "contract_digest": contract_digest,
        "contract": contract,
        "dispatch": {
            "enabled": dispatch_enabled,
            "strict": dispatch_strict,
            "dispatch_bin": dispatch_bin,
            "target_count": dispatch_targets.len(),
            "failed": dispatch_failed,
            "results": dispatch_results
        },
        "claim_evidence": [
            {
                "id": normalized_id,
                "claim": "srs_actionable_item_has_contract_receipt_and_deliverables",
                "evidence": {
                    "lane": "core/layer2/ops:srs_contract_runtime",
                    "state_root": STATE_ROOT
                }
            },
            {
                "id": "srs_contract_runtime_dispatch",
                "claim": "runtime_lane_deliverables_dispatch_to_authoritative_plane_commands_with_receipt_aggregation",
                "evidence": {
                    "target_count": dispatch_targets.len(),
                    "dispatch_failed": dispatch_failed,
                    "dispatch_strict": dispatch_strict
                }
            }
        ]
    }));

    write_json(&latest_path(root, &normalized_id), &receipt)?;
    append_jsonl(&history_path(root), &receipt)?;
    Ok(receipt)
}

fn print_json_line(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn usage() {
    println!("Usage:");
    println!("  infring-ops srs-contract-runtime run --id=<V6-...>");
    println!("  infring-ops srs-contract-runtime run-many --ids=<ID1,ID2,...>");
    println!("  infring-ops srs-contract-runtime run-many --ids-file=<path>");
    println!("  infring-ops srs-contract-runtime status --id=<V6-...>");
}
