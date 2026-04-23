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

fn dashboard_runtime_duplicate_guard(root: &Path, cfg: &DashboardLaunchConfig) -> Option<Value> {
    let listener_pids = normalized_running_pids(dashboard_listener_pids(cfg.port));
    let watchdog_pids = dashboard_watchdog_candidate_pids(root, cfg);
    let issues = dashboard_duplicate_runtime_issues(&listener_pids, &watchdog_pids, cfg.port);
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
}
