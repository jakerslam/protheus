fn web_tooling_policy_missing_codes_from_coverage(coverage: &serde_json::Value) -> Vec<String> {
    let mut missing = Vec::new();
    if !coverage
        .get("allow_search")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        missing.push("directive_allow_web_search_missing".to_string());
    }
    if !coverage
        .get("allow_fetch")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        missing.push("directive_allow_web_fetch_missing".to_string());
    }
    if !coverage
        .get("allow_runtime_channel")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
    {
        missing.push("directive_allow_runtime_web_channel_missing".to_string());
    }
    missing
}

pub fn web_tooling_policy_snapshot(root: &std::path::Path) -> serde_json::Value {
    let vault = load_vault(root);
    match collect_structured_chain_entries(&vault) {
        Ok(entries) => {
            let coverage = web_tooling_gate_coverage(&entries);
            let missing_codes = web_tooling_policy_missing_codes_from_coverage(&coverage);
            serde_json::json!({
                "ok": missing_codes.is_empty(),
                "type": "directive_web_tooling_policy_snapshot",
                "coverage": coverage,
                "missing_codes": missing_codes,
                "entry_count": entries.len(),
                "updated_at": crate::now_iso()
            })
        }
        Err(err) => serde_json::json!({
            "ok": false,
            "type": "directive_web_tooling_policy_snapshot",
            "error": crate::clean(err, 180),
            "missing_codes": ["directive_chain_invalid"],
            "updated_at": crate::now_iso()
        }),
    }
}

pub fn web_tooling_policy_missing_codes(root: &std::path::Path) -> Vec<String> {
    let snapshot = web_tooling_policy_snapshot(root);
    snapshot
        .get("missing_codes")
        .and_then(serde_json::Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_else(|| vec!["directive_web_tooling_policy_unknown".to_string()])
}

pub fn web_tooling_policy_ready(root: &std::path::Path) -> bool {
    web_tooling_policy_snapshot(root)
        .get("ok")
        .and_then(serde_json::Value::as_bool)
        .unwrap_or(false)
}
