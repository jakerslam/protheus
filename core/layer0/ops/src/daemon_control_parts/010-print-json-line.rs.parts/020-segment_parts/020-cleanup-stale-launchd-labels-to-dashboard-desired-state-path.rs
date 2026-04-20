
#[cfg(target_os = "macos")]
fn cleanup_stale_launchd_labels() -> Value {
    let current_label = std::env::var("INFRING_GATEWAY_LAUNCHD_LABEL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "ai.infring.gateway".to_string());
    let Some(uid) = launchd_uid_for_cleanup() else {
        return json!({
            "ok": false,
            "active": true,
            "error": "launchd_uid_unavailable",
            "current_label": current_label,
        });
    };
    let home = std::env::var("HOME").unwrap_or_default();
    let domain = format!("gui/{uid}");
    let mut rows = Vec::<Value>::new();
    for label in stale_launchd_labels_for_cleanup(&current_label) {
        let target = format!("{domain}/{label}");
        let plist_path = PathBuf::from(&home)
            .join("Library")
            .join("LaunchAgents")
            .join(format!("{label}.plist"));
        let plist_text = plist_path.to_string_lossy().to_string();
        let bootout_target =
            run_platform_command("launchctl", &[String::from("bootout"), target.clone()]);
        let bootout_path = run_platform_command(
            "launchctl",
            &[
                String::from("bootout"),
                domain.clone(),
                plist_text.clone(),
            ],
        );
        let unload = run_platform_command(
            "launchctl",
            &[
                String::from("unload"),
                String::from("-w"),
                plist_text.clone(),
            ],
        );
        let removed_service_file = if plist_path.exists() {
            fs::remove_file(&plist_path).is_ok()
        } else {
            false
        };
        rows.push(json!({
            "label": label,
            "service_target": target,
            "service_file": plist_text,
            "bootout_target": bootout_target,
            "bootout_path": bootout_path,
            "unload": unload,
            "removed_service_file": removed_service_file
        }));
    }
    json!({
        "ok": true,
        "active": true,
        "current_label": current_label,
        "rows": rows
    })
}

#[cfg(not(target_os = "macos"))]
fn cleanup_stale_launchd_labels() -> Value {
    json!({
        "ok": true,
        "active": false,
        "reason": "platform_not_macos",
        "rows": []
    })
}

fn resolved_infring_home(root: &Path) -> String {
    std::env::var("INFRING_HOME")
        .ok()
        .or_else(|| std::env::var("PROTHEUS_HOME").ok())
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| root.to_string_lossy().to_string())
}

fn verify_gateway_service_root(result_payload: &Value, expected_root: &str) -> Value {
    let service_file = result_payload
        .get("service_file")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    if service_file.is_empty() {
        return json!({
            "checked": false,
            "ok": true,
            "reason": "service_file_unavailable"
        });
    }
    let content = fs::read_to_string(&service_file).unwrap_or_default();
    let expected_unix = expected_root.replace('\\', "/");
    let content_unix = content.replace('\\', "/");
    let matches = content.contains(expected_root) || content_unix.contains(&expected_unix);
    json!({
        "checked": true,
        "ok": matches,
        "service_file": service_file,
        "expected_infring_home": expected_root,
    })
}

fn gateway_supervisor_enable(root: &Path, cfg: &DashboardLaunchConfig) -> GatewaySupervisorResult {
    let executable = match supervisor_executable() {
        Ok(path) => path,
        Err(err) => {
            return GatewaySupervisorResult {
                active: false,
                payload: json!({
                    "ok": false,
                    "action": "enable",
                    "error": err,
                }),
            };
        }
    };
    let pre_start_cleanup = json!({
        "stale_launchd_cleanup": cleanup_stale_launchd_labels(),
        "stale_pid_cleanup": cleanup_dashboard_pid_files(root, cfg)
    });
    let supervisor_cfg = gateway_supervisor_config(cfg);
    let mut result = gateway_supervisor::enable(
        root,
        &executable,
        &supervisor_cfg,
        &dashboard_watchdog_log_path(root),
    );
    let expected_home = resolved_infring_home(root);
    let root_contract = verify_gateway_service_root(&result.payload, expected_home.as_str());
    if let Some(obj) = result.payload.as_object_mut() {
        obj.insert("pre_start_cleanup".to_string(), pre_start_cleanup);
        obj.insert("service_root_contract".to_string(), root_contract.clone());
    }
    if root_contract.get("checked").and_then(Value::as_bool) == Some(true)
        && root_contract.get("ok").and_then(Value::as_bool) == Some(false)
    {
        result.active = false;
        if let Some(obj) = result.payload.as_object_mut() {
            obj.insert("ok".to_string(), Value::Bool(false));
            obj.insert(
                "error".to_string(),
                Value::String("service_root_mismatch".to_string()),
            );
            obj.insert(
                "expected_infring_home".to_string(),
                Value::String(expected_home),
            );
        }
    }
    result
}

fn dashboard_state_dir(root: &Path) -> std::path::PathBuf {
    root.join("local")
        .join("state")
        .join("ops")
        .join("daemon_control")
}

fn dashboard_pid_path(root: &Path) -> std::path::PathBuf {
    dashboard_state_dir(root).join("dashboard_ui.pid")
}

fn dashboard_log_path(root: &Path) -> std::path::PathBuf {
    dashboard_state_dir(root).join("dashboard_ui.log")
}

fn dashboard_watchdog_pid_path(root: &Path) -> std::path::PathBuf {
    dashboard_state_dir(root).join("dashboard_watchdog.pid")
}

fn dashboard_watchdog_log_path(root: &Path) -> std::path::PathBuf {
    dashboard_state_dir(root).join("dashboard_watchdog.log")
}

fn dashboard_stop_latch_path(root: &Path) -> std::path::PathBuf {
    dashboard_state_dir(root).join("dashboard.stop")
}

fn dashboard_desired_state_path(root: &Path) -> std::path::PathBuf {
    dashboard_state_dir(root).join("dashboard.desired")
}
