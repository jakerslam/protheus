fn dashboard_duplicate_runtime_issues(
    listener_pids: &[u32],
    watchdog_pids: &[u32],
    port: u16,
) -> Vec<Value> {
    let mut issues = Vec::<Value>::new();
    if listener_pids.len() > 1 {
        issues.push(json!({
            "code": "dashboard_listener_duplicate_running",
            "message": "multiple dashboard listeners detected on configured port",
            "pids": listener_pids,
            "port": port,
        }));
    }
    if watchdog_pids.len() > 1 {
        issues.push(json!({
            "code": "dashboard_watchdog_duplicate_running",
            "message": "multiple dashboard watchdog processes detected for configured dashboard route",
            "pids": watchdog_pids,
            "port": port,
        }));
    }
    issues
}

fn parse_gateway_launchd_labels(raw: &str) -> Vec<String> {
    let tracked = [
        "ai.infring.gateway",
        "infring.gateway",
        "ai.infring.gateway.legacy",
        "ai.openclaw.gateway",
        "openclaw.gateway",
        "ai.protheus.gateway",
        "protheus.gateway",
    ];
    let mut labels = Vec::<String>::new();
    for line in raw.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || trimmed.starts_with("PID") {
            continue;
        }
        let Some(candidate) = trimmed.split_whitespace().last() else {
            continue;
        };
        if tracked.iter().any(|label| candidate == *label) {
            labels.push(candidate.to_string());
        }
    }
    labels.sort();
    labels.dedup();
    labels
}

#[cfg(target_os = "macos")]
fn loaded_gateway_launchd_labels() -> Vec<String> {
    let output = run_platform_command("launchctl", &[String::from("list")]);
    if output.get("ok").and_then(Value::as_bool) != Some(true) {
        return Vec::new();
    }
    let stdout = output
        .get("stdout")
        .and_then(Value::as_str)
        .unwrap_or_default();
    parse_gateway_launchd_labels(stdout)
}

#[cfg(not(target_os = "macos"))]
fn loaded_gateway_launchd_labels() -> Vec<String> {
    Vec::new()
}

fn binary_file_digest(path: &Path) -> Option<String> {
    let bytes = fs::read(path).ok()?;
    Some(crate::v8_kernel::sha256_hex_bytes(&bytes))
}

fn expected_dashboard_binary_path(root: &Path) -> Option<PathBuf> {
    let explicit = std::env::var("INFRING_DAEMON_EXPECTED_BINARY")
        .ok()
        .map(|raw| raw.trim().to_string())
        .filter(|raw| !raw.is_empty());
    if let Some(raw) = explicit {
        let candidate = PathBuf::from(raw);
        if candidate.is_absolute() {
            return Some(candidate);
        }
        return Some(root.join(candidate));
    }
    let binary_name = if cfg!(windows) {
        "infring-ops.exe"
    } else {
        "infring-ops"
    };
    let mut candidates = Vec::<PathBuf>::new();
    if let Ok(home) = std::env::var("HOME") {
        let trimmed = home.trim();
        if !trimmed.is_empty() {
            candidates.push(
                PathBuf::from(trimmed)
                    .join(".local")
                    .join("bin")
                    .join(binary_name),
            );
        }
    }
    candidates.push(root.join("target").join("debug").join(binary_name));
    candidates.push(root.join("target").join("release").join(binary_name));
    candidates.into_iter().find(|path| path.is_file())
}

fn dashboard_binary_authority_issue(root: &Path) -> Option<Value> {
    let current_exe = std::env::current_exe().ok()?;
    let resolved = resolve_dashboard_executable(&current_exe);
    let current_name = resolved
        .file_name()
        .and_then(|value| value.to_str())
        .unwrap_or_default()
        .to_ascii_lowercase();
    let deprecated_name = current_name.contains("protheus") || current_name.contains("openclaw");
    let expected = expected_dashboard_binary_path(root);
    let mut reasons = Vec::<String>::new();
    if deprecated_name {
        reasons.push("deprecated_binary_name_runtime".to_string());
    }
    let mut expected_digest = Value::Null;
    let mut current_digest = Value::Null;
    if let Some(expected_path) = expected.as_ref() {
        if let Some(digest) = binary_file_digest(&resolved) {
            current_digest = Value::String(digest.clone());
            if let Some(expected_value) = binary_file_digest(expected_path) {
                expected_digest = Value::String(expected_value.clone());
                if digest != expected_value {
                    reasons.push("binary_digest_mismatch_current_vs_expected".to_string());
                }
            } else {
                reasons.push("expected_binary_digest_unavailable".to_string());
            }
        } else {
            reasons.push("current_binary_digest_unavailable".to_string());
        }
    }
    if reasons.is_empty() {
        return None;
    }
    Some(json!({
        "code": "dashboard_runtime_binary_authority_mismatch",
        "message": "dashboard runtime binary did not match canonical infring authority binary",
        "current_executable": resolved.to_string_lossy().to_string(),
        "expected_executable": expected.map(|path| path.to_string_lossy().to_string()),
        "current_digest": current_digest,
        "expected_digest": expected_digest,
        "reasons": reasons,
    }))
}

