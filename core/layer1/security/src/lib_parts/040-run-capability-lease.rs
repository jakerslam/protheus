pub fn run_capability_lease(repo_root: &Path, argv: &[String]) -> (Value, i32) {
    let parsed = parse_args(argv);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    let Some(key) = lease_key() else {
        return (
            json!({
                "ok": false,
                "error": "capability_lease_key_missing"
            }),
            1,
        );
    };
    let state_path = lease_state_path(repo_root);
    let audit_path = lease_audit_path(repo_root);
    let min_ttl = std::env::var("CAPABILITY_LEASE_MIN_TTL_SEC")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(30);
    let max_ttl = std::env::var("CAPABILITY_LEASE_MAX_TTL_SEC")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(3600);
    let default_ttl = std::env::var("CAPABILITY_LEASE_DEFAULT_TTL_SEC")
        .ok()
        .and_then(|v| v.parse::<i64>().ok())
        .unwrap_or(300);

    match cmd.as_str() {
        "issue" => {
            let scope = clean(flag(&parsed, "scope").unwrap_or(""), 180);
            if scope.is_empty() {
                return (json!({"ok":false,"error":"scope_required"}), 1);
            }
            let target = flag(&parsed, "target")
                .map(|v| clean(v, 240))
                .filter(|v| !v.is_empty());
            let issued_by = clean(
                flag(&parsed, "issued-by")
                    .or_else(|| flag(&parsed, "issued_by"))
                    .unwrap_or("unknown"),
                120,
            );
            let reason = flag(&parsed, "reason")
                .map(|v| clean(v, 240))
                .filter(|v| !v.is_empty());
            let ttl_raw = flag(&parsed, "ttl-sec")
                .or_else(|| flag(&parsed, "ttl_sec"))
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(default_ttl)
                .clamp(min_ttl, max_ttl);
            let issued_at_ms = Utc::now().timestamp_millis();
            let expires_at_ms = issued_at_ms + ttl_raw * 1000;
            let payload = json!({
                "v": "1.0",
                "id": lease_make_id(&scope, target.as_deref()),
                "scope": scope,
                "target": target,
                "issued_at_ms": issued_at_ms,
                "issued_at": now_iso(),
                "expires_at_ms": expires_at_ms,
                "expires_at": chrono::DateTime::<Utc>::from_timestamp_millis(expires_at_ms)
                    .map(|dt| dt.to_rfc3339_opts(SecondsFormat::Millis, true))
                    .unwrap_or_else(now_iso),
                "issued_by": issued_by,
                "reason": reason,
                "nonce": &sha256_hex_bytes(format!("{}:{}", issued_at_ms, std::process::id()).as_bytes())[..16]
            });
            let token = match lease_pack_token(&payload, &key) {
                Ok(v) => v,
                Err(err) => return (json!({"ok":false,"error":clean(err,220)}), 1),
            };
            let id = payload
                .get("id")
                .and_then(Value::as_str)
                .unwrap_or("lease_unknown")
                .to_string();
            let mut state = load_lease_state(&state_path);
            state.issued.insert(
                id.clone(),
                json!({
                    "id": id,
                    "scope": payload.get("scope").cloned().unwrap_or(Value::Null),
                    "target": payload.get("target").cloned().unwrap_or(Value::Null),
                    "issued_at": payload.get("issued_at").cloned().unwrap_or(Value::Null),
                    "expires_at": payload.get("expires_at").cloned().unwrap_or(Value::Null),
                    "issued_by": payload.get("issued_by").cloned().unwrap_or(Value::Null),
                    "reason": payload.get("reason").cloned().unwrap_or(Value::Null)
                }),
            );
            if let Err(err) = save_lease_state(&state_path, &state) {
                return (json!({"ok":false,"error":clean(err,220)}), 1);
            }
            let _ = append_jsonl(
                &audit_path,
                &json!({
                    "ts": now_iso(),
                    "type": "capability_lease_issued",
                    "lease_id": id,
                    "scope": payload.get("scope").cloned().unwrap_or(Value::Null),
                    "target": payload.get("target").cloned().unwrap_or(Value::Null),
                    "ttl_sec": ttl_raw,
                    "issued_by": payload.get("issued_by").cloned().unwrap_or(Value::Null)
                }),
            );
            (
                json!({
                    "ok": true,
                    "lease_id": payload.get("id").cloned().unwrap_or(Value::Null),
                    "scope": payload.get("scope").cloned().unwrap_or(Value::Null),
                    "target": payload.get("target").cloned().unwrap_or(Value::Null),
                    "expires_at": payload.get("expires_at").cloned().unwrap_or(Value::Null),
                    "ttl_sec": ttl_raw,
                    "token": token,
                    "lease_state_path": state_path.to_string_lossy(),
                    "lease_audit_path": audit_path.to_string_lossy()
                }),
                0,
            )
        }
        "verify" | "consume" => {
            let token = clean(flag(&parsed, "token").unwrap_or(""), 16_384);
            if token.is_empty() {
                return (json!({"ok":false,"error":"token_required"}), 1);
            }
            let (body, sig, payload) = match lease_unpack_token(&token) {
                Ok(v) => v,
                Err(err) => return (json!({"ok":false,"error":err}), 1),
            };
            let expected = match lease_sign(&body, &key) {
                Ok(v) => v,
                Err(err) => return (json!({"ok":false,"error":clean(err,220)}), 1),
            };
            if !secure_eq_hex(&sig, &expected) {
                return (json!({"ok":false,"error":"token_signature_invalid"}), 1);
            }

            let lease_id = clean(payload.get("id").and_then(Value::as_str).unwrap_or(""), 120);
            if lease_id.is_empty() {
                return (json!({"ok":false,"error":"token_missing_id"}), 1);
            }
            let lease_scope = clean(
                payload.get("scope").and_then(Value::as_str).unwrap_or(""),
                180,
            );
            let lease_target = payload
                .get("target")
                .and_then(Value::as_str)
                .map(|v| clean(v, 240))
                .filter(|v| !v.is_empty());
            let expires_at_ms = payload
                .get("expires_at_ms")
                .and_then(Value::as_i64)
                .unwrap_or(0);
            let now_ms = Utc::now().timestamp_millis();
            if expires_at_ms <= now_ms {
                return (
                    json!({
                        "ok": false,
                        "error": "lease_expired",
                        "lease_id": lease_id,
                        "expires_at": payload.get("expires_at").cloned().unwrap_or(Value::Null)
                    }),
                    1,
                );
            }
            if let Some(want_scope) = flag(&parsed, "scope") {
                if clean(want_scope, 180) != lease_scope {
                    return (
                        json!({
                            "ok": false,
                            "error": "scope_mismatch",
                            "lease_scope": lease_scope,
                            "required_scope": clean(want_scope,180),
                            "lease_id": lease_id
                        }),
                        1,
                    );
                }
            }
            if let Some(want_target) = flag(&parsed, "target") {
                let clean_target = clean(want_target, 240);
                if !clean_target.is_empty()
                    && lease_target.as_deref().unwrap_or_default() != clean_target
                {
                    return (
                        json!({
                            "ok": false,
                            "error": "target_mismatch",
                            "lease_target": lease_target,
                            "required_target": clean_target,
                            "lease_id": lease_id
                        }),
                        1,
                    );
                }
            }

            let mut state = load_lease_state(&state_path);
            if state.consumed.contains_key(&lease_id) {
                return (
                    json!({
                        "ok": false,
                        "error": "lease_already_consumed",
                        "lease_id": lease_id,
                        "consumed_at": state.consumed.get(&lease_id).and_then(|v| v.get("ts")).cloned().unwrap_or(Value::Null)
                    }),
                    1,
                );
            }
            if !state.issued.contains_key(&lease_id) {
                return (
                    json!({"ok":false,"error":"lease_unknown","lease_id":lease_id}),
                    1,
                );
            }

            let consume = cmd == "consume";
            if consume {
                let reason = clean(flag(&parsed, "reason").unwrap_or("consumed"), 180);
                state.consumed.insert(
                    lease_id.clone(),
                    json!({"ts": now_iso(), "reason": reason.clone()}),
                );
                if let Err(err) = save_lease_state(&state_path, &state) {
                    return (json!({"ok":false,"error":clean(err,220)}), 1);
                }
                let _ = append_jsonl(
                    &audit_path,
                    &json!({
                        "ts": now_iso(),
                        "type": "capability_lease_consumed",
                        "lease_id": lease_id,
                        "scope": lease_scope,
                        "target": lease_target,
                        "reason": reason
                    }),
                );
            }

            (
                json!({
                    "ok": true,
                    "lease_id": lease_id,
                    "scope": lease_scope,
                    "target": lease_target,
                    "expires_at": payload.get("expires_at").cloned().unwrap_or(Value::Null),
                    "consumed": consume
                }),
                0,
            )
        }
        _ => (
            json!({
                "ok": false,
                "error": "unknown_command",
                "usage": [
                    "capability-lease issue --scope=<scope> [--target=<target>] [--ttl-sec=<n>] [--issued-by=<id>] [--reason=<text>]",
                    "capability-lease verify --token=<token> [--scope=<scope>] [--target=<target>]",
                    "capability-lease consume --token=<token> [--scope=<scope>] [--target=<target>] [--reason=<text>]"
                ]
            }),
            2,
        ),
    }
}

