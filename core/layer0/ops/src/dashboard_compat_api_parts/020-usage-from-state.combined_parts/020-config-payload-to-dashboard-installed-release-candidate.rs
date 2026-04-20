
fn config_payload(root: &Path, snapshot: &Value) -> Value {
    let (provider, model) = effective_app_settings(root, snapshot);
    let llm_ready = crate::dashboard_model_catalog::catalog_payload(root, snapshot)
        .get("models")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .any(|row| row.get("available").and_then(Value::as_bool) == Some(true))
        })
        .unwrap_or(false);
    json!({
        "ok": true,
        "api_key": if llm_ready { "set" } else { "not set" },
        "api_key_set": llm_ready,
        "llm_ready": llm_ready,
        "provider": provider,
        "model": model,
        "cli_mode": "ops",
        "workspace_dir": root.to_string_lossy().to_string(),
        "log_level": clean_text(
            &std::env::var("RUST_LOG")
                .or_else(|_| std::env::var("LOG_LEVEL"))
                .unwrap_or_else(|_| "info".to_string()),
            32,
        )
    })
}

fn config_schema_payload() -> Value {
    json!({
        "ok": true,
        "sections": {
            "runtime": {"root_level": true},
            "llm": {"root_level": false}
        }
    })
}

fn auth_check_payload() -> Value {
    json!({
        "ok": true,
        "mode": "none",
        "authenticated": true,
        "user": "operator"
    })
}

#[derive(Clone)]
struct StatusPayloadCacheEntry {
    key: String,
    built_at_ms: u128,
    payload: Value,
}

fn monotonic_now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0)
}

fn normalize_dashboard_version_text(value: &str) -> String {
    clean_text(value.trim_start_matches(['v', 'V']), 120)
}

fn compare_dashboard_version_text(left: &str, right: &str) -> std::cmp::Ordering {
    let left_normalized = normalize_dashboard_version_text(left);
    let right_normalized = normalize_dashboard_version_text(right);
    match (
        semver::Version::parse(&left_normalized),
        semver::Version::parse(&right_normalized),
    ) {
        (Ok(a), Ok(b)) => a.cmp(&b),
        (Ok(_), Err(_)) => std::cmp::Ordering::Greater,
        (Err(_), Ok(_)) => std::cmp::Ordering::Less,
        _ => left_normalized.cmp(&right_normalized),
    }
}

fn dashboard_version_source_priority(source: &str) -> i32 {
    match clean_text(source, 80).as_str() {
        "git_latest_tag" => 40,
        "install_release_meta" => 30,
        "install_release_tag" => 28,
        "runtime_version_contract" => 20,
        "package_json" => 10,
        _ => 0,
    }
}

fn dashboard_version_candidate(version: &str, tag: &str, source: &str) -> Option<Value> {
    let normalized_version = normalize_dashboard_version_text(version);
    if normalized_version.is_empty() {
        return None;
    }
    let normalized_tag = {
        let cleaned = clean_text(tag, 120);
        if cleaned.is_empty() {
            format!("v{normalized_version}")
        } else {
            cleaned
        }
    };
    Some(json!({
        "version": normalized_version,
        "tag": normalized_tag,
        "source": clean_text(source, 80)
    }))
}

fn pick_dashboard_version_candidate(best: Option<Value>, candidate: Option<Value>) -> Option<Value> {
    let Some(candidate_value) = candidate else {
        return best;
    };
    let candidate_version = candidate_value
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("");
    let candidate_source = candidate_value
        .get("source")
        .and_then(Value::as_str)
        .unwrap_or("");
    match best {
        None => Some(candidate_value),
        Some(best_value) => {
            let best_version = best_value.get("version").and_then(Value::as_str).unwrap_or("");
            let cmp = compare_dashboard_version_text(candidate_version, best_version);
            if cmp == std::cmp::Ordering::Greater {
                Some(candidate_value)
            } else if cmp == std::cmp::Ordering::Less {
                Some(best_value)
            } else {
                let best_source = best_value
                    .get("source")
                    .and_then(Value::as_str)
                    .unwrap_or("");
                if dashboard_version_source_priority(candidate_source)
                    >= dashboard_version_source_priority(best_source)
                {
                    Some(candidate_value)
                } else {
                    Some(best_value)
                }
            }
        }
    }
}

fn dashboard_git_latest_tag_candidate(root: &Path) -> Option<Value> {
    let output = std::process::Command::new("git")
        .args(["tag", "--list", "--sort=-v:refname", "v*"])
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let tag = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(|row| clean_text(row, 120))
        .find(|row| !row.is_empty())?;
    dashboard_version_candidate(&tag, &tag, "git_latest_tag")
}

fn dashboard_installed_release_candidate(root: &Path) -> Option<Value> {
    let meta_path = root
        .join("local")
        .join("state")
        .join("ops")
        .join("install_release_meta.json");
    if let Some(meta) = read_json(&meta_path) {
        let value = clean_text(
            meta.get("release_version_normalized")
                .and_then(Value::as_str)
                .or_else(|| meta.get("release_tag").and_then(Value::as_str))
                .unwrap_or(""),
            120,
        );
        let tag = clean_text(
            meta.get("release_tag")
                .and_then(Value::as_str)
                .unwrap_or(""),
            120,
        );
        let candidate = dashboard_version_candidate(&value, &tag, "install_release_meta");
        if candidate.is_some() {
            return candidate;
        }
    }
    let tag_path = root
        .join("local")
        .join("state")
        .join("ops")
        .join("install_release_tag.txt");
    let raw = std::fs::read_to_string(tag_path).ok()?;
    let tag = clean_text(raw.lines().next().unwrap_or(""), 120);
    dashboard_version_candidate(&tag, &tag, "install_release_tag")
}
