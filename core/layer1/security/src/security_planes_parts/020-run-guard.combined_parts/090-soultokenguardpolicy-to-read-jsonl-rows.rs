
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
struct SoulTokenGuardPolicy {
    version: String,
    enabled: bool,
    enforcement_mode: String,
    bind_to_fingerprint: bool,
    default_attestation_valid_hours: i64,
    key_env: String,
    token_state_path: String,
    audit_path: String,
    attestation_path: String,
}

impl Default for SoulTokenGuardPolicy {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            enabled: true,
            enforcement_mode: "advisory".to_string(),
            bind_to_fingerprint: true,
            default_attestation_valid_hours: 24 * 7,
            key_env: "SOUL_TOKEN_GUARD_KEY".to_string(),
            token_state_path: "local/state/security/soul_token_guard.json".to_string(),
            audit_path: "local/state/security/soul_token_guard_audit.jsonl".to_string(),
            attestation_path: "local/state/security/release_attestations.jsonl".to_string(),
        }
    }
}

fn load_soul_token_policy(repo_root: &Path, parsed: &ParsedArgs) -> SoulTokenGuardPolicy {
    let policy_path = flag(parsed, "policy")
        .map(|v| resolve_runtime_or_state(repo_root, v))
        .unwrap_or_else(|| runtime_config_path(repo_root, "soul_token_guard_policy.json"));
    if !policy_path.exists() {
        return SoulTokenGuardPolicy::default();
    }
    match fs::read_to_string(&policy_path) {
        Ok(raw) => serde_json::from_str::<SoulTokenGuardPolicy>(&raw).unwrap_or_default(),
        Err(_) => SoulTokenGuardPolicy::default(),
    }
}

fn soul_fingerprint(repo_root: &Path) -> String {
    let hostname = std::env::var("HOSTNAME").unwrap_or_else(|_| "unknown-host".to_string());
    let seed = format!(
        "{}|{}|{}|{}",
        hostname,
        std::env::consts::OS,
        std::env::consts::ARCH,
        repo_root.display()
    );
    format!("fp_{}", &sha256_hex_bytes(seed.as_bytes())[0..16])
}

fn read_jsonl_rows(path: &Path) -> Vec<Value> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    raw.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect::<Vec<_>>()
}
