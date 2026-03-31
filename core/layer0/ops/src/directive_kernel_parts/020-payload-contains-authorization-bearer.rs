fn payload_contains_authorization_bearer(payload: &str, min_len: usize) -> bool {
    let lowered = payload.to_ascii_lowercase();
    let marker = "authorization: bearer ";
    let bytes = lowered.as_bytes();
    let marker_bytes = marker.as_bytes();
    let mut idx = 0usize;
    while idx + marker_bytes.len() <= bytes.len() {
        if &bytes[idx..idx + marker_bytes.len()] != marker_bytes {
            idx += 1;
            continue;
        }
        let mut count = 0usize;
        let mut cursor = idx + marker_bytes.len();
        while cursor < bytes.len() {
            let ch = bytes[cursor] as char;
            if ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.' {
                count += 1;
                cursor += 1;
                continue;
            }
            break;
        }
        if count >= min_len {
            return true;
        }
        idx = cursor;
    }
    false
}

fn approval_required_by_default(action_type: &str) -> bool {
    matches!(
        action_type,
        "publish_publicly"
            | "spend_money"
            | "change_credentials"
            | "delete_data"
            | "outbound_contact_new"
            | "deployment"
    )
}

fn irreversible_pattern(command_text: &str) -> Option<&'static str> {
    let patterns = [
        "rm -rf",
        "drop database",
        "drop table",
        "truncate",
        "delete",
        "destroy",
        "reset --hard",
        "git clean -fd",
    ];
    let lowered = command_text.to_ascii_lowercase();
    patterns
        .iter()
        .find(|pattern| lowered.contains(**pattern))
        .copied()
}

fn validate_action_envelope(root: &Path, envelope: &Value) -> Result<Value, String> {
    let directives = load_active_directives(root, false, false)?;
    let constraints = merge_active_constraints(&directives);
    let action_id = envelope
        .get("action_id")
        .and_then(Value::as_str)
        .map(|value| clean(value, 160))
        .unwrap_or_default();
    let tier = envelope.get("tier").and_then(Value::as_i64).unwrap_or(2);
    let action_type = envelope
        .get("type")
        .and_then(Value::as_str)
        .map(|value| clean(value, 120).to_ascii_lowercase())
        .unwrap_or_else(|| "other".to_string());
    let summary = envelope
        .get("summary")
        .and_then(Value::as_str)
        .map(|value| clean(value, 320).to_ascii_lowercase())
        .unwrap_or_default();
    let risk = envelope
        .get("risk")
        .and_then(Value::as_str)
        .map(|value| clean(value, 80).to_ascii_lowercase())
        .unwrap_or_else(|| "low".to_string());
    let payload_json = serde_json::to_string(envelope.get("payload").unwrap_or(&Value::Null))
        .unwrap_or_else(|_| "{}".to_string());

    let mut out = json!({
        "allowed": true,
        "requires_approval": false,
        "blocked_reason": Value::Null,
        "approval_reason": Value::Null,
        "effective_constraints": constraints.clone(),
        "action_id": action_id,
        "tier": tier
    });

    if constraints
        .get("hard_blocks")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false)
    {
        if payload_contains_secret_token(&payload_json, "moltbook_sk_", 20) {
            out["allowed"] = Value::Bool(false);
            out["blocked_reason"] = Value::String(
                "T0 INVARIANT VIOLATION: Secrets must always be redacted. Unredacted secret token detected in payload"
                    .to_string(),
            );
            return Ok(out);
        }
        if payload_contains_authorization_bearer(&payload_json, 20) {
            out["allowed"] = Value::Bool(false);
            out["blocked_reason"] = Value::String(
                "T0 INVARIANT VIOLATION: Secrets must always be redacted. Unredacted authorization header detected in payload"
                    .to_string(),
            );
            return Ok(out);
        }
    }

    let action_text = format!("{action_type} {summary}");
    if let Some(rows) = constraints
        .get("approval_required")
        .and_then(Value::as_array)
    {
        'approval_rows: for row in rows {
            let Some(obj) = row.as_object() else {
                continue;
            };
            if let Some(examples) = obj.get("examples").and_then(Value::as_array) {
                for example in examples {
                    let Some(example_text) = example.as_str() else {
                        continue;
                    };
                    let example_norm = clean(example_text, 160).to_ascii_lowercase();
                    if example_norm.is_empty() || !action_text.contains(&example_norm) {
                        continue;
                    }
                    out["requires_approval"] = Value::Bool(true);
                    out["approval_reason"] = Value::String(format!(
                        "{} (matched: {})",
                        clean(
                            obj.get("description")
                                .and_then(Value::as_str)
                                .unwrap_or(example_text),
                            240
                        ),
                        example_norm
                    ));
                    break 'approval_rows;
                }
            }
        }
    }

    if out
        .get("requires_approval")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        == false
    {
        if let Some(domains) = constraints
            .get("high_stakes_domains")
            .and_then(Value::as_array)
        {
            for domain in domains {
                let Some(domain_text) = domain.as_str() else {
                    continue;
                };
                if !action_text.contains(domain_text) {
                    continue;
                }
                out["requires_approval"] = Value::Bool(true);
                out["approval_reason"] = Value::String(format!(
                    "High-stakes domain '{}' requires approval",
                    domain_text
                ));
                break;
            }
        }
    }

    if let Some(command_text) = envelope
        .get("metadata")
        .and_then(|value| value.get("command_text"))
        .and_then(Value::as_str)
    {
        if irreversible_pattern(command_text).is_some()
            && !out
                .get("requires_approval")
                .and_then(Value::as_bool)
                .unwrap_or(false)
        {
            out["requires_approval"] = Value::Bool(true);
            out["approval_reason"] = Value::String(format!(
                "Irreversible action detected: {}",
                irreversible_pattern(command_text).unwrap_or("unknown")
            ));
        }
    }

    if approval_required_by_default(&action_type)
        && !out
            .get("requires_approval")
            .and_then(Value::as_bool)
            .unwrap_or(false)
    {
        out["requires_approval"] = Value::Bool(true);
        out["approval_reason"] = Value::String(format!(
            "Action type '{}' requires approval per T0 invariants",
            action_type
        ));
    }

    if risk == "high" && tier < 2 {
        out["requires_approval"] = Value::Bool(true);
        out["approval_reason"] =
            Value::String("High-risk action at Tier < 2 requires approval".to_string());
    }

    Ok(out)
}

