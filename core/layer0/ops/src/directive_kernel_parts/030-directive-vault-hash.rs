fn append_directive_entry(
    root: &Path,
    bucket: &str,
    directive_text: &str,
    signer: &str,
    parent_id: Option<&str>,
    supersedes: Option<&str>,
    source: &str,
) -> Result<Value, String> {
    let mut vault = load_vault(root);
    let obj = vault_obj_mut(&mut vault);
    let chain_head = obj
        .get("chain_head")
        .and_then(Value::as_str)
        .unwrap_or("genesis")
        .to_string();
    let (rule_kind, rule_pattern) = normalize_rule(directive_text);

    let mut payload = json!({
        "id": format!("dir_{}", &sha256_hex_str(&format!("{}:{}:{}", now_iso(), directive_text, signer))[..16]),
        "directive": clean(directive_text, 512),
        "rule_kind": rule_kind,
        "rule_pattern": rule_pattern,
        "signer": clean(signer, 128),
        "source": clean(source, 128),
        "parent_id": parent_id.unwrap_or(""),
        "supersedes": supersedes.unwrap_or(""),
        "accepted": true,
        "ts": now_iso(),
        "prev_hash": chain_head
    });
    let signature = signature_for_entry(&payload);
    payload["signature"] = Value::String(signature);
    let entry_hash = sha256_hex_str(&serde_json::to_string(&payload).unwrap_or_default());
    payload["entry_hash"] = Value::String(entry_hash.clone());

    let list = ensure_array(obj, bucket);
    list.push(payload.clone());
    obj.insert("chain_head".to_string(), Value::String(entry_hash));

    write_vault(root, &vault)?;
    Ok(payload)
}

pub(crate) fn append_compaction_directive_entry(
    root: &Path,
    directive_text: &str,
    signer: &str,
    parent_id: Option<&str>,
    source: &str,
) -> Result<Value, String> {
    append_directive_entry(
        root,
        "derived",
        directive_text,
        signer,
        parent_id,
        None,
        source,
    )
}

