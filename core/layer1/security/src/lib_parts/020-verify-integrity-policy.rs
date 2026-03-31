fn verify_integrity_policy(repo_root: &Path, policy_path: &Path) -> Value {
    let runtime = runtime_root(repo_root);
    let policy = load_integrity_policy(policy_path);
    let present_files = collect_integrity_present_files(&runtime, &policy);
    let mut violations = Vec::<Value>::new();

    if policy.hashes.is_empty() {
        violations.push(json!({
            "type": "policy_unsealed",
            "file": Value::Null,
            "detail": "hashes_empty"
        }));
    }

    for (rel, expected) in &policy.hashes {
        let abs = runtime.join(rel);
        if !abs.exists() {
            violations.push(json!({"type":"missing_sealed_file","file":rel}));
            continue;
        }
        let digest = expected.to_ascii_lowercase();
        if digest.len() != 64 || !digest.chars().all(|ch| ch.is_ascii_hexdigit()) {
            violations.push(json!({"type":"invalid_hash_entry","file":rel,"expected":digest}));
            continue;
        }
        match sha256_hex_file(&abs) {
            Ok(actual) => {
                if actual != digest {
                    violations.push(
                        json!({"type":"hash_mismatch","file":rel,"expected":digest,"actual":actual}),
                    );
                }
            }
            Err(_) => violations.push(json!({"type":"read_failed","file":rel})),
        }
    }

    let expected_set = policy
        .hashes
        .keys()
        .map(|v| normalize_rel(v))
        .collect::<BTreeSet<_>>();
    let present_set = present_files
        .iter()
        .map(normalize_rel)
        .collect::<BTreeSet<_>>();

    for rel in &present_files {
        if !expected_set.contains(rel) {
            violations.push(json!({"type":"unsealed_file","file":rel}));
        }
    }

    for rel in expected_set {
        if !present_set.contains(&rel) {
            let already_missing = violations.iter().any(|row| {
                row.get("type").and_then(Value::as_str) == Some("missing_sealed_file")
                    && row.get("file").and_then(Value::as_str) == Some(rel.as_str())
            });
            if !already_missing {
                violations.push(json!({"type":"sealed_file_outside_scope","file":rel}));
            }
        }
    }

    let counts = summarize_violation_counts(&violations);
    json!({
        "ok": violations.is_empty(),
        "ts": now_iso(),
        "policy_path": policy_path.to_string_lossy(),
        "policy_version": policy.version,
        "checked_present_files": present_files.len(),
        "expected_files": policy.hashes.len(),
        "violations": violations,
        "violation_counts": counts
    })
}

fn git_changed_paths(repo_root: &Path, staged: bool) -> Vec<String> {
    let args = if staged {
        vec![
            "diff".to_string(),
            "--name-only".to_string(),
            "--cached".to_string(),
        ]
    } else {
        vec!["diff".to_string(), "--name-only".to_string()]
    };
    let output = Command::new("git")
        .args(args)
        .current_dir(repo_root)
        .output();
    let Ok(out) = output else {
        return Vec::new();
    };
    if !out.status.success() {
        return Vec::new();
    }
    String::from_utf8_lossy(&out.stdout)
        .lines()
        .map(normalize_rel)
        .filter(|row| !row.is_empty())
        .map(|row| {
            row.strip_prefix("client/runtime/")
                .map(|v| v.to_string())
                .unwrap_or(row)
        })
        .collect::<Vec<_>>()
}

fn seal_integrity_policy(
    repo_root: &Path,
    policy_path: &Path,
    approval_note: Option<&str>,
    sealed_by: Option<&str>,
) -> Result<Value, String> {
    let runtime = runtime_root(repo_root);
    let mut policy = load_integrity_policy(policy_path);
    let present = collect_integrity_present_files(&runtime, &policy);
    let mut hashes = BTreeMap::<String, String>::new();
    for rel in &present {
        let digest = sha256_hex_file(&runtime.join(rel))?;
        hashes.insert(rel.clone(), digest);
    }

    policy.hashes = hashes;
    let mut out = serde_json::to_value(&policy)
        .map_err(|err| format!("encode_integrity_policy_failed:{err}"))?;
    let sealed_by_value = sealed_by
        .map(ToString::to_string)
        .or_else(|| std::env::var("USER").ok())
        .unwrap_or_else(|| "unknown".to_string());
    if let Some(obj) = out.as_object_mut() {
        obj.insert("sealed_at".to_string(), Value::String(now_iso()));
        obj.insert(
            "sealed_by".to_string(),
            Value::String(clean(sealed_by_value, 120)),
        );
        if let Some(note) = approval_note {
            let clean_note = clean(note, 240);
            if !clean_note.is_empty() {
                obj.insert("last_approval_note".to_string(), Value::String(clean_note));
            }
        }
    }
    write_json_atomic(policy_path, &out)?;

    Ok(json!({
        "ok": true,
        "policy_path": policy_path.to_string_lossy(),
        "policy_version": policy.version,
        "sealed_files": present.len(),
        "sealed_at": now_iso()
    }))
}

