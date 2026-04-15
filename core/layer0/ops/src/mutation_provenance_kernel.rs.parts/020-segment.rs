fn load_policy(root: &Path, payload: &Map<String, Value>) -> Value {
    let fallback = json!({
        "version": "1.0-fallback",
        "channels": {
            "adaptive": {
                "allowed_source_prefixes": [
                    "systems/adaptive/",
                    "systems/sensory/",
                    "systems/strategy/",
                    "systems/autonomy/",
                    "systems/spine/",
                    "lib/"
                ],
                "require_reason": true
            },
            "memory": {
                "allowed_source_prefixes": [
                    "systems/memory/",
                    "systems/spine/",
                    "systems/adaptive/core/",
                    "lib/"
                ],
                "require_reason": true
            }
        }
    });
    let path = resolve_policy_path(root, payload);
    let mut policy = read_json_safe(&path, fallback.clone());
    if policy.get("channels").and_then(Value::as_object).is_none() {
        policy["channels"] = fallback
            .get("channels")
            .cloned()
            .unwrap_or_else(|| json!({}));
    }
    if policy
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("")
        .trim()
        .is_empty()
    {
        policy["version"] = Value::String("1.0-fallback".to_string());
    }
    policy
}

fn channel_config(policy: &Value, channel: &str) -> (Vec<String>, bool) {
    let channel_obj = policy
        .get("channels")
        .and_then(Value::as_object)
        .and_then(|row| row.get(channel))
        .and_then(Value::as_object);
    let prefixes = channel_obj
        .and_then(|row| row.get("allowed_source_prefixes"))
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .map(|row| normalize_path_string(&as_str(Some(row))))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let require_reason = channel_obj
        .and_then(|row| row.get("require_reason"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    (prefixes, require_reason)
}

fn parse_bool(value: Option<&Value>, fallback: bool) -> bool {
    match value {
        Some(Value::Bool(v)) => *v,
        Some(Value::String(v)) => lane_utils::parse_bool(Some(v.as_str()), fallback),
        Some(Value::Number(n)) => n.as_i64().map(|row| row != 0).unwrap_or(fallback),
        _ => fallback,
    }
}

fn is_strict(channel: &str, opts: Option<&Map<String, Value>>) -> bool {
    if parse_bool(opts.and_then(|row| row.get("strict")), false) {
        return true;
    }
    if std::env::var("MUTATION_PROVENANCE_STRICT")
        .ok()
        .map(|row| lane_utils::parse_bool(Some(row.as_str()), false))
        .unwrap_or(false)
    {
        return true;
    }
    match channel {
        "adaptive" => std::env::var("ADAPTIVE_MUTATION_STRICT")
            .ok()
            .map(|row| lane_utils::parse_bool(Some(row.as_str()), false))
            .unwrap_or(false),
        "memory" => std::env::var("MEMORY_MUTATION_STRICT")
            .ok()
            .map(|row| lane_utils::parse_bool(Some(row.as_str()), false))
            .unwrap_or(false),
        _ => false,
    }
}

fn violation_path(root: &Path, channel: &str) -> PathBuf {
    client_root(root)
        .join("local")
        .join("state")
        .join("security")
        .join(format!("{channel}_mutation_violations.jsonl"))
}

fn audit_path(root: &Path, channel: &str) -> PathBuf {
    client_root(root)
        .join("local")
        .join("state")
        .join("security")
        .join(format!("{channel}_mutations.jsonl"))
}

fn enforce_value(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let channel = clean_text(payload.get("channel"), 64).to_ascii_lowercase();
    let fallback_source = clean_text(payload.get("fallback_source"), 240);
    let default_reason = clean_text(payload.get("default_reason"), 160);
    let opts = as_object(payload.get("opts"));
    let policy = load_policy(root, payload);
    let policy_version = as_str(policy.get("version"));
    let (prefixes, require_reason) = channel_config(&policy, &channel);
    let normalized = normalize_meta_value(
        root,
        as_object(payload.get("meta")),
        &fallback_source,
        &default_reason,
    );
    let normalized_obj = payload_obj(&normalized);
    let source = as_str(normalized_obj.get("source"));
    let mut violations = Vec::<String>::new();

    if source.is_empty() {
        violations.push("missing_source".to_string());
    } else {
        let allowed = prefixes.iter().any(|prefix| {
            let exact = prefix.trim_end_matches('/');
            source == exact || source.starts_with(prefix)
        });
        if !allowed {
            violations.push("source_not_allowlisted".to_string());
        }
    }
    if require_reason && clean_text(normalized_obj.get("reason"), 160).is_empty() {
        violations.push("missing_reason".to_string());
    }

    let out = json!({
        "ok": violations.is_empty(),
        "channel": channel,
        "policy_version": policy_version,
        "meta": normalized,
        "source_rel": source,
        "violations": violations,
    });

    if !out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        append_jsonl(
            &violation_path(
                root,
                out.get("channel")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
            ),
            &json!({
                "ts": now_iso(),
                "type": "mutation_provenance_violation",
                "channel": out.get("channel").cloned().unwrap_or(Value::Null),
                "policy_version": out.get("policy_version").cloned().unwrap_or(Value::Null),
                "source": if source.is_empty() { Value::Null } else { Value::String(source.clone()) },
                "reason": normalized_obj.get("reason").cloned().unwrap_or(Value::Null),
                "actor": normalized_obj.get("actor").cloned().unwrap_or(Value::Null),
                "context": clean_text(opts.and_then(|row| row.get("context")), 200),
                "violations": out.get("violations").cloned().unwrap_or_else(|| json!([])),
            }),
        )?;
        if is_strict(
            out.get("channel").and_then(Value::as_str).unwrap_or(""),
            opts,
        ) {
            let reasons = out
                .get("violations")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .unwrap_or_default();
            return Err(format!(
                "mutation_provenance_blocked:{}:{}",
                out.get("channel")
                    .and_then(Value::as_str)
                    .unwrap_or("unknown"),
                reasons
            ));
        }
    }

    Ok(out)
}

fn record_audit_value(root: &Path, payload: &Map<String, Value>) -> Result<Value, String> {
    let channel = clean_text(payload.get("channel"), 64).to_ascii_lowercase();
    let row = as_object(payload.get("row")).cloned().unwrap_or_default();
    let mut audit_row = Map::new();
    audit_row.insert("ts".to_string(), Value::String(now_iso()));
    audit_row.insert("channel".to_string(), Value::String(channel.clone()));
    for (key, value) in row {
        audit_row.insert(key, value);
    }
    let target = audit_path(root, &channel);
    append_jsonl(&target, &Value::Object(audit_row))?;
    Ok(json!({
        "ok": true,
        "channel": channel,
        "audit_path": target.to_string_lossy(),
    }))
}

fn run_command(root: &Path, command: &str, payload: &Map<String, Value>) -> Result<Value, String> {
    match command {
        "load-policy" => {
            let policy = load_policy(root, payload);
            Ok(json!({
                "ok": true,
                "policy": policy,
                "policy_path": resolve_policy_path(root, payload).to_string_lossy(),
            }))
        }
        "normalize-meta" => Ok(json!({
            "ok": true,
            "meta": normalize_meta_value(
                root,
                as_object(payload.get("meta")),
                &clean_text(payload.get("fallback_source"), 240),
                &clean_text(payload.get("default_reason"), 160),
            ),
        })),
        "enforce" => enforce_value(root, payload),
        "record-audit" => record_audit_value(root, payload),
        _ => Err("mutation_provenance_kernel_unknown_command".to_string()),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let Some(command) = argv.first().map(|row| row.as_str()) else {
        usage();
        return 1;
    };
    if matches!(command, "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let payload = match payload_json(argv) {
        Ok(value) => value,
        Err(err) => {
            print_json_line(&cli_error("mutation_provenance_kernel", &err));
            return 1;
        }
    };
    match run_command(root, command, payload_obj(&payload)) {
        Ok(out) => {
            print_json_line(&cli_receipt("mutation_provenance_kernel", out));
            0
        }
        Err(err) => {
            print_json_line(&cli_error("mutation_provenance_kernel", &err));
            1
        }
    }
}