fn startup_policy_path(repo_root: &Path) -> PathBuf {
    std::env::var("STARTUP_ATTESTATION_POLICY_PATH")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            runtime_root(repo_root)
                .join("config")
                .join("startup_attestation_policy.json")
        })
}

fn startup_state_path(repo_root: &Path) -> PathBuf {
    std::env::var("STARTUP_ATTESTATION_STATE_PATH")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            state_root(repo_root)
                .join("security")
                .join("startup_attestation.json")
        })
}

fn startup_audit_path(repo_root: &Path) -> PathBuf {
    std::env::var("STARTUP_ATTESTATION_AUDIT_PATH")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            state_root(repo_root)
                .join("security")
                .join("startup_attestation_audit.jsonl")
        })
}

fn startup_secret_candidates(repo_root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::<PathBuf>::new();
    if let Ok(v) = std::env::var("STARTUP_ATTESTATION_KEY_PATH") {
        let clean_v = clean(v, 520);
        if !clean_v.is_empty() {
            out.push(PathBuf::from(clean_v));
        }
    }
    if let Ok(v) = std::env::var("SECRET_BROKER_LOCAL_KEY_PATH") {
        let clean_v = clean(v, 520);
        if !clean_v.is_empty() {
            out.push(PathBuf::from(clean_v));
        }
    }
    if let Ok(home) = std::env::var("HOME") {
        let base = PathBuf::from(home)
            .join(".config")
            .join("protheus")
            .join("secrets");
        out.push(base.join("startup_attestation_key.txt"));
        out.push(base.join("secret_broker_key.txt"));
    }
    out.push(
        state_root(repo_root)
            .join("security")
            .join("secret_broker_key.txt"),
    );
    out.push(
        runtime_root(repo_root)
            .join("local")
            .join("state")
            .join("security")
            .join("secret_broker_key.txt"),
    );
    out
}