fn check_tier_conflict(lower_tier_action: &Value, higher_tier_directive: &Value) -> Value {
    let lower_tier = lower_tier_action
        .get("tier")
        .and_then(Value::as_i64)
        .unwrap_or(2);
    let higher_tier = higher_tier_directive
        .get("tier")
        .and_then(Value::as_i64)
        .unwrap_or(0);
    if lower_tier > higher_tier {
        return json!({
            "is_conflict": true,
            "reason": format!(
                "Tier {} action attempted to override Tier {} directive",
                lower_tier,
                higher_tier
            ),
            "resolution": "Lower tier wins"
        });
    }
    json!({"is_conflict": false})
}

fn legacy_source_paths(root: &Path) -> Vec<PathBuf> {
    vec![
        root.join("docs")
            .join("workspace")
            .join("AGENT-CONSTITUTION.md"),
        root.join("docs")
            .join("client")
            .join("PROTHEUS_PRIME_SEED.md"),
        root.join("docs")
            .join("client")
            .join("internal")
            .join("persona")
            .join("AGENT-CONSTITUTION.md"),
    ]
}

fn default_vault() -> Value {
    json!({
        "version": "1.0",
        "prime": [],
        "derived": [],
        "chain_head": "genesis",
        "created_at": now_iso(),
        "migrations": []
    })
}

fn load_vault(root: &Path) -> Value {
    read_json(&vault_path(root)).unwrap_or_else(default_vault)
}

fn write_vault(root: &Path, vault: &Value) -> Result<(), String> {
    write_json(&vault_path(root), vault)
}

fn vault_obj_mut(vault: &mut Value) -> &mut Map<String, Value> {
    if !vault.is_object() {
        *vault = default_vault();
    }
    vault.as_object_mut().expect("vault_object")
}

fn ensure_array<'a>(obj: &'a mut Map<String, Value>, key: &str) -> &'a mut Vec<Value> {
    if !obj.get(key).map(Value::is_array).unwrap_or(false) {
        obj.insert(key.to_string(), Value::Array(Vec::new()));
    }
    obj.get_mut(key)
        .and_then(Value::as_array_mut)
        .expect("array")
}

