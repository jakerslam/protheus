fn emergency_stop_state_default() -> Value {
    json!({
        "engaged": false,
        "scopes": [],
        "updated_at": Value::Null,
        "reason": Value::Null,
        "actor": Value::Null,
        "approval_note": Value::Null
    })
}

fn emergency_stop_load_state(repo_root: &Path) -> Value {
    let path = emergency_stop_state_path(repo_root);
    let raw = read_json_or(&path, emergency_stop_state_default());
    let engaged = raw.get("engaged").and_then(Value::as_bool).unwrap_or(false);
    let scopes = if engaged {
        emergency_stop_normalize_scopes(
            raw.get("scopes")
                .and_then(Value::as_array)
                .map(|rows| {
                    rows.iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join(",")
                })
                .as_deref(),
        )
    } else {
        Vec::new()
    };
    json!({
        "engaged": engaged,
        "scopes": scopes,
        "updated_at": raw.get("updated_at").cloned().unwrap_or(Value::Null),
        "reason": raw.get("reason").cloned().unwrap_or(Value::Null),
        "actor": raw.get("actor").cloned().unwrap_or(Value::Null),
        "approval_note": raw.get("approval_note").cloned().unwrap_or(Value::Null)
    })
}

pub fn run_emergency_stop(repo_root: &Path, argv: &[String]) -> (Value, i32) {
    let started = std::time::Instant::now();
    let execution_receipt = |status: &str, stage: &str, error: Option<&str>| {
        json!({
            "status": status,
            "stage": stage,
            "duration_ms": started.elapsed().as_millis(),
            "ts": now_iso(),
            "error": error.map(|v| clean(v, 220))
        })
    };
    let parsed = parse_args(argv);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    let state_path = emergency_stop_state_path(repo_root);

    match cmd.as_str() {
        "status" => (
            json!({
                "ok": true,
                "type": "emergency_stop_status",
                "ts": now_iso(),
                "state": emergency_stop_load_state(repo_root),
                "execution_receipt": execution_receipt("ok", "status", None)
            }),
            0,
        ),
        "engage" => {
            let note = flag(&parsed, "approval-note")
                .or_else(|| flag(&parsed, "approval_note"))
                .map(|v| clean(v, 240))
                .unwrap_or_default();
            if note.len() < 10 {
                return (
                    json!({
                        "ok": false,
                        "type": "emergency_stop_engage",
                        "error": "approval_note_too_short",
                        "min_len": 10,
                        "execution_receipt": execution_receipt("error", "validate_approval_note", Some("approval_note_too_short"))
                    }),
                    2,
                );
            }

            let scopes = emergency_stop_normalize_scopes(flag(&parsed, "scope"));
            let reason = flag(&parsed, "reason")
                .map(|v| clean(v, 240))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "manual_emergency_stop".to_string());
            let actor = flag(&parsed, "actor")
                .map(|v| clean(v, 120))
                .filter(|v| !v.is_empty())
                .or_else(|| std::env::var("USER").ok())
                .unwrap_or_else(|| "unknown".to_string());
            let next = json!({
                "engaged": true,
                "scopes": scopes,
                "updated_at": now_iso(),
                "reason": reason,
                "actor": actor,
                "approval_note": note
            });
            if let Err(err) = write_json_atomic(&state_path, &next) {
                return (
                    json!({
                        "ok": false,
                        "type": "emergency_stop_engage",
                        "error": clean(err, 220),
                        "execution_receipt": execution_receipt("error", "persist_engage_state", Some(err))
                    }),
                    1,
                );
            }
            (
                json!({
                    "ok": true,
                    "type": "emergency_stop_engage",
                    "result": "engaged",
                    "ts": now_iso(),
                    "valid_scopes": emergency_stop_valid_scopes(),
                    "state": next,
                    "execution_receipt": execution_receipt("ok", "engage", None)
                }),
                0,
            )
        }
        "release" => {
            let note = flag(&parsed, "approval-note")
                .or_else(|| flag(&parsed, "approval_note"))
                .map(|v| clean(v, 240))
                .unwrap_or_default();
            if note.len() < 10 {
                return (
                    json!({
                        "ok": false,
                        "type": "emergency_stop_release",
                        "error": "approval_note_too_short",
                        "min_len": 10,
                        "execution_receipt": execution_receipt("error", "validate_approval_note", Some("approval_note_too_short"))
                    }),
                    2,
                );
            }

            let reason = flag(&parsed, "reason")
                .map(|v| clean(v, 240))
                .filter(|v| !v.is_empty())
                .unwrap_or_else(|| "manual_release".to_string());
            let actor = flag(&parsed, "actor")
                .map(|v| clean(v, 120))
                .filter(|v| !v.is_empty())
                .or_else(|| std::env::var("USER").ok())
                .unwrap_or_else(|| "unknown".to_string());
            let next = json!({
                "engaged": false,
                "scopes": [],
                "updated_at": now_iso(),
                "reason": reason,
                "actor": actor,
                "approval_note": note
            });
            if let Err(err) = write_json_atomic(&state_path, &next) {
                return (
                    json!({
                        "ok": false,
                        "type": "emergency_stop_release",
                        "error": clean(err, 220),
                        "execution_receipt": execution_receipt("error", "persist_release_state", Some(err))
                    }),
                    1,
                );
            }
            (
                json!({
                    "ok": true,
                    "type": "emergency_stop_release",
                    "result": "released",
                    "ts": now_iso(),
                    "state": next,
                    "execution_receipt": execution_receipt("ok", "release", None)
                }),
                0,
            )
        }
        _ => (
            json!({
                "ok": false,
                "type": "emergency_stop_error",
                "error": "unknown_command",
                "usage": [
                    "emergency-stop status",
                    "emergency-stop engage --scope=<all|autonomy|routing|actuation|spine[,..]> --approval-note=<text>",
                    "emergency-stop release --approval-note=<text>"
                ],
                "execution_receipt": execution_receipt("error", "dispatch", Some("unknown_command"))
            }),
            2,
        ),
    }
}

