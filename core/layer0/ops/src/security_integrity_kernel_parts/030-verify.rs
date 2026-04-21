
fn verify(
    runtime_root: &Path,
    policy_path: &Path,
    payload: Option<&Map<String, Value>>,
) -> Result<Value, String> {
    let policy = resolve_policy(runtime_root, policy_path, payload);
    let expected_paths = policy.hashes.keys().cloned().collect::<Vec<_>>();
    let present_paths = collect_present_files(runtime_root, &policy);
    let hash_pattern = Regex::new(r"^[a-f0-9]{64}$")
        .map_err(|err| format!("security_integrity_kernel_regex_failed:{err}"))?;
    let mut violations = Vec::new();

    if expected_paths.is_empty() {
        violations.push(json!({
            "type": "policy_unsealed",
            "file": Value::Null,
            "detail": "hashes_empty"
        }));
    }

    for rel in &expected_paths {
        let abs = runtime_root.join(rel);
        if !abs.exists() {
            violations.push(json!({
                "type": "missing_sealed_file",
                "file": rel
            }));
            continue;
        }
        let expected = policy.hashes.get(rel).cloned().unwrap_or_default();
        if !hash_pattern.is_match(&expected) {
            violations.push(json!({
                "type": "invalid_hash_entry",
                "file": rel,
                "expected": expected
            }));
            continue;
        }
        let actual = sha256_file(&abs)?;
        if actual != expected {
            violations.push(json!({
                "type": "hash_mismatch",
                "file": rel,
                "expected": expected,
                "actual": actual
            }));
        }
    }

    for rel in &present_paths {
        if !policy.hashes.contains_key(rel) {
            violations.push(json!({
                "type": "unsealed_file",
                "file": rel
            }));
        }
    }

    for rel in &expected_paths {
        let missing = !present_paths.iter().any(|existing| existing == rel);
        let already_missing = violations.iter().any(|violation| {
            violation.get("type").and_then(Value::as_str) == Some("missing_sealed_file")
                && violation.get("file").and_then(Value::as_str) == Some(rel.as_str())
        });
        if missing && !already_missing {
            violations.push(json!({
                "type": "sealed_file_outside_scope",
                "file": rel
            }));
        }
    }

    Ok(json!({
        "ok": violations.is_empty(),
        "ts": now_iso(),
        "policy_path": policy_path.to_string_lossy(),
        "policy_version": policy.version,
        "checked_present_files": present_paths.len(),
        "expected_files": expected_paths.len(),
        "violations": violations,
        "violation_counts": summarize_violations(&violations)
    }))
}

fn seal(
    runtime_root: &Path,
    policy_path: &Path,
    payload: &Map<String, Value>,
) -> Result<Value, String> {
    let mut policy = load_policy(runtime_root, policy_path);
    let present = collect_present_files(runtime_root, &policy);
    let mut hashes = BTreeMap::new();
    for rel in &present {
        hashes.insert(rel.clone(), sha256_file(&runtime_root.join(rel))?);
    }
    policy.hashes = hashes;
    policy.sealed_at = Some(now_iso());
    policy.sealed_by = Some(
        clean_text(payload.get("sealed_by"), 120)
            .if_empty_then(&std::env::var("USER").unwrap_or_else(|_| "unknown".to_string())),
    );
    policy.last_approval_note =
        Some(clean_text(payload.get("approval_note"), 240)).filter(|v| !v.is_empty());
    write_json_atomic(policy_path, &policy_to_value(&policy))?;
    Ok(json!({
        "ok": true,
        "policy_path": policy_path.to_string_lossy(),
        "policy_version": policy.version,
        "sealed_files": present.len(),
        "sealed_at": policy.sealed_at,
        "sealed_by": policy.sealed_by
    }))
}

fn append_event(log_path: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let entry = payload
        .get("entry")
        .cloned()
        .unwrap_or_else(|| Value::Object(payload.clone()));
    let row = if let Value::Object(mut map) = entry {
        if !map.contains_key("ts") {
            map.insert("ts".to_string(), Value::String(now_iso()));
        }
        Value::Object(map)
    } else {
        json!({
            "ts": now_iso(),
            "entry": entry
        })
    };
    append_jsonl(log_path, &row)?;
    Ok(json!({
        "ok": true,
        "log_path": log_path.to_string_lossy(),
        "entry": row
    }))
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "load-policy".to_string());
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload = match payload_json(&argv[1..]) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("security_integrity_kernel_error", &err));
            return 1;
        }
    };
    let payload = payload_obj(&payload);
    let runtime_root = runtime_root(root, payload);
    let policy_path = resolve_path(
        &runtime_root,
        &clean_text(payload.get("policy_path"), 520),
        DEFAULT_POLICY_REL,
    );
    let log_path = resolve_path(
        &runtime_root,
        &clean_text(payload.get("log_path"), 520),
        DEFAULT_LOG_REL,
    );

    let result = match command.as_str() {
        "load-policy" => {
            let policy = resolve_policy(&runtime_root, &policy_path, Some(payload));
            Ok(json!({
                "ok": true,
                "policy_path": policy_path.to_string_lossy(),
                "log_path": log_path.to_string_lossy(),
                "policy": policy_to_value(&policy)
            }))
        }
        "collect-present-files" => {
            let policy = resolve_policy(&runtime_root, &policy_path, Some(payload));
            Ok(json!({
                "ok": true,
                "policy_path": policy_path.to_string_lossy(),
                "files": collect_present_files(&runtime_root, &policy)
            }))
        }
        "verify" => verify(&runtime_root, &policy_path, Some(payload)),
        "seal" => seal(&runtime_root, &policy_path, payload),
        "append-event" => append_event(&log_path, payload),
        _ => Err(format!(
            "security_integrity_kernel_unknown_command:{command}"
        )),
    };

    match result {
        Ok(payload) => {
            print_json_line(&cli_receipt(
                &format!("security_integrity_kernel_{}", command.replace('-', "_")),
                payload,
            ));
            0
        }
        Err(err) => {
            print_json_line(&cli_error(
                &format!("security_integrity_kernel_{}", command.replace('-', "_")),
                &err,
            ));
            1
        }
    }
}