fn dashboard_runtime_duplicate_guard(root: &Path, cfg: &DashboardLaunchConfig) -> Option<Value> {
    let listener_pids = normalized_running_pids(dashboard_listener_pids(cfg.port));
    let watchdog_pids = dashboard_watchdog_candidate_pids(root, cfg);
    let mut issues = dashboard_duplicate_runtime_issues(&listener_pids, &watchdog_pids, cfg.port);
    let launchd_labels = loaded_gateway_launchd_labels();
    if launchd_labels.len() > 1 {
        issues.push(json!({
            "code": "gateway_launchd_duplicate_labels_loaded",
            "message": "multiple gateway launchd labels are loaded; stale labels must be booted out",
            "labels": launchd_labels,
        }));
    }
    if let Some(binary_issue) = dashboard_binary_authority_issue(root) {
        issues.push(binary_issue);
    }
    if issues.is_empty() {
        return None;
    }
    Some(json!({
        "ok": false,
        "error": "dashboard_duplicate_runtime_detected",
        "issue_count": issues.len(),
        "issues": issues,
    }))
}

fn dashboard_duplicate_restart_payload(root: &Path, cfg: &DashboardLaunchConfig) -> Option<Value> {
    dashboard_runtime_duplicate_guard(root, cfg).map(|duplicate_runtime| {
        json!({
            "ok": false,
            "running": false,
            "launched": false,
            "error": "dashboard_duplicate_runtime_detected",
            "duplicate_runtime": duplicate_runtime,
        })
    })
}

fn dashboard_duplicate_spawn_error(root: &Path, cfg: &DashboardLaunchConfig) -> Option<String> {
    dashboard_runtime_duplicate_guard(root, cfg).map(|duplicate_runtime| {
        let hash = deterministic_receipt_hash(&duplicate_runtime);
        format!("dashboard_duplicate_runtime_detected:receipt_hash={hash}")
    })
}

#[cfg(test)]
mod duplicate_runtime_guard_tests {
    use super::*;

    #[test]
    fn dashboard_duplicate_runtime_issues_empty_when_single_runtime_paths_present() {
        let issues = dashboard_duplicate_runtime_issues(&[1001], &[2001], 4173);
        assert!(issues.is_empty());
    }

    #[test]
    fn dashboard_duplicate_runtime_issues_flags_listener_duplicates() {
        let issues = dashboard_duplicate_runtime_issues(&[1001, 1002], &[2001], 4173);
        assert_eq!(issues.len(), 1);
        assert_eq!(
            issues[0].get("code").and_then(Value::as_str),
            Some("dashboard_listener_duplicate_running")
        );
    }

    #[test]
    fn dashboard_duplicate_runtime_issues_flags_watchdog_duplicates() {
        let issues = dashboard_duplicate_runtime_issues(&[1001], &[2001, 2002], 4173);
        assert_eq!(issues.len(), 1);
        assert_eq!(
            issues[0].get("code").and_then(Value::as_str),
            Some("dashboard_watchdog_duplicate_running")
        );
    }

    #[test]
    fn parse_gateway_launchd_labels_extracts_known_labels() {
        let parsed = parse_gateway_launchd_labels(
            "\
PID\tStatus\tLabel\n\
8999\t0\tai.infring.gateway\n\
-\t0\tai.openclaw.gateway\n\
",
        );
        assert_eq!(
            parsed,
            vec![
                "ai.infring.gateway".to_string(),
                "ai.openclaw.gateway".to_string()
            ]
        );
    }
}