fn prime_rows(vault: &Value) -> Vec<Value> {
    vault
        .get("prime")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn derived_rows(vault: &Value) -> Vec<Value> {
    vault
        .get("derived")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn is_entry_active(entry: &Value) -> bool {
    entry
        .get("accepted")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn collect_rules(vault: &Value) -> Vec<Value> {
    let mut out = Vec::new();
    for row in prime_rows(vault) {
        if is_entry_active(&row) && is_structured_directive_entry(&row) {
            out.push(row);
        }
    }
    for row in derived_rows(vault) {
        if is_entry_active(&row) && is_structured_directive_entry(&row) {
            out.push(row);
        }
    }
    out
}

pub fn directive_vault_hash(root: &Path) -> String {
    let vault = load_vault(root);
    sha256_hex_str(&serde_json::to_string(&vault).unwrap_or_default())
}

fn canonical_entry_for_hash(entry: &Value) -> Value {
    let mut canonical = entry.clone();
    if let Some(obj) = canonical.as_object_mut() {
        obj.remove("entry_hash");
    }
    canonical
}

fn recompute_entry_hash(entry: &Value) -> String {
    sha256_hex_str(&serde_json::to_string(&canonical_entry_for_hash(entry)).unwrap_or_default())
}

pub fn directive_vault_integrity(root: &Path) -> Value {
    let vault = load_vault(root);
    let mut raw_rows = prime_rows(&vault);
    raw_rows.extend(derived_rows(&vault));
    let raw_entry_count = raw_rows.len() as u64;
    let mut rows = Vec::new();
    let mut ignored_legacy_entry_count = 0u64;
    for row in raw_rows {
        if is_structured_directive_entry(&row) {
            rows.push(row);
        } else {
            ignored_legacy_entry_count += 1;
        }
    }
    let entry_count = rows.len() as u64;
    let chain_head = vault
        .get("chain_head")
        .and_then(Value::as_str)
        .unwrap_or("genesis")
        .to_string();

    let mut signature_valid_count = 0u64;
    let mut hash_valid_count = 0u64;
    let mut errors: Vec<String> = Vec::new();
    let mut by_hash: HashMap<String, Value> = HashMap::new();
    for (idx, row) in rows.iter().enumerate() {
        if verify_entry_signature(row) {
            signature_valid_count += 1;
        } else {
            errors.push(format!("signature_invalid_at:{idx}"));
        }
        let actual = row
            .get("entry_hash")
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        let expected = recompute_entry_hash(row);
        if !actual.is_empty() && actual.eq_ignore_ascii_case(&expected) {
            hash_valid_count += 1;
            if by_hash.insert(actual.clone(), row.clone()).is_some() {
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
            let Some(row) = by_hash.get(&cursor) else {
                chain_valid = false;
                errors.push(format!("chain_head_missing_entry:{cursor}"));
                break;
            };
            traversed_count += 1;
            cursor = row
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

    json!({
        "ok": entry_count == signature_valid_count && entry_count == hash_valid_count && chain_valid,
        "raw_entry_count": raw_entry_count,
        "entry_count": entry_count,
        "ignored_legacy_entry_count": ignored_legacy_entry_count,
        "signature_valid_count": signature_valid_count,
        "hash_valid_count": hash_valid_count,
        "chain_valid": chain_valid,
        "chain_head": chain_head,
        "errors": errors
    })
}

pub fn evaluate_action(root: &Path, action: &str) -> Value {
    let vault = load_vault(root);
    let integrity = directive_vault_integrity(root);
    if !integrity
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return json!({
            "allowed": false,
            "action": clean(action, 320).to_ascii_lowercase(),
            "deny_hits": [{"id":"integrity", "rule_kind":"deny", "rule_pattern":"vault_integrity"}],
            "allow_hits": [],
            "invalid_signature_hits": [],
            "superseded_ids": [],
            "integrity": integrity,
            "policy_hash": directive_vault_hash(root)
        });
    }
    let action_norm = clean(action, 320).to_ascii_lowercase();
    let rules = collect_rules(&vault);
    let mut superseded_ids = HashSet::new();
    for row in &rules {
        if !verify_entry_signature(row) {
            continue;
        }
        let supersedes = row
            .get("supersedes")
            .and_then(Value::as_str)
            .map(|v| clean(v, 128))
            .unwrap_or_default();
        if !supersedes.is_empty() {
            superseded_ids.insert(supersedes);
        }
    }

    let mut deny_hits = Vec::new();
    let mut allow_hits = Vec::new();
    let mut invalid_signature_hits = Vec::new();
    for row in rules {
        let row_id = row
            .get("id")
            .and_then(Value::as_str)
            .map(|v| clean(v, 128))
            .unwrap_or_default();
        if !row_id.is_empty() && superseded_ids.contains(&row_id) {
            continue;
        }
        if !verify_entry_signature(&row) {
            invalid_signature_hits.push(json!({
                "id": row.get("id").cloned().unwrap_or(Value::Null),
                "signer": row.get("signer").cloned().unwrap_or(Value::Null),
                "reason": "invalid_signature"
            }));
            continue;
        }
        let kind = row
            .get("rule_kind")
            .and_then(Value::as_str)
            .unwrap_or("allow")
            .to_ascii_lowercase();
        let pattern = row
            .get("rule_pattern")
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_ascii_lowercase();
        if !matches_pattern(&action_norm, &pattern) {
            continue;
        }
        let hit = json!({
            "id": row.get("id").cloned().unwrap_or(Value::Null),
            "rule_kind": kind,
            "rule_pattern": pattern,
            "signer": row.get("signer").cloned().unwrap_or(Value::Null)
        });
        if kind == "deny" {
            deny_hits.push(hit);
        } else {
            allow_hits.push(hit);
        }
    }

    let allowed = deny_hits.is_empty() && !allow_hits.is_empty();
    json!({
        "allowed": allowed,
        "action": action_norm,
        "deny_hits": deny_hits,
        "allow_hits": allow_hits,
        "invalid_signature_hits": invalid_signature_hits,
        "superseded_ids": superseded_ids.into_iter().collect::<Vec<_>>(),
        "integrity": integrity,
        "policy_hash": directive_vault_hash(root)
    })
}

pub fn action_allowed(root: &Path, action: &str) -> bool {
    evaluate_action(root, action)
        .get("allowed")
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

fn resolve_parent(vault: &Value, parent_hint: &str) -> Option<Value> {
    let norm = clean(parent_hint, 512);
    if norm.is_empty() {
        return None;
    }
    let mut rows = prime_rows(vault);
    rows.extend(derived_rows(vault));
    rows.into_iter().find(|row| {
        row.get("id")
            .and_then(Value::as_str)
            .map(|id| id == norm)
            .unwrap_or(false)
            || row
                .get("directive")
                .and_then(Value::as_str)
                .map(|text| text == norm)
                .unwrap_or(false)
    })
}

fn has_inheritance_conflict(parent: &Value, child_rule_kind: &str, child_pattern: &str) -> bool {
    let parent_kind = parent
        .get("rule_kind")
        .and_then(Value::as_str)
        .unwrap_or("allow")
        .to_ascii_lowercase();
    let parent_pattern = parent
        .get("rule_pattern")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_ascii_lowercase();

    if parent_kind != "deny" || child_rule_kind != "allow" {
        return false;
    }
    child_pattern == parent_pattern
        || matches_pattern(child_pattern, &parent_pattern)
        || matches_pattern(&parent_pattern, child_pattern)
}

fn migrate_legacy_markdown(root: &Path, apply: bool) -> Result<Value, String> {
    let mut harvested = Vec::new();
    for path in legacy_source_paths(root) {
        if !path.exists() {
            continue;
        }
        let raw = fs::read_to_string(&path)
            .map_err(|err| format!("legacy_directive_read_failed:{}:{err}", path.display()))?;
        for line in raw.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if trimmed.starts_with('#') {
                continue;
            }
            if trimmed.starts_with("-") || trimmed.starts_with('*') {
                let cleaned = trimmed
                    .trim_start_matches('-')
                    .trim_start_matches('*')
                    .trim();
                if !cleaned.is_empty() {
                    harvested.push(clean(cleaned, 512));
                }
            }
        }
    }

    harvested.sort();
    harvested.dedup();

    let mut imported = Vec::new();
    if apply {
        for directive in &harvested {
            let entry = append_directive_entry(
                root,
                "prime",
                directive,
                "migration",
                None,
                None,
                "legacy_markdown",
            )?;
            imported.push(entry);
        }

        let mut vault = load_vault(root);
        let obj = vault_obj_mut(&mut vault);
        let migrations = ensure_array(obj, "migrations");
        migrations.push(json!({
            "ts": now_iso(),
            "type": "legacy_markdown_import",
            "count": harvested.len()
        }));
        write_vault(root, &vault)?;
    }

    Ok(json!({
        "harvested_count": harvested.len(),
        "imported_count": imported.len(),
        "legacy_paths": legacy_source_paths(root)
            .iter()
            .map(|p| p.display().to_string())
            .collect::<Vec<_>>()
    }))
}
