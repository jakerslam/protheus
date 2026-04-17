fn collect_structured_chain_entries(vault: &Value) -> Result<Vec<Value>, String> {
    let mut rows = prime_rows(vault);
    rows.extend(derived_rows(vault));
    let chain_head = vault
        .get("chain_head")
        .and_then(Value::as_str)
        .unwrap_or("genesis")
        .to_string();
    let mut by_hash: HashMap<String, Value> = HashMap::new();
    for row in rows {
        if !is_structured_directive_entry(&row) {
            continue;
        }
        let actual = row
            .get("entry_hash")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        let expected = recompute_entry_hash(&row);
        if actual.is_empty() || !actual.eq_ignore_ascii_case(&expected) {
            return Err("repair_entry_hash_invalid".to_string());
        }
        by_hash.insert(actual, row);
    }

    if by_hash.is_empty() {
        return Ok(Vec::new());
    }
    if chain_head == "genesis" {
        return Err("repair_missing_chain_head".to_string());
    }

    let mut cursor = chain_head;
    let mut visited = HashSet::new();
    let mut ordered = Vec::new();
    loop {
        if cursor == "genesis" {
            break;
        }
        if !visited.insert(cursor.clone()) {
            return Err("repair_chain_cycle_detected".to_string());
        }
        let Some(row) = by_hash.get(&cursor) else {
            return Err("repair_chain_head_missing_entry".to_string());
        };
        ordered.push(row.clone());
        cursor = row
            .get("prev_hash")
            .and_then(Value::as_str)
            .unwrap_or("genesis")
            .to_string();
    }

    if ordered.len() != by_hash.len() {
        return Err("repair_chain_length_mismatch".to_string());
    }
    ordered.reverse();
    Ok(ordered)
}

fn web_tooling_gate_coverage(entries: &[Value]) -> Value {
    let mut allow_search = false;
    let mut allow_fetch = false;
    let mut allow_runtime_channel = false;
    for row in entries {
        let blob = row.to_string().to_ascii_lowercase();
        if blob.contains("allow:web-search")
            || blob.contains("allow:web_search")
            || blob.contains("web-search")
        {
            allow_search = true;
        }
        if blob.contains("allow:web-fetch")
            || blob.contains("allow:web_fetch")
            || blob.contains("web-fetch")
        {
            allow_fetch = true;
        }
        if blob.contains("runtime-web-channel")
            || blob.contains("web-tooling-status")
            || blob.contains("network:web-tooling")
        {
            allow_runtime_channel = true;
        }
    }

    let coverage_state = if allow_search && allow_fetch && allow_runtime_channel {
        "ready"
    } else if allow_search || allow_fetch || allow_runtime_channel {
        "partial"
    } else {
        "missing"
    };
    json!({
        "allow_search": allow_search,
        "allow_fetch": allow_fetch,
        "allow_runtime_channel": allow_runtime_channel,
        "coverage_state": coverage_state,
        "entry_count": entries.len(),
    })
}

fn repair_vault_signatures(
    root: &Path,
    apply: bool,
    allow_unsigned: bool,
) -> Result<Value, String> {
    let key_present = signing_key_present();
    if !key_present && !allow_unsigned {
        return Err("missing_signing_key".to_string());
    }

    let mut vault = load_vault(root);
    let ordered = collect_structured_chain_entries(&vault)?;
    let preflight_web_tooling_coverage = web_tooling_gate_coverage(&ordered);
    let mode = if key_present { "keyed" } else { "unsigned" };

    if !apply {
        return Ok(json!({
            "apply": false,
            "mode": mode,
            "key_present": key_present,
            "eligible_entries": ordered.len(),
            "web_tooling_gate_coverage": preflight_web_tooling_coverage
        }));
    }

    let mut replacement_by_id: HashMap<String, Value> = HashMap::new();
    let mut prev_hash = "genesis".to_string();
    for row in ordered {
        let mut updated = row.clone();
        updated["prev_hash"] = Value::String(prev_hash.clone());
        updated["signature"] = Value::String(signature_for_entry(&updated));
        let entry_hash = recompute_entry_hash(&updated);
        updated["entry_hash"] = Value::String(entry_hash.clone());
        prev_hash = entry_hash;

        let id = updated
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| "repair_entry_id_missing".to_string())?
            .to_string();
        replacement_by_id.insert(id, updated);
    }

    let obj = vault_obj_mut(&mut vault);
    for bucket in ["prime", "derived"] {
        let rows = ensure_array(obj, bucket);
        for row in rows.iter_mut() {
            if !is_structured_directive_entry(row) {
                continue;
            }
            let Some(id) = row.get("id").and_then(Value::as_str) else {
                continue;
            };
            if let Some(replacement) = replacement_by_id.get(id) {
                *row = replacement.clone();
            }
        }
    }
    obj.insert("chain_head".to_string(), Value::String(prev_hash.clone()));
    write_vault(root, &vault)?;
    let repaired_rows = replacement_by_id.values().cloned().collect::<Vec<_>>();

    Ok(json!({
        "apply": true,
        "mode": mode,
        "key_present": key_present,
        "repaired_entries": replacement_by_id.len(),
        "new_chain_head": prev_hash,
        "web_tooling_gate_coverage": web_tooling_gate_coverage(&repaired_rows)
    }))
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    directive_kernel_run::run(root, argv)
}
