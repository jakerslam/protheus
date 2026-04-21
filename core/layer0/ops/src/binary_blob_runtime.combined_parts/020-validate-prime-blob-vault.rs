
fn validate_prime_blob_vault(vault: &Value) -> Value {
    let entries = vault
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let entry_count = entries.len() as u64;
    let chain_head = vault
        .get("chain_head")
        .and_then(Value::as_str)
        .unwrap_or("genesis")
        .to_string();
    let mut signature_valid = 0u64;
    let mut hash_valid = 0u64;
    let mut errors: Vec<String> = Vec::new();
    let mut by_hash: HashMap<String, Value> = HashMap::new();

    for (idx, entry) in entries.iter().enumerate() {
        if verify_blob_entry_signature(entry) {
            signature_valid += 1;
        } else {
            errors.push(format!("signature_invalid_at:{idx}"));
        }
        let actual = entry
            .get("entry_hash")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        let expected = recompute_blob_entry_hash(entry);
        if !actual.is_empty() && actual.eq_ignore_ascii_case(&expected) {
            hash_valid += 1;
            if by_hash.insert(actual.clone(), entry.clone()).is_some() {
                errors.push(format!("duplicate_entry_hash:{actual}"));
            }
        } else {
            errors.push(format!("entry_hash_mismatch_at:{idx}"));
        }
    }

    let mut chain_valid = true;
    let mut traversed_count = 0u64;
    if entry_count == 0 {
        if chain_head != "genesis" {
            chain_valid = false;
            errors.push("non_genesis_chain_head_for_empty_vault".to_string());
        }
    } else if chain_head == "genesis" {
        chain_valid = false;
        errors.push("missing_chain_head".to_string());
    } else {
        let mut cursor = chain_head.clone();
        let mut visited = HashSet::new();
        loop {
            if cursor == "genesis" {
                break;
            }
            if !visited.insert(cursor.clone()) {
                chain_valid = false;
                errors.push("chain_cycle_detected".to_string());
                break;
            }
            let Some(entry) = by_hash.get(&cursor) else {
                chain_valid = false;
                errors.push(format!("chain_head_missing_entry:{cursor}"));
                break;
            };
            traversed_count += 1;
            cursor = entry
                .get("prev_hash")
                .and_then(Value::as_str)
                .unwrap_or("genesis")
                .to_string();
        }
        if traversed_count != entry_count {
            chain_valid = false;
            errors.push(format!(
                "chain_length_mismatch:traversed={traversed_count}:entries={entry_count}"
            ));
        }
    }

    let ok = entry_count == signature_valid && entry_count == hash_valid && chain_valid;
    json!({
        "ok": ok,
        "entry_count": entry_count,
        "signature_valid_count": signature_valid,
        "hash_valid_count": hash_valid,
        "chain_valid": chain_valid,
        "chain_head": chain_head,
        "errors": errors
    })
}

fn append_prime_blob_vault_entry(root: &Path, snapshot: &Value) -> Result<Value, String> {
    let mut vault = load_prime_blob_vault(root);
    if !vault.is_object() {
        vault = default_prime_blob_vault();
    }
    let obj = vault
        .as_object_mut()
        .ok_or_else(|| "blob_vault_not_object".to_string())?;
    let prev_hash = obj
        .get("chain_head")
        .and_then(Value::as_str)
        .unwrap_or("genesis")
        .to_string();

    let mut entry = json!({
        "entry_id": format!("blobv_{}", &sha256_hex_str(&format!("{}:{}", now_iso(), snapshot.get("blob_id").and_then(Value::as_str).unwrap_or("unknown")))[..16]),
        "module": snapshot.get("module").cloned().unwrap_or(Value::Null),
        "blob_id": snapshot.get("blob_id").cloned().unwrap_or(Value::Null),
        "source_hash": snapshot.get("source_hash").cloned().unwrap_or(Value::Null),
        "blob_hash": snapshot.get("blob_hash").cloned().unwrap_or(Value::Null),
        "policy_hash": snapshot.get("policy_hash").cloned().unwrap_or(Value::Null),
        "mode": snapshot.get("mode").cloned().unwrap_or(Value::Null),
        "shadow_pointer": snapshot.get("shadow_pointer").cloned().unwrap_or(Value::Null),
        "rollback_pointer": snapshot.get("rollback_pointer").cloned().unwrap_or(Value::Null),
        "prev_hash": prev_hash,
        "ts": now_iso()
    });
    let signature = sign_blob_entry(&entry);
    entry["signature"] = Value::String(signature);
    let entry_hash = recompute_blob_entry_hash(&entry);
    entry["entry_hash"] = Value::String(entry_hash.clone());

    if !obj.get("entries").map(Value::is_array).unwrap_or(false) {
        obj.insert("entries".to_string(), Value::Array(Vec::new()));
    }
    obj.get_mut("entries")
        .and_then(Value::as_array_mut)
        .expect("entries_array")
        .push(entry.clone());
    obj.insert("chain_head".to_string(), Value::String(entry_hash));
    store_prime_blob_vault(root, &vault)?;
    Ok(entry)
}

