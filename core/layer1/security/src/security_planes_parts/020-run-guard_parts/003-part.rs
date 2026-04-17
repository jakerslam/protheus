                    json!({"ok": false, "type": "constitution_activate_change", "error": "candidate_copy_missing"}),
                    1,
                );
            }
            if paths.constitution.exists() {
                let backup_name = format!("{}_constitution.md", Utc::now().format("%Y%m%d%H%M%S"));
                let backup_path = paths.history_dir.join(backup_name);
                if let Some(parent) = backup_path.parent() {
                    let _ = fs::create_dir_all(parent);
                }
                let _ = fs::copy(&paths.constitution, &backup_path);
            }
            if let Some(parent) = paths.constitution.parent() {
                let _ = fs::create_dir_all(parent);
            }
            if let Err(err) = fs::copy(&candidate_path, &paths.constitution) {
                return (
                    json!({"ok": false, "type": "constitution_activate_change", "error": clean(format!("activate_copy_failed:{err}"), 220)}),
                    1,
                );
            }
            if let Some(obj) = proposal.as_object_mut() {
                obj.insert("status".to_string(), Value::String("active".to_string()));
                obj.insert("activated_at".to_string(), Value::String(now_iso()));
                obj.insert(
                    "activation".to_string(),
                    json!({"approver_id": approver_id, "approval_note": approval_note}),
                );
            }
            if let Err(err) = save_proposal(&paths, &proposal_id, &proposal) {
                return (
                    json!({"ok": false, "type": "constitution_activate_change", "error": clean(err, 220)}),
                    1,
                );
            }
            if let Err(err) = write_json_atomic(
                &paths.active_state,
                &json!({
                    "active_proposal_id": proposal_id,
                    "activated_at": now_iso(),
                    "constitution_sha256": sha256_hex_file(&paths.constitution).unwrap_or_default()
                }),
            ) {
                return (
                    json!({"ok": false, "type": "constitution_activate_change", "error": clean(err, 220)}),
                    1,
                );
            }
            let _ = append_jsonl(
                &paths.events,
                &json!({"ts": now_iso(), "type": "constitution_activated", "proposal_id": proposal_id}),
            );
            (
                json!({"ok": true, "type": "constitution_activate_change", "proposal": proposal}),
                0,
            )
        }
        "enforce-inheritance" => {
            let actor = clean(flag(&parsed, "actor").unwrap_or("unknown"), 120);
            let target = clean(flag(&parsed, "target").unwrap_or("unknown"), 120);
            let locked = policy.enforce_inheritance_lock;
            let out = json!({
                "ok": true,
                "type": "constitution_enforce_inheritance",
                "actor": actor,
                "target": target,
                "inheritance_lock_enforced": locked,
                "ts": now_iso()
            });
            let _ = append_jsonl(&paths.events, &out);
            (out, 0)
        }
        "emergency-rollback" => {
            let note = clean(flag(&parsed, "note").unwrap_or(""), 400);
            if policy.emergency_rollback_requires_approval
                && note.len() < policy.min_approval_note_chars
            {
                return (
                    json!({"ok": false, "type": "constitution_emergency_rollback", "error": "approval_note_too_short"}),
                    1,
                );
            }
            let mut backups = fs::read_dir(&paths.history_dir)
                .ok()
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
                .filter(|e| e.file_type().map(|t| t.is_file()).unwrap_or(false))
                .collect::<Vec<_>>();
            backups.sort_by_key(|e| e.file_name());
            let Some(entry) = backups.pop() else {
                return (
                    json!({"ok": false, "type": "constitution_emergency_rollback", "error": "no_backup_available"}),
                    1,
                );
            };
            if let Err(err) = fs::copy(entry.path(), &paths.constitution) {
                return (
                    json!({"ok": false, "type": "constitution_emergency_rollback", "error": clean(format!("rollback_copy_failed:{err}"), 220)}),
                    1,
                );
            }
            let _ = append_jsonl(
                &paths.events,
                &json!({
                    "ts": now_iso(),
                    "type": "constitution_emergency_rollback",
                    "rollback_from": normalize_rel(entry.path().to_string_lossy()),
                    "note": note
                }),
            );
            (
                json!({
                    "ok": true,
                    "type": "constitution_emergency_rollback",
                    "rollback_from": normalize_rel(entry.path().to_string_lossy())
                }),
                0,
            )
        }
        "status" => {
            let proposals = fs::read_dir(&paths.proposals_dir)
                .ok()
                .into_iter()
                .flatten()
                .filter_map(Result::ok)
                .filter(|e| e.file_type().map(|t| t.is_dir()).unwrap_or(false))
                .filter_map(|e| e.file_name().into_string().ok())
                .collect::<Vec<_>>();
            (
                json!({
                    "ok": true,
                    "type": "constitution_guardian_status",
                    "ts": now_iso(),
                    "policy_version": policy.version,
                    "constitution_path": normalize_rel(paths.constitution.to_string_lossy()),
                    "genesis": read_json_or(&paths.genesis, Value::Null),
                    "active_state": read_json_or(&paths.active_state, Value::Null),
                    "proposals_count": proposals.len(),
                    "proposals": proposals.into_iter().take(25).collect::<Vec<_>>(),
                    "state_dir": normalize_rel(paths.state_dir.to_string_lossy())
                }),
                0,
            )
        }
        _ => (
            json!({
                "ok": false,
                "type": "constitution_guardian",
                "error": "unknown_command",
                "usage": [
                    "constitution-guardian init-genesis [--force=1|0]",
                    "constitution-guardian propose-change --candidate-file=<path> --proposer-id=<id> --reason=<text>",
                    "constitution-guardian approve-change --proposal-id=<id> --approver-id=<id> --approval-note=<text>",
                    "constitution-guardian veto-change --proposal-id=<id> --veto-by=<id> --note=<text>",
                    "constitution-guardian run-gauntlet --proposal-id=<id> [--critical-failures=<n>] [--evidence=<text>]",
                    "constitution-guardian activate-change --proposal-id=<id> --approver-id=<id> --approval-note=<text>",
                    "constitution-guardian enforce-inheritance --actor=<id> --target=<id>",
                    "constitution-guardian emergency-rollback --note=<text>",
                    "constitution-guardian status"
                ]
            }),
            2,
        ),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct RemoteEmergencyHaltPolicy {
    version: String,
    enabled: bool,
    key_env: String,
    max_ttl_seconds: i64,
    max_clock_skew_seconds: i64,
    replay_nonce_ttl_seconds: i64,
    paths: RemoteEmergencyPaths,
    secure_purge: RemoteEmergencyPurge,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct RemoteEmergencyPaths {
    state: String,
    nonce_store: String,
    audit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct RemoteEmergencyPurge {
    enabled: bool,
    allow_live_purge: bool,
    confirm_phrase: String,
    sensitive_paths: Vec<String>,
}

impl Default for RemoteEmergencyPaths {
    fn default() -> Self {
        Self {
            state: "local/state/security/remote_emergency_halt_state.json".to_string(),
            nonce_store: "local/state/security/remote_emergency_halt_nonces.json".to_string(),
            audit: "local/state/security/remote_emergency_halt_audit.jsonl".to_string(),
        }
    }
}

impl Default for RemoteEmergencyPurge {
    fn default() -> Self {
        Self {
            enabled: true,
            allow_live_purge: false,
            confirm_phrase: "I UNDERSTAND THIS PURGES SENSITIVE STATE".to_string(),
            sensitive_paths: vec![
                "local/state/security/soul_token_guard.json".to_string(),
                "local/state/security/release_attestations.jsonl".to_string(),
                "local/state/security/capability_leases.json".to_string(),
                "local/state/security/capability_leases.jsonl".to_string(),
            ],
        }
    }
}

impl Default for RemoteEmergencyHaltPolicy {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            enabled: true,
            key_env: "REMOTE_EMERGENCY_HALT_KEY".to_string(),
            max_ttl_seconds: 300,
            max_clock_skew_seconds: 30,
            replay_nonce_ttl_seconds: 86_400,
            paths: RemoteEmergencyPaths::default(),
            secure_purge: RemoteEmergencyPurge::default(),
        }
    }
}