pub fn run_integrity_kernel(repo_root: &Path, argv: &[String]) -> (Value, i32) {
    let parsed = parse_args(argv);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "help".to_string());
    let policy = flag(&parsed, "policy").map(ToString::to_string);

    match cmd.as_str() {
        "run" | "status" => {
            let mut args = vec!["check".to_string()];
            if let Some(policy_path) = policy {
                args.push(format!("--policy={policy_path}"));
            }
            let (check, code) = run_integrity_reseal(repo_root, &args);
            (
                json!({
                    "ok": code == 0,
                    "type": "integrity_kernel_status",
                    "ts": now_iso(),
                    "kernel": check
                }),
                code,
            )
        }
        "seal" => {
            let note = flag(&parsed, "approval-note")
                .or_else(|| flag(&parsed, "approval_note"))
                .map(ToString::to_string)
                .unwrap_or_default();
            let mut args = vec![
                "apply".to_string(),
                format!("--approval-note={}", clean(note, 240)),
            ];
            if let Some(policy_path) = policy {
                args.push(format!("--policy={policy_path}"));
            }
            let (apply, code) = run_integrity_reseal(repo_root, &args);
            (
                json!({
                    "ok": code == 0,
                    "type": "integrity_kernel_seal",
                    "ts": now_iso(),
                    "kernel": apply
                }),
                code,
            )
        }
        _ => (
            json!({
                "ok": false,
                "error": "unknown_command",
                "usage": [
                    "integrity-kernel run [--policy=<path>]",
                    "integrity-kernel status [--policy=<path>]",
                    "integrity-kernel seal --approval-note=<text> [--policy=<path>]"
                ]
            }),
            2,
        ),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct LeaseState {
    version: String,
    issued: BTreeMap<String, Value>,
    consumed: BTreeMap<String, Value>,
}

impl Default for LeaseState {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            issued: BTreeMap::new(),
            consumed: BTreeMap::new(),
        }
    }
}

fn lease_state_path(repo_root: &Path) -> PathBuf {
    std::env::var("CAPABILITY_LEASE_STATE_PATH")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            state_root(repo_root)
                .join("security")
                .join("capability_leases.json")
        })
}

fn lease_audit_path(repo_root: &Path) -> PathBuf {
    std::env::var("CAPABILITY_LEASE_AUDIT_PATH")
        .ok()
        .filter(|v| !v.trim().is_empty())
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            state_root(repo_root)
                .join("security")
                .join("capability_leases.jsonl")
        })
}

fn lease_key() -> Option<String> {
    std::env::var("CAPABILITY_LEASE_KEY")
        .ok()
        .map(|v| clean(v, 4096))
        .filter(|v| !v.is_empty())
}

fn load_lease_state(path: &Path) -> LeaseState {
    if !path.exists() {
        return LeaseState::default();
    }
    let raw = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return LeaseState::default(),
    };
    serde_json::from_str::<LeaseState>(&raw).unwrap_or_default()
}

fn save_lease_state(path: &Path, state: &LeaseState) -> Result<(), String> {
    let payload =
        serde_json::to_value(state).map_err(|err| format!("encode_lease_state_failed:{err}"))?;
    write_json_atomic(path, &payload)
}

fn lease_sign(body: &str, key: &str) -> Result<String, String> {
    hmac_sha256_hex(key, body)
}

fn lease_make_id(scope: &str, target: Option<&str>) -> String {
    let seed = format!(
        "{}|{}|{}|{}",
        now_iso(),
        scope,
        target.unwrap_or("none"),
        std::process::id()
    );
    let digest = sha256_hex_bytes(seed.as_bytes());
    format!("lease_{}", &digest[..16])
}

fn lease_pack_token(payload: &Value, key: &str) -> Result<String, String> {
    let body = URL_SAFE_NO_PAD.encode(
        serde_json::to_vec(payload).map_err(|err| format!("encode_lease_payload_failed:{err}"))?,
    );
    let sig = lease_sign(&body, key)?;
    Ok(format!("{body}.{sig}"))
}

fn lease_unpack_token(token: &str) -> Result<(String, String, Value), String> {
    let mut parts = token.trim().split('.');
    let body = parts.next().unwrap_or_default().to_string();
    let sig = parts.next().unwrap_or_default().to_string();
    if body.is_empty() || sig.is_empty() || parts.next().is_some() {
        return Err("token_malformed".to_string());
    }
    let bytes = URL_SAFE_NO_PAD
        .decode(body.as_bytes())
        .map_err(|_| "token_payload_invalid".to_string())?;
    let payload =
        serde_json::from_slice::<Value>(&bytes).map_err(|_| "token_payload_invalid".to_string())?;
    if !payload.is_object() {
        return Err("token_payload_invalid".to_string());
    }
    Ok((body, sig, payload))
}
