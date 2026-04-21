const PROVIDER_INFERENCE_RECEIPTS_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/provider_inference_receipts.jsonl";
const PROVIDER_OUTBOUND_GUARD_RECEIPTS_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/provider_outbound_guard_receipts.jsonl";
const PROVIDER_NETWORK_POLICY_REL: &str = "client/runtime/config/provider_network_policy.json";
const DEFAULT_TELEMETRY_BLOCKLIST: &[&str] = &[
    "segment.io",
    "sentry.io",
    "mixpanel.com",
    "amplitude.com",
    "datadoghq.com",
    "newrelic.com",
];
const DEFAULT_DENY_DOMAINS: &[&str] = &[
    "metadata.google.internal",
    "169.254.169.254",
];

fn provider_network_policy_path(root: &Path) -> PathBuf {
    root.join(PROVIDER_NETWORK_POLICY_REL)
}

fn web_tooling_relaxed_test_mode_env_enabled() -> bool {
    for name in [
        "INFRING_WEB_TOOLING_RELAXED_TEST_MODE",
        "PROTHEUS_WEB_TOOLING_RELAXED_TEST_MODE",
    ] {
        if let Ok(raw) = std::env::var(name) {
            match clean_text(&raw, 40).to_ascii_lowercase().as_str() {
                "1" | "true" | "yes" | "on" => return true,
                "0" | "false" | "no" | "off" => return false,
                _ => {}
            }
        }
    }
    false
}

fn default_provider_network_policy() -> Value {
    json!({
        "type": "infring_provider_network_policy",
        "version": "v1",
        "local_first_default": true,
        "require_explicit_provider_consent": true,
        "relaxed_test_mode": false,
        "telemetry_blocklist_enabled": true,
        "telemetry_blocklist_domains": DEFAULT_TELEMETRY_BLOCKLIST,
        "deny_domains": DEFAULT_DENY_DOMAINS,
        "allow_provider_ids": [],
        "updated_at": crate::now_iso(),
    })
}

fn provider_network_policy(root: &Path) -> Value {
    let path = provider_network_policy_path(root);
    let mut changed = false;
    if !path.exists() {
        write_json_pretty(&path, &default_provider_network_policy());
    }
    let mut policy = read_json(&path).unwrap_or_else(default_provider_network_policy);
    if !policy.is_object() {
        policy = default_provider_network_policy();
        changed = true;
    }
    if policy.get("type").and_then(Value::as_str).unwrap_or("") != "infring_provider_network_policy" {
        policy["type"] = json!("infring_provider_network_policy");
        changed = true;
    }
    if policy.get("version").and_then(Value::as_str).unwrap_or("").is_empty() {
        policy["version"] = json!("v1");
        changed = true;
    }
    if policy.get("local_first_default").and_then(Value::as_bool).is_none() {
        policy["local_first_default"] = json!(true);
        changed = true;
    }
    if policy
        .get("require_explicit_provider_consent")
        .and_then(Value::as_bool)
        .is_none()
    {
        policy["require_explicit_provider_consent"] = json!(true);
        changed = true;
    }
    if policy.get("relaxed_test_mode").and_then(Value::as_bool).is_none() {
        policy["relaxed_test_mode"] = json!(false);
        changed = true;
    }
    if policy
        .get("telemetry_blocklist_enabled")
        .and_then(Value::as_bool)
        .is_none()
    {
        policy["telemetry_blocklist_enabled"] = json!(true);
        changed = true;
    }
    if !policy
        .get("telemetry_blocklist_domains")
        .map(Value::is_array)
        .unwrap_or(false)
    {
        policy["telemetry_blocklist_domains"] = json!(DEFAULT_TELEMETRY_BLOCKLIST);
        changed = true;
    }
    if !policy.get("deny_domains").map(Value::is_array).unwrap_or(false) {
        policy["deny_domains"] = json!(DEFAULT_DENY_DOMAINS);
        changed = true;
    }
    if !policy
        .get("allow_provider_ids")
        .map(Value::is_array)
        .unwrap_or(false)
    {
        policy["allow_provider_ids"] = json!([]);
        changed = true;
    }

    let deny_domains = policy
        .get("deny_domains")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|row| row.as_str().map(|v| clean_text(v, 220)))
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    let sanitized_deny_domains = deny_domains
        .iter()
        .filter(|value| !domain_is_loopback(value))
        .cloned()
        .collect::<Vec<_>>();
    if sanitized_deny_domains != deny_domains {
        policy["deny_domains"] = json!(sanitized_deny_domains);
        changed = true;
    }

    if changed {
        policy["updated_at"] = json!(crate::now_iso());
        write_json_pretty(&path, &policy);
    }
    policy
}