fn load_remote_emergency_policy(
    repo_root: &Path,
    parsed: &ParsedArgs,
) -> RemoteEmergencyHaltPolicy {
    let policy_path = flag(parsed, "policy")
        .map(|v| resolve_runtime_or_state(repo_root, v))
        .unwrap_or_else(|| runtime_config_path(repo_root, "remote_emergency_halt_policy.json"));
    if !policy_path.exists() {
        return RemoteEmergencyHaltPolicy::default();
    }
    match fs::read_to_string(&policy_path) {
        Ok(raw) => serde_json::from_str::<RemoteEmergencyHaltPolicy>(&raw).unwrap_or_default(),
        Err(_) => RemoteEmergencyHaltPolicy::default(),
    }
}

fn decode_b64_json(raw: &str) -> Option<Value> {
    let bytes = BASE64_STANDARD.decode(raw.as_bytes()).ok()?;
    serde_json::from_slice::<Value>(&bytes).ok()
}

fn clean_expired_nonces(store: &mut Map<String, Value>, now_ms: i64) {
    let keys = store
        .iter()
        .filter_map(|(k, v)| {
            let exp = v.as_i64().unwrap_or(0);
            if exp <= now_ms {
                Some(k.clone())
            } else {
                None
            }
        })
        .collect::<Vec<_>>();
    for key in keys {
        store.remove(&key);
    }
}

