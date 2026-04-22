
fn load_and_verify(root: &Path, module: &str) -> Result<Value, String> {
    let vault_integrity = validate_prime_blob_vault(&load_prime_blob_vault(root));
    if !vault_integrity
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Err("prime_blob_vault_chain_invalid".to_string());
    }

    let active = load_active_map(root);
    let Some(entry) = active.get(module).cloned() else {
        return Err("module_not_settled".to_string());
    };

    let snapshot_path = entry
        .get("snapshot_path")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| "snapshot_path_missing".to_string())?;
    if !snapshot_path.exists() {
        return Err(format!("snapshot_missing:{}", snapshot_path.display()));
    }

    let snapshot = read_json(&snapshot_path)
        .ok_or_else(|| format!("snapshot_read_failed:{}", snapshot_path.display()))?;
    let source_path = snapshot
        .get("source_path")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .ok_or_else(|| "snapshot_source_path_missing".to_string())?;
    let expected_source_hash = snapshot
        .get("source_hash")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let source_hash = sha256_file(&source_path)?;
    let expected_policy_hash = snapshot
        .get("policy_hash")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let blob_path = snapshot
        .get("blob_path")
        .and_then(Value::as_str)
        .map(PathBuf::from)
        .or_else(|| {
            entry
                .get("blob_path")
                .and_then(Value::as_str)
                .map(PathBuf::from)
        })
        .ok_or_else(|| "snapshot_blob_path_missing".to_string())?;
    let expected_blob_hash = snapshot
        .get("blob_hash")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let expected_blob_id = snapshot
        .get("blob_id")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let current_policy_hash = directive_kernel::directive_vault_hash(root);

    if source_hash != expected_source_hash {
        return Err("source_hash_mismatch".to_string());
    }
    if !blob_path.exists() {
        return Err(format!("blob_missing:{}", blob_path.display()));
    }
    let blob_hash = sha256_file_mmap(&blob_path)?;
    if blob_hash != expected_blob_hash {
        return Err("blob_hash_mismatch".to_string());
    }
    if current_policy_hash != expected_policy_hash {
        return Err("policy_hash_mismatch".to_string());
    }

    let Some(vault_entry) = find_prime_blob_entry(root, module, &expected_blob_id) else {
        return Err("prime_blob_vault_entry_missing".to_string());
    };
    if !verify_blob_entry_signature(&vault_entry) {
        return Err("prime_blob_vault_signature_invalid".to_string());
    }
    if vault_entry
        .get("policy_hash")
        .and_then(Value::as_str)
        .unwrap_or("")
        != expected_policy_hash
    {
        return Err("prime_blob_vault_policy_mismatch".to_string());
    }
    if vault_entry
        .get("blob_hash")
        .and_then(Value::as_str)
        .unwrap_or("")
        != expected_blob_hash
    {
        return Err("prime_blob_vault_blob_hash_mismatch".to_string());
    }

    Ok(json!({
        "module": module,
        "snapshot_path": snapshot_path.display().to_string(),
        "source_path": source_path.display().to_string(),
        "blob_path": blob_path.display().to_string(),
        "source_hash": source_hash,
        "blob_hash": blob_hash,
        "policy_hash": current_policy_hash,
        "prime_blob_vault_entry_id": vault_entry.get("entry_id").cloned().unwrap_or(Value::Null),
        "prime_blob_vault_signature_verified": true,
        "prime_blob_vault_integrity": vault_integrity,
        "blob_first_bytes_hex": hex::encode(read_first_bytes(&blob_path, 16)?),
        "verified": true
    }))
}

fn emit(root: &Path, payload: Value) -> i32 {
    let mut normalized = payload;
    if normalized.get("lane").is_none() {
        normalized["lane"] = Value::String("core/layer0/ops".to_string());
    }
    if normalized.get("strict").is_none() {
        normalized["strict"] = Value::Bool(true);
    }
    if normalized.get("schema").is_none() {
        normalized["schema"] = Value::String("infring_layer1_security".to_string());
    }
    match write_receipt(root, STATE_ENV, STATE_SCOPE, normalized) {
        Ok(out) => {
            print_json(&out);
            if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                0
            } else {
                2
            }
        }
        Err(err) => {
            let mut out = json!({
                "ok": false,
                "type": "binary_blob_runtime_error",
                "lane": "core/layer0/ops",
                "error": clean(err, 240),
                "exit_code": 2
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            print_json(&out);
            2
        }
    }
}

fn verify_debug_token(root: &Path) -> Value {
    let (payload, code) = crate::infring_layer1_security_bridge::run_soul_token_guard(
        root,
        &["verify".to_string(), "--strict=1".to_string()],
    );
    json!({"ok": code == 0 && payload.get("ok").and_then(Value::as_bool).unwrap_or(false), "payload": payload, "code": code})
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    binary_blob_runtime_run::run(root, argv)
}

#[cfg(test)]
#[path = "../binary_blob_runtime_tests.rs"]
mod tests;
