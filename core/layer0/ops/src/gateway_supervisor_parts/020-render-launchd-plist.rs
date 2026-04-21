
#[cfg(target_os = "macos")]
fn render_launchd_plist(
    root: &Path,
    log_path: &Path,
    label: &str,
    watchdog_args: &[String],
) -> String {
    let launchd_home = home_dir().unwrap_or_else(|| root.to_path_buf());
    let launchd_home_text = launchd_home.to_string_lossy().to_string();
    let launchd_path = launchd_env_path();
    let watchdog_bin = watchdog_args.first().cloned().unwrap_or_default();
    let mut args_xml = String::new();
    for arg in watchdog_args {
        args_xml.push_str(&format!(
            "    <string>{}</string>\n",
            xml_escape(arg.as_str())
        ));
    }
    format!(
        "<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n\
<!DOCTYPE plist PUBLIC \"-//Apple//DTD PLIST 1.0//EN\" \"http://www.apple.com/DTDs/PropertyList-1.0.dtd\">\n\
<plist version=\"1.0\">\n\
<dict>\n\
  <key>Label</key>\n\
  <string>{label}</string>\n\
  <key>ProgramArguments</key>\n\
  <array>\n\
{args_xml}  </array>\n\
  <key>WorkingDirectory</key>\n\
  <string>{working_dir}</string>\n\
  <key>EnvironmentVariables</key>\n\
  <dict>\n\
    <key>HOME</key>\n\
    <string>{env_home}</string>\n\
    <key>PATH</key>\n\
    <string>{env_path}</string>\n\
    <key>PROTHEUS_OPS_ALLOW_STALE</key>\n\
    <string>1</string>\n\
    <key>PROTHEUS_NPM_ALLOW_STALE</key>\n\
    <string>1</string>\n\
    <key>PROTHEUS_NPM_BINARY</key>\n\
    <string>{env_binary}</string>\n\
  </dict>\n\
  <key>KeepAlive</key>\n\
  <true/>\n\
  <key>RunAtLoad</key>\n\
  <true/>\n\
  <key>ThrottleInterval</key>\n\
  <integer>2</integer>\n\
  <key>StandardOutPath</key>\n\
  <string>{log_file}</string>\n\
  <key>StandardErrorPath</key>\n\
  <string>{log_file}</string>\n\
</dict>\n\
</plist>\n",
        label = xml_escape(label),
        args_xml = args_xml,
        working_dir = xml_escape(root.to_string_lossy().as_ref()),
        env_home = xml_escape(launchd_home_text.as_str()),
        env_path = xml_escape(launchd_path.as_str()),
        env_binary = xml_escape(watchdog_bin.as_str()),
        log_file = xml_escape(log_path.to_string_lossy().as_ref())
    )
}

#[cfg(target_os = "macos")]
fn launchctl_state(stdout: &str) -> Option<String> {
    stdout.lines().find_map(|line| {
        let trimmed = line.trim();
        let (_, value) = trimmed.split_once("state =")?;
        let state = value.trim();
        if state.is_empty() {
            None
        } else {
            Some(state.to_string())
        }
    })
}

#[cfg(target_os = "macos")]
fn launchctl_status(uid: &str, label: &str, plist_path: &Path) -> GatewaySupervisorResult {
    let target = format!("gui/{uid}/{label}");
    let print = launchctl(&[String::from("print"), target.clone()]);
    let stdout = print
        .get("stdout")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let active = command_ok(&print);
    let service_state = launchctl_state(stdout);
    let running = active && service_state.as_deref() == Some("running");
    GatewaySupervisorResult {
        active,
        payload: json!({
            "ok": true,
            "platform": "launchd",
            "label": label,
            "service_target": target,
            "service_file": plist_path.to_string_lossy().to_string(),
            "installed": plist_path.exists(),
            "active": active,
            "running": running,
            "service_state": service_state,
            "status_probe": print,
        }),
    }
}

