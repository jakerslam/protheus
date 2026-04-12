
#[cfg(feature = "embedded-minimal-core")]
fn embedded_minimal_core_status() -> Value {
    let planes = embedded_minimal_core_planes();
    let lane_entries: Vec<Value> = planes
        .iter()
        .map(|(feature, lane, runner)| {
            json!({
                "feature": feature,
                "lane": lane,
                "runner_ptr": format!("{:p}", *runner as *const ())
            })
        })
        .collect();
    let runner_ptr_fingerprint = deterministic_receipt_hash(&json!(lane_entries));
    let mut out = json!({
        "ok": true,
        "type": "protheusd_embedded_minimal_core_status",
        "ts": now_iso(),
        "embedded_feature": "embedded-minimal-core",
        "planes_embedded": lane_entries,
        "runner_ptr_fingerprint": runner_ptr_fingerprint,
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

#[cfg(feature = "tiny")]
fn tiny_status() -> Value {
    let profile = protheus_tiny_runtime::tiny_profile();
    let capacity = protheus_tiny_runtime::normalized_capacity_score(
        profile.max_heap_kib,
        profile.max_concurrent_hands,
    );
    let mut out = json!({
        "ok": true,
        "type": "protheusd_tiny_status",
        "ts": now_iso(),
        "profile": profile.profile,
        "no_std": profile.no_std,
        "max_heap_kib": profile.max_heap_kib,
        "max_concurrent_hands": profile.max_concurrent_hands,
        "supports_hibernation": profile.supports_hibernation,
        "supports_receipt_batching": profile.supports_receipt_batching,
        "capacity_score": capacity
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

#[cfg(feature = "embedded-max")]
fn tiny_max_status() -> Value {
    let profile = protheus_tiny_runtime::tiny_profile();
    let mut out = json!({
        "ok": true,
        "type": "protheusd_tiny_max_status",
        "ts": now_iso(),
        "mode": "embedded-max",
        "no_std_runtime": profile.no_std,
        "allocator_profile": "minimal-alloc",
        "pgo_profile_enabled": cfg!(feature = "pgo-profile"),
        "max_heap_kib": profile.max_heap_kib,
        "max_concurrent_hands": profile.max_concurrent_hands
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum InstallerCompatCommand {
    DaemonControl,
    DashboardUi,
}

fn installer_compat_command(command: &str) -> Option<InstallerCompatCommand> {
    match command {
        "daemon-control" => Some(InstallerCompatCommand::DaemonControl),
        "dashboard-ui" => Some(InstallerCompatCommand::DashboardUi),
        _ => None,
    }
}

fn main() {
    configure_low_memory_allocator_env();
    #[cfg(feature = "embedded-max")]
    std::env::set_var("PROTHEUS_EMBEDDED_MAX", "1");
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let args = parse_os_args(env::args_os().skip(1));
    let command = args
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return;
    }

    if let Some(compat) = installer_compat_command(command.as_str()) {
        let exit = match compat {
            InstallerCompatCommand::DaemonControl => daemon_control::run(&cwd, &args[1..]),
            InstallerCompatCommand::DashboardUi => {
                protheus_ops_core::dashboard_ui::run(&cwd, &args[1..])
            }
        };
        std::process::exit(exit);
    }

    match command.as_str() {
        "status" | "start" | "stop" | "restart" | "attach" | "subscribe" | "tick"
        | "diagnostics" => {
            let exit = daemon_control::run(&cwd, &args);
            std::process::exit(exit);
        }
        "think" => {
            let exit = run_think(&cwd, &args[1..]);
            std::process::exit(exit);
        }
        "research" => {
            let exit = run_research(&cwd, &args[1..]);
            std::process::exit(exit);
        }
        "memory" => {
            let exit = run_memory(&cwd, &args[1..]);
            std::process::exit(exit);
        }
        "orchestration" => {
            let exit = run_orchestration(&cwd, &args[1..]);
            std::process::exit(exit);
        }
        "swarm-runtime" | "swarm" => {
            let exit = run_swarm(&cwd, &args[1..]);
            std::process::exit(exit);
        }
        "capability-profile" => {
            print_json(&capability_profile_payload(&args[1..]));
            std::process::exit(0);
        }
        "efficiency-status" => {
            let parsed = protheus_ops_core::parse_args(&[]);
            let out = status_runtime_efficiency_floor(&cwd, &parsed).json;
            print_json(&out);
            std::process::exit(0);
        }
        #[cfg(feature = "embedded-minimal-core")]
        "embedded-core-status" => {
            print_json(&embedded_minimal_core_status());
            std::process::exit(0);
        }
        #[cfg(feature = "tiny")]
        "tiny-status" => {
            print_json(&tiny_status());
            std::process::exit(0);
        }
        #[cfg(feature = "embedded-max")]
        "tiny-max-status" => {
            print_json(&tiny_max_status());
            std::process::exit(0);
        }
        _ => {
            usage();
            print_json(&cli_error("unknown_command", command.as_str()));
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    use serde_json::Value;

    #[test]
    fn installer_compat_aliases_are_recognized() {
        assert_eq!(
            installer_compat_command("daemon-control"),
            Some(InstallerCompatCommand::DaemonControl)
        );
        assert_eq!(
            installer_compat_command("dashboard-ui"),
            Some(InstallerCompatCommand::DashboardUi)
        );
        assert_eq!(installer_compat_command("status"), None);
    }

    #[test]
    fn memory_write_and_query_roundtrip() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path();
        let payload = memory_write_payload(
            root,
            &[
                "--text=remember pure intelligence".to_string(),
                "--session-id=test".to_string(),
                "--tags=intel,pure".to_string(),
            ],
        )
        .expect("write memory");
        assert_eq!(
            payload.get("type").and_then(Value::as_str),
            Some("pure_memory_write")
        );

        let query = memory_query_payload(
            root,
            &[
                "--q=intelligence".to_string(),
                "--session-id=test".to_string(),
                "--limit=5".to_string(),
            ],
        );
        assert_eq!(
            query.get("type").and_then(Value::as_str),
            Some("pure_memory_query")
        );
        assert!(query
            .get("matches")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn think_uses_session_memory_hits() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path();
        memory_write_payload(
            root,
            &[
                "--text=research rust safety constraints".to_string(),
                "--session-id=alpha".to_string(),
            ],
        )
        .expect("seed memory");
        let thought = think_payload(
            root,
            &[
                "--prompt=Can you research safety constraints?".to_string(),
                "--session-id=alpha".to_string(),
            ],
        )
        .expect("think");
        assert_eq!(
            thought.get("type").and_then(Value::as_str),
            Some("pure_think")
        );
        assert!(thought
            .get("memory_hits")
            .and_then(Value::as_array)
            .map(|rows| !rows.is_empty())
            .unwrap_or(false));
        assert_eq!(
            thought
                .get("effective_memory_limit")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            5
        );
    }

    #[test]
    fn mcu_profile_sheds_heavy_paths() {
        let profile = runtime_capability_profile(&[
            "--hardware-class=mcu".to_string(),
            "--tiny-max=1".to_string(),
            "--memory-mb=256".to_string(),
            "--cpu-cores=1".to_string(),
        ]);
        assert_eq!(
            profile.hardware_class,
            RuntimeHardwareClass::Microcontroller
        );
        assert!(!profile.allow_research_fetch);
        assert!(!profile.allow_persistent_swarm);
        assert_eq!(profile.max_swarm_depth, 1);
        assert_eq!(profile.max_memory_hits, 2);
    }

    #[test]
    fn think_clamps_memory_limit_on_mcu_profile() {
        let tmp = tempdir().expect("tempdir");
        let root = tmp.path();
        for idx in 0..6 {
            memory_write_payload(
                root,
                &[
                    format!("--text=research note {idx}"),
                    "--session-id=alpha".to_string(),
                ],
            )
            .expect("seed memory");
        }
        let thought = think_payload(
            root,
            &[
                "--prompt=research note".to_string(),
                "--session-id=alpha".to_string(),
                "--memory-limit=20".to_string(),
                "--hardware-class=mcu".to_string(),
                "--tiny-max=1".to_string(),
            ],
        )
        .expect("think");
        assert_eq!(
            thought
                .get("effective_memory_limit")
                .and_then(Value::as_u64)
                .unwrap_or(0),
            2
        );
        let hits = thought
            .get("memory_hits")
            .and_then(Value::as_array)
            .expect("memory hits");
        assert!(hits.len() <= 2);
    }

    #[test]
    fn mcu_profile_blocks_heavy_orchestration_ops() {
        let profile = runtime_capability_profile(&["--hardware-class=mcu".to_string()]);
        let err = validate_orchestration_profile(
            &profile,
            &[
                "invoke".to_string(),
                "--op=coordinator.run".to_string(),
                "--payload-json={}".to_string(),
            ],
        )
        .expect_err("should block heavy op");
        assert!(err.contains("hardware_profile_blocks_orchestration_op"));
    }

    #[test]
    fn mcu_profile_limits_swarm_depth() {
        let profile = runtime_capability_profile(&["--hardware-class=mcu".to_string()]);
        let err = validate_swarm_profile(
            &profile,
            &[
                "spawn".to_string(),
                "--task=test".to_string(),
                "--max-depth=3".to_string(),
            ],
        )
        .expect_err("should enforce max depth");
        assert!(err.contains("hardware_profile_max_swarm_depth_exceeded"));
    }

    #[cfg(feature = "tiny")]
    #[test]
    fn tiny_status_emits_receipt_and_profile() {
        let payload = tiny_status();
        assert_eq!(
            payload.get("type").and_then(Value::as_str),
            Some("protheusd_tiny_status")
        );
        assert_eq!(payload.get("no_std").and_then(Value::as_bool), Some(true));
        assert!(payload
            .get("receipt_hash")
            .and_then(Value::as_str)
            .map(|value| !value.is_empty())
            .unwrap_or(false));
    }
}