fn provider_inference_receipts_path(root: &Path) -> PathBuf {
    root.join(PROVIDER_INFERENCE_RECEIPTS_REL)
}

fn provider_outbound_guard_receipts_path(root: &Path) -> PathBuf {
    root.join(PROVIDER_OUTBOUND_GUARD_RECEIPTS_REL)
}

fn append_jsonl_row(path: &Path, row: &Value) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(raw) = serde_json::to_string(row) {
        let _ = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .and_then(|mut file| file.write_all(format!("{raw}\n").as_bytes()));
    }
}

fn append_provider_receipt(path: &Path, receipt_type: &str, mut row: Value) {
    if !row.is_object() {
        row = json!({});
    }
    row["type"] = json!(receipt_type);
    row["ts"] = json!(crate::now_iso());
    row["receipt_hash"] = json!(crate::deterministic_receipt_hash(&row));
    append_jsonl_row(path, &row);
}

fn append_provider_inference_receipt(root: &Path, row: Value) {
    append_provider_receipt(
        &provider_inference_receipts_path(root),
        "infring_provider_inference_receipt",
        row,
    );
}

fn append_provider_outbound_guard_receipt(root: &Path, row: Value) {
    append_provider_receipt(
        &provider_outbound_guard_receipts_path(root),
        "infring_provider_outbound_guard_receipt",
        row,
    );
}

fn provider_key_or_error(root: &Path, provider: &str) -> Result<String, String> {
    provider_key(root, provider)
        .ok_or_else(|| "couldn't reach a chat model backend: provider key missing".to_string())
}

fn model_backend_unavailable(value: &Value) -> String {
    format!("model backend unavailable: {}", error_text_from_value(value))
}

fn url_host(raw: &str) -> String {
    let cleaned = clean_text(raw, 500).to_ascii_lowercase();
    let trimmed = cleaned
        .trim_start_matches("https://")
        .trim_start_matches("http://");
    clean_text(
        trimmed
            .split(['/', '?', '#'])
            .next()
            .unwrap_or_default()
            .split('@')
            .next_back()
            .unwrap_or_default()
            .split(':')
            .next()
            .unwrap_or_default()
            .trim_matches('.'),
        220,
    )
    .to_ascii_lowercase()
}

fn host_matches_domain(host: &str, domain: &str) -> bool {
    let host_clean = clean_text(host, 220).to_ascii_lowercase();
    let domain_clean = clean_text(domain, 220).to_ascii_lowercase();
    if host_clean.is_empty() || domain_clean.is_empty() {
        return false;
    }
    host_clean == domain_clean || host_clean.ends_with(&format!(".{domain_clean}"))
}

fn domain_is_loopback(raw: &str) -> bool {
    let domain = clean_text(raw, 220).to_ascii_lowercase();
    domain == "localhost"
        || domain == "0.0.0.0"
        || domain == "::1"
        || domain == "127.0.0.1"
        || domain.starts_with("127.")
}

fn host_is_loopback(raw: &str) -> bool {
    domain_is_loopback(raw)
}
