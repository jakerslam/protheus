
#[cfg(target_os = "linux")]
fn render_systemd_service(root: &Path, cfg: &GatewaySupervisorConfig, executable: &Path) -> String {
    let args = watchdog_args(executable, cfg)
        .into_iter()
        .map(|arg| shell_quote(arg.as_str()))
        .collect::<Vec<_>>()
        .join(" ");
    let command = format!("{args}");
    format!(
        "[Unit]\n\
Description=Infring Gateway Watchdog\n\
After=network.target\n\
\n\
[Service]\n\
Type=simple\n\
WorkingDirectory={working_dir}\n\
ExecStart=/bin/sh -lc {exec_start}\n\
Restart=always\n\
RestartSec=1\n\
KillMode=process\n\
Environment=PROTHEUS_ROOT={root_env}\n\
\n\
[Install]\n\
WantedBy=default.target\n",
        working_dir = root.to_string_lossy(),
        exec_start = shell_quote(command.as_str()),
        root_env = root.to_string_lossy(),
    )
}

#[cfg(target_os = "linux")]
fn systemd_status(root: &Path) -> GatewaySupervisorResult {
    let Some((unit, service_path)) = systemd_paths() else {
        return unsupported_payload("status", "systemd_identity_unavailable");
    };
    let active_cmd = systemctl_user(&[
        String::from("--user"),
        String::from("is-active"),
        unit.clone(),
    ]);
    let enabled_cmd = systemctl_user(&[
        String::from("--user"),
        String::from("is-enabled"),
        unit.clone(),
    ]);
    let active = command_ok(&active_cmd)
        && active_cmd
            .get("stdout")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .contains("active");
    GatewaySupervisorResult {
        active,
        payload: json!({
            "ok": true,
            "platform": "systemd-user",
            "action": "status",
            "unit": unit,
            "service_file": service_path.to_string_lossy().to_string(),
            "installed": service_path.exists(),
            "active": active,
            "enabled": command_ok(&enabled_cmd),
            "active_cmd": active_cmd,
            "enabled_cmd": enabled_cmd,
            "root": root.to_string_lossy().to_string(),
        }),
    }
}

#[cfg(target_os = "linux")]
fn systemd_enable(
    root: &Path,
    executable: &Path,
    cfg: &GatewaySupervisorConfig,
) -> GatewaySupervisorResult {
    let Some((unit, service_path)) = systemd_paths() else {
        return unsupported_payload("enable", "systemd_identity_unavailable");
    };
    if let Some(parent) = service_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let service = render_systemd_service(root, cfg, executable);
    if let Err(err) = fs::write(&service_path, service) {
        return GatewaySupervisorResult {
            active: false,
            payload: json!({
                "ok": false,
                "platform": "systemd-user",
                "action": "enable",
                "error": format!("systemd_service_write_failed:{err}"),
                "service_file": service_path.to_string_lossy().to_string(),
            }),
        };
    }

    let daemon_reload = systemctl_user(&[String::from("--user"), String::from("daemon-reload")]);
    let enable_now = systemctl_user(&[
        String::from("--user"),
        String::from("enable"),
        String::from("--now"),
        unit.clone(),
    ]);
    let restart = systemctl_user(&[
        String::from("--user"),
        String::from("restart"),
        unit.clone(),
    ]);

    let mut status = systemd_status(root);
    if let Some(obj) = status.payload.as_object_mut() {
        obj.insert("action".to_string(), Value::String("enable".to_string()));
        obj.insert("daemon_reload".to_string(), daemon_reload);
        obj.insert("enable_now".to_string(), enable_now);
        obj.insert("restart".to_string(), restart);
    }
    status
}

#[cfg(target_os = "linux")]
fn systemd_disable(root: &Path) -> GatewaySupervisorResult {
    let Some((unit, service_path)) = systemd_paths() else {
        return unsupported_payload("disable", "systemd_identity_unavailable");
    };
    let disable_now = systemctl_user(&[
        String::from("--user"),
        String::from("disable"),
        String::from("--now"),
        unit.clone(),
    ]);
    let removed = fs::remove_file(&service_path).is_ok();
    let daemon_reload = systemctl_user(&[String::from("--user"), String::from("daemon-reload")]);
    GatewaySupervisorResult {
        active: false,
        payload: json!({
            "ok": true,
            "platform": "systemd-user",
            "action": "disable",
            "unit": unit,
            "service_file": service_path.to_string_lossy().to_string(),
            "removed_service_file": removed,
            "disable_now": disable_now,
            "daemon_reload": daemon_reload,
            "root": root.to_string_lossy().to_string(),
        }),
    }
}

pub fn enable(
    _root: &Path,
    _executable: &Path,
    _cfg: &GatewaySupervisorConfig,
    _log_path: &Path,
) -> GatewaySupervisorResult {
    #[cfg(target_os = "macos")]
    {
        return launchd_enable(_root, _executable, _cfg, _log_path);
    }
    #[cfg(target_os = "linux")]
    {
        return systemd_enable(_root, _executable, _cfg);
    }
    #[allow(unreachable_code)]
    unsupported_payload("enable", "platform_not_supported")
}

pub fn disable(_root: &Path) -> GatewaySupervisorResult {
    #[cfg(target_os = "macos")]
    {
        return launchd_disable(_root);
    }
    #[cfg(target_os = "linux")]
    {
        return systemd_disable(_root);
    }
    #[allow(unreachable_code)]
    unsupported_payload("disable", "platform_not_supported")
}

pub fn status(_root: &Path) -> GatewaySupervisorResult {
    #[cfg(target_os = "macos")]
    {
        return launchd_status(_root);
    }
    #[cfg(target_os = "linux")]
    {
        return systemd_status(_root);
    }
    #[allow(unreachable_code)]
    unsupported_payload("status", "platform_not_supported")
}