pub fn run_integrity_reseal(repo_root: &Path, argv: &[String]) -> (Value, i32) {
    let parsed = parse_args(argv);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    let policy_path = flag(&parsed, "policy")
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            runtime_root(repo_root)
                .join("config")
                .join("security_integrity_policy.json")
        });

    match cmd.as_str() {
        "check" | "status" | "run" => {
            let staged = bool_flag(&parsed, "staged", true);
            let verify = verify_integrity_policy(repo_root, &policy_path);
            let protected_set = {
                let policy = load_integrity_policy(&policy_path);
                let present = collect_integrity_present_files(&runtime_root(repo_root), &policy);
                let expected = policy.hashes.keys().map(normalize_rel).collect::<Vec<_>>();
                present
                    .into_iter()
                    .chain(expected.into_iter())
                    .collect::<BTreeSet<_>>()
            };
            let changed = git_changed_paths(repo_root, staged)
                .into_iter()
                .filter(|row| protected_set.contains(row))
                .collect::<Vec<_>>();

            let ok = verify.get("ok").and_then(Value::as_bool).unwrap_or(false);
            let out = json!({
                "ok": ok,
                "ts": now_iso(),
                "type": "integrity_reseal_check",
                "policy_path": policy_path.to_string_lossy(),
                "staged": staged,
                "protected_changes": changed,
                "reseal_required": !ok,
                "violation_counts": verify.get("violation_counts").cloned().unwrap_or_else(|| json!({})),
                "violations": verify
                    .get("violations")
                    .and_then(Value::as_array)
                    .map(|rows| rows.iter().take(12).cloned().collect::<Vec<_>>())
                    .unwrap_or_default()
            });
            let code = if ok { 0 } else { 1 };
            (out, code)
        }
        "apply" | "reseal" | "seal" => {
            let force = bool_flag(&parsed, "force", false);
            let note = flag(&parsed, "approval-note")
                .or_else(|| flag(&parsed, "approval_note"))
                .map(ToString::to_string)
                .or_else(|| std::env::var("INTEGRITY_RESEAL_NOTE").ok())
                .unwrap_or_default();
            let verify_before = verify_integrity_policy(repo_root, &policy_path);
            let already_ok = verify_before
                .get("ok")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if already_ok && !force {
                return (
                    json!({
                        "ok": true,
                        "ts": now_iso(),
                        "type": "integrity_reseal_apply",
                        "policy_path": policy_path.to_string_lossy(),
                        "applied": false,
                        "reason": "already_sealed"
                    }),
                    0,
                );
            }
            if clean(&note, 500).len() < 10 {
                return (
                    json!({
                        "ok": false,
                        "type": "integrity_reseal_apply",
                        "error": "approval_note_too_short",
                        "min_len": 10
                    }),
                    2,
                );
            }
            match seal_integrity_policy(
                repo_root,
                &policy_path,
                Some(&note),
                std::env::var("USER").ok().as_deref(),
            ) {
                Ok(seal) => {
                    let verify_after = verify_integrity_policy(repo_root, &policy_path);
                    let ok = verify_after
                        .get("ok")
                        .and_then(Value::as_bool)
                        .unwrap_or(false);
                    (
                        json!({
                            "ok": ok,
                            "ts": now_iso(),
                            "type": "integrity_reseal_apply",
                            "policy_path": policy_path.to_string_lossy(),
                            "applied": true,
                            "seal": seal,
                            "verify": {
                                "ok": ok,
                                "violation_counts": verify_after.get("violation_counts").cloned().unwrap_or_else(|| json!({})),
                                "violations": verify_after
                                    .get("violations")
                                    .and_then(Value::as_array)
                                    .map(|rows| rows.iter().take(12).cloned().collect::<Vec<_>>())
                                    .unwrap_or_default()
                            }
                        }),
                        if ok { 0 } else { 1 },
                    )
                }
                Err(err) => (
                    json!({
                        "ok": false,
                        "type": "integrity_reseal_apply",
                        "error": clean(err, 220)
                    }),
                    1,
                ),
            }
        }
        _ => (
            json!({
                "ok": false,
                "type": "integrity_reseal",
                "error": "unknown_command",
                "usage": [
                    "integrity-reseal check [--policy=<path>] [--staged=1|0]",
                    "integrity-reseal apply [--policy=<path>] [--approval-note=<text>] [--force=1]"
                ]
            }),
            2,
        ),
    }
}