fn normalize_rule(raw: &str) -> (String, String) {
    let cleaned = clean(raw, 512).to_ascii_lowercase();
    if let Some(v) = cleaned.strip_prefix("deny:") {
        return ("deny".to_string(), clean(v, 320));
    }
    if let Some(v) = cleaned.strip_prefix("allow:") {
        return ("allow".to_string(), clean(v, 320));
    }
    if cleaned.contains("deny") {
        ("deny".to_string(), cleaned)
    } else {
        ("allow".to_string(), cleaned)
    }
}

fn matches_pattern(action: &str, pattern: &str) -> bool {
    if pattern.is_empty() || pattern == "*" || pattern == "all" {
        return true;
    }
    if pattern.contains('*') {
        let parts = pattern
            .split('*')
            .map(str::trim)
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>();
        if parts.is_empty() {
            return true;
        }
        return parts.iter().all(|part| action.contains(part));
    }
    action.contains(pattern)
}

fn signature_for_entry(entry: &Value) -> String {
    let payload = canonical_signature_payload(entry);
    let key = std::env::var(SIGNING_ENV).unwrap_or_default();
    if key.trim().is_empty() {
        // still deterministic, but marked as unsigned in policy metadata.
        return format!(
            "unsigned:{}",
            sha256_hex_str(&serde_json::to_string(&payload).unwrap_or_default())
        );
    }
    format!("sig:{}", keyed_digest_hex(&key, &payload))
}

fn canonical_signature_payload(entry: &Value) -> Value {
    json!({
        "id": entry.get("id").cloned().unwrap_or(Value::Null),
        "directive": entry.get("directive").cloned().unwrap_or(Value::Null),
        "rule_kind": entry.get("rule_kind").cloned().unwrap_or(Value::Null),
        "rule_pattern": entry.get("rule_pattern").cloned().unwrap_or(Value::Null),
        "signer": entry.get("signer").cloned().unwrap_or(Value::Null),
        "source": entry.get("source").cloned().unwrap_or(Value::Null),
        "parent_id": entry.get("parent_id").cloned().unwrap_or(Value::Null),
        "supersedes": entry.get("supersedes").cloned().unwrap_or(Value::Null),
        "ts": entry.get("ts").cloned().unwrap_or(Value::Null),
        "prev_hash": entry.get("prev_hash").cloned().unwrap_or(Value::Null)
    })
}

fn verify_entry_signature(entry: &Value) -> bool {
    let signature = entry
        .get("signature")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .to_string();
    if signature.is_empty() {
        return false;
    }

    let payload = canonical_signature_payload(entry);
    if let Some(raw) = signature.strip_prefix("unsigned:") {
        return raw.eq_ignore_ascii_case(&sha256_hex_str(
            &serde_json::to_string(&payload).unwrap_or_default(),
        ));
    }
    if let Some(raw) = signature.strip_prefix("sig:") {
        let key = std::env::var(SIGNING_ENV).unwrap_or_default();
        if key.trim().is_empty() {
            return false;
        }
        return raw.eq_ignore_ascii_case(&keyed_digest_hex(&key, &payload));
    }
    false
}

fn is_structured_directive_entry(entry: &Value) -> bool {
    let Some(obj) = entry.as_object() else {
        return false;
    };
    let required = [
        "id",
        "directive",
        "rule_kind",
        "rule_pattern",
        "signer",
        "source",
        "prev_hash",
        "signature",
        "entry_hash",
        "ts",
    ];
    required.iter().all(|key| {
        obj.get(*key)
            .and_then(Value::as_str)
            .map(|v| !v.trim().is_empty())
            .unwrap_or(false)
    })
}

fn signature_counts(vault: &Value) -> (u64, u64) {
    let rows = collect_rules(vault);
    let total = rows.len() as u64;
    let valid = rows
        .iter()
        .filter(|row| verify_entry_signature(row))
        .count() as u64;
    (total, valid)
}

fn signing_key_present() -> bool {
    std::env::var(SIGNING_ENV)
        .ok()
        .map(|v| !v.trim().is_empty())
        .unwrap_or(false)
}