#[cfg(target_os = "macos")]
fn launchd_enable(
    root: &Path,
    executable: &Path,
    cfg: &GatewaySupervisorConfig,
    log_path: &Path,
) -> GatewaySupervisorResult {
    let Some((uid, label, plist_path)) = launchd_paths() else {
        return unsupported_payload("enable", "launchd_identity_unavailable");
    };
    if let Some(parent) = plist_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    let args = watchdog_args(executable, cfg);
    let codesign_refresh = refresh_codesign_signature(executable);
    let plist = render_launchd_plist(root, log_path, label.as_str(), &args);
    if let Err(err) = fs::write(&plist_path, plist) {
        return GatewaySupervisorResult {
            active: false,
            payload: json!({
                "ok": false,
                "platform": "launchd",
                "action": "enable",
                "error": format!("launchd_plist_write_failed:{err}"),
                "service_file": plist_path.to_string_lossy().to_string(),
                "codesign_refresh": codesign_refresh,
            }),
        };
    }

    let target = format!("gui/{uid}/{label}");
    let domain = format!("gui/{uid}");
    let plist_str = plist_path.to_string_lossy().to_string();
    let bootout_target = launchctl(&[String::from("bootout"), target.clone()]);
    let bootout_path = launchctl(&[String::from("bootout"), domain.clone(), plist_str.clone()]);
    let bootstrap = launchctl(&[String::from("bootstrap"), domain.clone(), plist_str.clone()]);
    let fallback_load = if !command_ok(&bootstrap) {
        Some(launchctl(&[
            String::from("load"),
            String::from("-w"),
            plist_str.clone(),
        ]))
    } else {
        None
    };
    let enable = launchctl(&[String::from("enable"), target.clone()]);
    let kickstart = launchctl(&[
        String::from("kickstart"),
        String::from("-k"),
        target.clone(),
    ]);

    let mut status = launchctl_status(uid.as_str(), label.as_str(), &plist_path);
    let enabled = status.active;
    if let Some(obj) = status.payload.as_object_mut() {
        obj.insert("action".to_string(), Value::String("enable".to_string()));
        obj.insert("bootout_target".to_string(), bootout_target);
        obj.insert("bootout_path".to_string(), bootout_path);
        obj.insert("bootstrap".to_string(), bootstrap);
        if let Some(load) = fallback_load {
            obj.insert("fallback_load".to_string(), load);
        }
        obj.insert("codesign_refresh".to_string(), codesign_refresh);
        obj.insert("enable_cmd".to_string(), enable);
        obj.insert("kickstart".to_string(), kickstart);
        obj.insert("watchdog_args".to_string(), json!(args));
    }
    status.active = enabled;
    status
}

#[cfg(target_os = "macos")]
fn launchd_disable(_root: &Path) -> GatewaySupervisorResult {
    let Some((uid, label, plist_path)) = launchd_paths() else {
        return unsupported_payload("disable", "launchd_identity_unavailable");
    };
    let target = format!("gui/{uid}/{label}");
    let domain = format!("gui/{uid}");
    let plist_str = plist_path.to_string_lossy().to_string();
    let bootout_target = launchctl(&[String::from("bootout"), target.clone()]);
    let bootout_path = launchctl(&[String::from("bootout"), domain.clone(), plist_str.clone()]);
    let unload = launchctl(&[
        String::from("unload"),
        String::from("-w"),
        plist_str.clone(),
    ]);
    let removed = fs::remove_file(&plist_path).is_ok();
    GatewaySupervisorResult {
        active: false,
        payload: json!({
            "ok": true,
            "platform": "launchd",
            "action": "disable",
            "label": label,
            "service_target": target,
            "service_file": plist_path.to_string_lossy().to_string(),
            "removed_service_file": removed,
            "bootout_target": bootout_target,
            "bootout_path": bootout_path,
            "unload": unload,
        }),
    }
}

#[cfg(target_os = "macos")]
fn launchd_status(root: &Path) -> GatewaySupervisorResult {
    let Some((uid, label, plist_path)) = launchd_paths() else {
        return unsupported_payload("status", "launchd_identity_unavailable");
    };
    let mut status = launchctl_status(uid.as_str(), label.as_str(), &plist_path);
    if let Some(obj) = status.payload.as_object_mut() {
        obj.insert("action".to_string(), Value::String("status".to_string()));
        obj.insert(
            "root".to_string(),
            Value::String(root.to_string_lossy().to_string()),
        );
    }
    status
}

#[cfg(target_os = "linux")]
fn systemd_paths() -> Option<(String, PathBuf)> {
    let unit = systemd_unit_name();
    let service_path = home_dir()?
        .join(".config")
        .join("systemd")
        .join("user")
        .join(unit.as_str());
    Some((unit, service_path))
}