pub fn run_integrity_reseal_assistant(repo_root: &Path, argv: &[String]) -> (Value, i32) {
    let parsed = parse_args(argv);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    match cmd.as_str() {
        "run" => {
            let mut check_args = vec!["check".to_string(), "--staged=0".to_string()];
            if let Some(policy) = flag(&parsed, "policy") {
                check_args.push(format!("--policy={policy}"));
            }
            let (check_out, _) = run_integrity_reseal(repo_root, &check_args);
            let reseal_required = !check_out
                .get("ok")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                || check_out
                    .get("reseal_required")
                    .and_then(Value::as_bool)
                    .unwrap_or(false);
            let apply = bool_flag(&parsed, "apply", true);
            let strict = bool_flag(&parsed, "strict", false);
            let mut applied = false;
            let mut apply_result = Value::Null;
            let mut ok = true;
            if reseal_required && apply {
                let auto_note = {
                    let violations = check_out
                        .get("violations")
                        .and_then(Value::as_array)
                        .cloned()
                        .unwrap_or_default();
                    let files = violations
                        .iter()
                        .filter_map(|row| row.get("file").and_then(Value::as_str))
                        .map(|v| clean(v, 120))
                        .filter(|v| !v.is_empty())
                        .take(8)
                        .collect::<Vec<_>>();
                    let focus = if files.is_empty() {
                        "files=none".to_string()
                    } else {
                        format!("files={}", files.join(","))
                    };
                    format!(
                        "Automated integrity reseal assistant run ({focus}) at {}",
                        now_iso()
                    )
                };
                let note = flag(&parsed, "note")
                    .map(ToString::to_string)
                    .unwrap_or(auto_note);
                let mut apply_args = vec!["apply".to_string(), format!("--approval-note={note}")];
                if let Some(policy) = flag(&parsed, "policy") {
                    apply_args.push(format!("--policy={policy}"));
                }
                let (apply_out, apply_code) = run_integrity_reseal(repo_root, &apply_args);
                applied = true;
                apply_result = apply_out.clone();
                ok = apply_code == 0;
            }
            let out = json!({
                "ok": ok,
                "type": "integrity_reseal_assistant",
                "ts": now_iso(),
                "apply": apply,
                "strict": strict,
                "policy": flag(&parsed, "policy"),
                "reseal_required": reseal_required,
                "check": check_out,
                "applied": applied,
                "apply_result": if applied { apply_result } else { Value::Null }
            });
            let code = if strict && !ok {
                1
            } else if ok {
                0
            } else {
                1
            };
            (out, code)
        }
        "status" => {
            let mut args = vec!["check".to_string()];
            if let Some(policy) = flag(&parsed, "policy") {
                args.push(format!("--policy={policy}"));
            }
            let (check_out, code) = run_integrity_reseal(repo_root, &args);
            let ok = code == 0;
            (
                json!({
                    "ok": ok,
                    "type": "integrity_reseal_assistant_status",
                    "ts": now_iso(),
                    "reseal_required": !ok || check_out.get("reseal_required").and_then(Value::as_bool).unwrap_or(false),
                    "check": check_out
                }),
                if ok { 0 } else { 1 },
            )
        }
        _ => (
            json!({
                "ok": false,
                "type": "integrity_reseal_assistant",
                "error": "unknown_command",
                "usage": [
                    "integrity-reseal-assistant run [--apply=1|0] [--policy=<path>] [--note=<text>] [--strict=1|0]",
                    "integrity-reseal-assistant status [--policy=<path>]"
                ]
            }),
            2,
        ),
    }
}

fn emergency_stop_state_path(repo_root: &Path) -> PathBuf {
    runtime_root(repo_root)
        .join("local")
        .join("state")
        .join("security")
        .join("emergency_stop.json")
}

fn emergency_stop_valid_scopes() -> Vec<&'static str> {
    vec!["all", "autonomy", "routing", "actuation", "spine"]
}

fn emergency_stop_normalize_scopes(raw: Option<&str>) -> Vec<String> {
    let valid = emergency_stop_valid_scopes();
    let mut out = Vec::<String>::new();
    let input = raw.unwrap_or("all");
    for seg in input.split(',') {
        let scope = clean(seg, 64).to_ascii_lowercase();
        if scope.is_empty() {
            continue;
        }
        if !valid.iter().any(|row| *row == scope) {
            continue;
        }
        if !out.iter().any(|row| row == &scope) {
            out.push(scope);
        }
    }
    if out.is_empty() {
        out.push("all".to_string());
    }
    if out.iter().any(|row| row == "all") {
        return vec!["all".to_string()];
    }
    out.sort();
    out
}