fn startup_resolve_secret(repo_root: &Path) -> Option<String> {
    if let Ok(v) = std::env::var("STARTUP_ATTESTATION_KEY") {
        let c = clean(v, 4096);
        if !c.is_empty() {
            return Some(c);
        }
    }
    if let Ok(v) = std::env::var("SECRET_BROKER_KEY") {
        let c = clean(v, 4096);
        if !c.is_empty() {
            return Some(c);
        }
    }
    for candidate in startup_secret_candidates(repo_root) {
        if !candidate.exists() {
            continue;
        }
        if let Ok(raw) = fs::read_to_string(&candidate) {
            let c = clean(raw, 4096);
            if !c.is_empty() {
                return Some(c);
            }
        }
    }
    None
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct StartupPolicy {
    version: String,
    ttl_hours: i64,
    critical_paths: Vec<String>,
}

impl Default for StartupPolicy {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            ttl_hours: 24,
            critical_paths: Vec::new(),
        }
    }
}

fn load_startup_policy(path: &Path) -> StartupPolicy {
    let mut policy = StartupPolicy::default();
    if path.exists() {
        if let Ok(raw) = fs::read_to_string(path) {
            if let Ok(parsed) = serde_json::from_str::<StartupPolicy>(&raw) {
                policy = parsed;
            }
        }
    }
    policy.version = clean(policy.version, 40);
    if policy.version.is_empty() {
        policy.version = "1.0".to_string();
    }
    policy.ttl_hours = policy.ttl_hours.clamp(1, 240);
    policy.critical_paths = policy
        .critical_paths
        .into_iter()
        .map(normalize_rel)
        .filter(|v| !v.is_empty() && !v.contains(".."))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    policy
}

fn startup_hash_critical_paths(
    repo_root: &Path,
    policy: &StartupPolicy,
) -> (Vec<Value>, Vec<String>) {
    let runtime = runtime_root(repo_root);
    let mut rows = Vec::<Value>::new();
    let mut missing = Vec::<String>::new();
    for rel in &policy.critical_paths {
        let abs = runtime.join(rel);
        if !abs.exists() || !abs.is_file() {
            missing.push(rel.clone());
            continue;
        }
        match sha256_hex_file(&abs) {
            Ok(digest) => {
                let size_bytes = fs::metadata(&abs).map(|m| m.len()).unwrap_or(0);
                rows.push(json!({"path": rel, "sha256": digest, "size_bytes": size_bytes}));
            }
            Err(_) => missing.push(rel.clone()),
        }
    }
    rows.sort_by(|a, b| {
        a.get("path")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(b.get("path").and_then(Value::as_str).unwrap_or(""))
    });
    missing.sort();
    (rows, missing)
}