fn repair_prime_blob_vault(
    root: &Path,
    apply: bool,
    allow_unsigned: bool,

) -> Result<Value, String> {
    let key_present = !blob_vault_signing_keys().is_empty();
    if !key_present && !allow_unsigned {
        return Err("missing_blob_vault_signing_key".to_string());
    }

    let mut vault = load_prime_blob_vault(root);
    let entries = vault
        .get("entries")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mode = if key_present { "keyed" } else { "unsigned" };

    if !apply {
        return Ok(json!({
            "apply": false,
            "mode": mode,
            "key_present": key_present,
            "eligible_entries": entries.len()
        }));
    }

    let mut repaired_entries = Vec::<Value>::with_capacity(entries.len());
    let mut prev_hash = "genesis".to_string();
    for (idx, row) in entries.iter().enumerate() {
        let mut updated = if row.is_object() {
            row.clone()
        } else {
            json!({
                "entry_id": format!("blobv_repair_{idx}"),
                "module": Value::Null,
                "blob_id": Value::Null,
                "source_hash": Value::Null,
                "blob_hash": Value::Null,
                "policy_hash": Value::Null,
                "mode": Value::Null,
                "shadow_pointer": Value::Null,
                "rollback_pointer": Value::Null,
                "ts": now_iso()
            })
        };
        if updated
            .get("entry_id")
            .and_then(Value::as_str)
            .map(|v| v.trim().is_empty())
            .unwrap_or(true)
        {
            updated["entry_id"] = Value::String(format!(
                "blobv_repair_{}",
                &sha256_hex_str(&format!("{}:{idx}", now_iso()))[..16]
            ));
        }
        if updated.get("ts").and_then(Value::as_str).is_none() {
            updated["ts"] = Value::String(now_iso());
        }
        updated["prev_hash"] = Value::String(prev_hash.clone());
        updated["signature"] = Value::String(sign_blob_entry(&updated));
        let entry_hash = recompute_blob_entry_hash(&updated);
        updated["entry_hash"] = Value::String(entry_hash.clone());
        prev_hash = entry_hash;
        repaired_entries.push(updated);
    }

    if !vault.is_object() {
        vault = default_prime_blob_vault();
    }
    let obj = vault
        .as_object_mut()
        .ok_or_else(|| "blob_vault_not_object".to_string())?;
    obj.insert(
        "entries".to_string(),
        Value::Array(repaired_entries.clone()),
    );
    obj.insert("chain_head".to_string(), Value::String(prev_hash.clone()));
    if obj.get("version").and_then(Value::as_str).is_none() {
        obj.insert("version".to_string(), Value::String("1.0".to_string()));
    }
    if obj.get("created_at").and_then(Value::as_str).is_none() {
        obj.insert("created_at".to_string(), Value::String(now_iso()));
    }
    store_prime_blob_vault(root, &vault)?;
    let integrity = validate_prime_blob_vault(&vault);
    Ok(json!({
        "apply": true,
        "mode": mode,
        "key_present": key_present,
        "repaired_entries": repaired_entries.len(),
        "new_chain_head": prev_hash,
        "integrity": integrity
    }))
}

fn find_prime_blob_entry(root: &Path, module: &str, blob_id: &str) -> Option<Value> {
    let vault = load_prime_blob_vault(root);
    let entries = vault.get("entries").and_then(Value::as_array)?;
    entries
        .iter()
        .rev()
        .find(|row| {
            row.get("module")
                .and_then(Value::as_str)
                .map(|v| v == module)
                .unwrap_or(false)
                && row
                    .get("blob_id")
                    .and_then(Value::as_str)
                    .map(|v| v == blob_id)
                    .unwrap_or(false)
        })
        .cloned()
}
