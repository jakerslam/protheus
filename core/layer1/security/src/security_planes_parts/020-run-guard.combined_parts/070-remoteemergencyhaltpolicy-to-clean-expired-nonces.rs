
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
