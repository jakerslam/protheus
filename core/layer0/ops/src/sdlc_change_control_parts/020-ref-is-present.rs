
fn ref_is_present(raw: &str) -> bool {
    let value = raw.trim();
    !value.is_empty() && !matches!(value.to_ascii_lowercase().as_str(), "n/a" | "none" | "tbd")
}

fn looks_like_url(raw: &str) -> bool {
    let value = raw.trim().to_ascii_lowercase();
    value.starts_with("http://") || value.starts_with("https://")
}

fn ref_exists(root: &Path, raw: &str) -> bool {
    if !ref_is_present(raw) {
        return false;
    }
    if looks_like_url(raw) {
        return true;
    }
    root.join(raw.trim()).exists()
}

fn ensure_parent(path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
}

fn write_text_atomic(path: &Path, text: &str) -> Result<(), String> {
    ensure_parent(path);
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&tmp, text).map_err(|e| format!("write_tmp_failed:{}:{e}", path.display()))?;
    fs::rename(&tmp, path).map_err(|e| format!("rename_tmp_failed:{}:{e}", path.display()))
}

fn append_jsonl(path: &Path, value: &Value) -> Result<(), String> {
    ensure_parent(path);
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("open_jsonl_failed:{}:{e}", path.display()))?;
    let line = serde_json::to_string(value).map_err(|e| format!("encode_jsonl_failed:{e}"))?;
    f.write_all(line.as_bytes())
        .and_then(|_| f.write_all(b"\n"))
        .map_err(|e| format!("append_jsonl_failed:{}:{e}", path.display()))
}

fn load_policy(root: &Path, policy_override: Option<&String>) -> Policy {
    let policy_path = policy_override
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(DEFAULT_POLICY_REL));

    let raw = fs::read_to_string(&policy_path)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .unwrap_or_else(|| json!({}));

    let high_risk_path_prefixes = parse_nonempty_string_array(raw.get("high_risk_path_prefixes"))
        .unwrap_or_else(|| {
            vec![
                "core/layer0/security/".to_string(),
                "core/layer2/conduit/".to_string(),
                "client/runtime/systems/security/".to_string(),
                "client/runtime/config/infring_conduit_policy.json".to_string(),
                "client/runtime/config/rust_source_of_truth_policy.json".to_string(),
            ]
        });

    let major_path_prefixes = parse_nonempty_string_array(raw.get("major_path_prefixes"))
        .unwrap_or_else(|| {
            vec![
                "core/layer0/ops/".to_string(),
                "client/runtime/systems/ops/".to_string(),
                ".github/workflows/".to_string(),
                "client/runtime/config/".to_string(),
            ]
        });

    let outputs = raw.get("outputs").and_then(Value::as_object);

    Policy {
        strict_default: raw
            .get("strict_default")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        high_risk_path_prefixes,
        major_path_prefixes,
        required_approvers_major: raw
            .get("required_approvers_major")
            .and_then(Value::as_u64)
            .unwrap_or(1) as usize,
        required_approvers_high_risk: raw
            .get("required_approvers_high_risk")
            .and_then(Value::as_u64)
            .unwrap_or(2) as usize,
        require_rfc_for_major: raw
            .get("require_rfc_for_major")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        require_adr_for_high_risk: raw
            .get("require_adr_for_high_risk")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        require_rollback_drill_for_high_risk: raw
            .get("require_rollback_drill_for_high_risk")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        require_approval_receipts_for_major: raw
            .get("require_approval_receipts_for_major")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        latest_path: resolve_path(
            root,
            outputs
                .and_then(|o| o.get("latest_path"))
                .and_then(Value::as_str),
            "local/state/ops/sdlc_change_control/latest.json",
        ),
        history_path: resolve_path(
            root,
            outputs
                .and_then(|o| o.get("history_path"))
                .and_then(Value::as_str),
            "local/state/ops/sdlc_change_control/history.jsonl",
        ),
        policy_path,
    }
}
