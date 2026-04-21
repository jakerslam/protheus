
pub fn ethical_reasoning_status(
    root: &Path,
    explicit_policy_path: Option<&Path>,
    explicit_state_dir: Option<&Path>,
) -> Value {
    let policy = load_policy(root, explicit_policy_path);
    let paths = resolve_runtime_paths(root, explicit_state_dir);
    let latest = read_json(&paths.latest_path);
    let priors_raw = read_json(&paths.priors_state_path);

    let priors = priors_raw
        .get("priors")
        .cloned()
        .filter(|v| v.is_object())
        .unwrap_or_else(|| {
            let map: Map<String, Value> = policy
                .value_priors
                .iter()
                .map(|(k, v)| (k.clone(), json!(*v)))
                .collect();
            Value::Object(map)
        });

    json!({
        "ok": true,
        "type": "ethical_reasoning_status",
        "ts": now_iso(),
        "latest": if latest.is_null() { Value::Null } else { latest },
        "priors": priors,
        "paths": {
            "latest_path": paths.latest_path,
            "history_path": paths.history_path,
            "receipts_path": paths.receipts_path
        }
    })
}

#[cfg(test)]
include!("ethical_reasoning_tests.rs");