pub fn run_remote_emergency_halt(repo_root: &Path, argv: &[String]) -> (Value, i32) {
    let parsed = parse_args(argv);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let policy = load_remote_emergency_policy(repo_root, &parsed);
    let state_path = resolve_runtime_or_state(repo_root, &policy.paths.state);
    let nonce_store_path = resolve_runtime_or_state(repo_root, &policy.paths.nonce_store);
    let audit_path = resolve_runtime_or_state(repo_root, &policy.paths.audit);
    let key = std::env::var(&policy.key_env)
        .ok()
        .map(|v| clean(v, 4096))
        .unwrap_or_default();

    match cmd.as_str() {
        "status" => (
            json!({
                "ok": true,
                "type": "remote_emergency_halt_status",
                "ts": now_iso(),
                "enabled": policy.enabled,
                "key_env": policy.key_env,
                "state_path": normalize_rel(state_path.to_string_lossy()),
                "nonce_store_path": normalize_rel(nonce_store_path.to_string_lossy()),
                "state": read_json_or(&state_path, json!({"halted": false})),
                "nonces": read_json_or(&nonce_store_path, json!({"version":1, "entries": {}}))
                    .get("entries")
                    .and_then(Value::as_object)
                    .map(|m| m.len())
                    .unwrap_or(0)
            }),
            0,
        ),
        "sign-halt" | "sign-purge" => {
            if key.is_empty() {
                return (
                    json!({"ok": false, "type":"remote_emergency_halt_sign", "error":"remote_emergency_halt_key_missing", "key_env": policy.key_env}),
                    1,
                );
            }
            let action = if cmd == "sign-purge" { "purge" } else { "halt" };
            let ttl = flag(&parsed, "ttl-sec")
                .or_else(|| flag(&parsed, "ttl_sec"))
                .and_then(|v| v.parse::<i64>().ok())
                .unwrap_or(120)
                .clamp(10, policy.max_ttl_seconds.max(10));
            let now_ms = Utc::now().timestamp_millis();
            let mut payload = json!({
                "type": "remote_emergency_command",
                "action": action,
                "command_id": format!("reh_{}", &sha256_hex_bytes(format!("{}|{}", now_ms, action).as_bytes())[0..12]),
                "nonce": format!("nonce_{}", &sha256_hex_bytes(format!("{}|{}", now_ms, std::process::id()).as_bytes())[0..10]),
                "issued_at": now_iso(),
                "issued_at_ms": now_ms,
                "expires_at_ms": now_ms + ttl * 1000,
                "scope": clean(flag(&parsed, "scope").unwrap_or("all"), 120),
                "approval_note": clean(flag(&parsed, "approval-note").or_else(|| flag(&parsed, "approval_note")).unwrap_or(""), 400),
                "pending_id": clean(flag(&parsed, "pending-id").or_else(|| flag(&parsed, "pending_id")).unwrap_or(""), 120)
            });
            let signature = match hmac_sha256_hex(&key, &payload) {
                Ok(v) => v,
                Err(err) => return (json!({"ok": false, "error": clean(err, 220)}), 1),
            };
            if let Some(obj) = payload.as_object_mut() {
                obj.insert("signature".to_string(), Value::String(signature));
            }
            (
                json!({
                    "ok": true,
                    "type": "remote_emergency_halt_sign",
                    "action": action,
                    "command": payload.clone(),
                    "command_b64": BASE64_STANDARD.encode(stable_json_string(&payload))
                }),
                0,
            )
        }
        "receive-b64" => {
            let raw = clean(
                flag(&parsed, "command-b64")
                    .or_else(|| flag(&parsed, "command_b64"))
                    .unwrap_or(""),
                16_384,
            );
            if raw.is_empty() {
                return (
                    json!({"ok": false, "type":"remote_emergency_halt_receive", "error":"command_b64_required"}),
                    1,
                );
            }
            let Some(cmd_payload) = decode_b64_json(&raw) else {
                return (
                    json!({"ok": false, "type":"remote_emergency_halt_receive", "error":"command_b64_invalid"}),
                    1,
                );
            };
            let mut next_args = vec!["receive".to_string()];
            next_args.push(format!("--command={}", stable_json_string(&cmd_payload)));
            run_remote_emergency_halt(repo_root, &next_args)
        }
        "receive" => {
            if key.is_empty() {
                return (
                    json!({"ok": false, "type":"remote_emergency_halt_receive", "error":"remote_emergency_halt_key_missing", "key_env": policy.key_env}),
                    1,
                );
            }
            let raw_cmd = clean(flag(&parsed, "command").unwrap_or(""), 32_000);
            if raw_cmd.is_empty() {
                return (
                    json!({"ok": false, "type":"remote_emergency_halt_receive", "error":"command_required"}),
                    1,
                );
            }
            let mut payload = match serde_json::from_str::<Value>(&raw_cmd) {
                Ok(v) => v,
                Err(_) => {
                    return (
                        json!({"ok": false, "type":"remote_emergency_halt_receive", "error":"command_json_invalid"}),
                        1,
                    )
                }
            };
            let Some(signature) = payload
                .get("signature")
                .and_then(Value::as_str)
                .map(|v| clean(v, 240))
            else {
                return (
                    json!({"ok": false, "type":"remote_emergency_halt_receive", "error":"signature_missing"}),
                    1,
                );
            };
            if let Some(obj) = payload.as_object_mut() {
                obj.remove("signature");
            }
            let expected = match hmac_sha256_hex(&key, &payload) {
                Ok(v) => v,
                Err(err) => return (json!({"ok":false, "error": clean(err, 220)}), 1),
            };
            if !secure_eq_hex(&signature, &expected) {
                return (
