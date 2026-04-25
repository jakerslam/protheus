#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    fn assert_non_silent_outcome(payload: &Value, expected_type: &str) {
        assert_eq!(
            payload.get("type").and_then(Value::as_str),
            Some(expected_type)
        );
        assert!(payload.get("ok").and_then(Value::as_bool).is_some());
        assert!(
            payload
                .get("receipt_hash")
                .and_then(Value::as_str)
                .is_some()
                || payload
                    .get("claim_evidence")
                    .and_then(Value::as_array)
                    .is_some()
                || payload.get("error").is_some()
                || payload.get("reason").is_some()
        );
    }

    fn payload_for(command: &str) -> Value {
        success_receipt(
            command,
            Some("persistent"),
            &[command.to_string(), "--mode=persistent".to_string()],
            Path::new("."),
        )
    }

    #[test]
    fn daemon_control_supports_attach_subscribe_and_diagnostics() {
        for command in ["attach", "subscribe", "diagnostics"] {
            let payload = payload_for(command);
            assert_non_silent_outcome(&payload, "daemon_control_receipt");
            assert_eq!(
                payload.get("command").and_then(Value::as_str),
                Some(command),
                "command should round-trip in receipt"
            );
            assert!(
                payload
                    .get("receipt_hash")
                    .and_then(Value::as_str)
                    .map(|value| !value.trim().is_empty())
                    .unwrap_or(false),
                "receipt hash should be present"
            );
        }
    }

    #[test]
    fn unknown_command_returns_error_exit_code() {
        let root = Path::new(".");
        let exit = run(root, &[String::from("not-a-command")]);
        assert_eq!(exit, 2);
    }

    #[test]
    fn drift_status_receipt_exposes_mode_and_tolerances() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        let payload = verity_drift_status_receipt(root, &["drift-status".to_string()]);
        assert_non_silent_outcome(&payload, "verity_drift_status");
        assert_eq!(
            payload.get("type").and_then(Value::as_str),
            Some("verity_drift_status")
        );
        assert_eq!(
            payload.get("mode").and_then(Value::as_str),
            Some("production")
        );
        assert_eq!(
            payload.get("active_tolerance_ms").and_then(Value::as_i64),
            Some(500)
        );
    }

    #[test]
    fn dashboard_launch_config_defaults_to_autoboot_for_start() {
        let cfg = parse_dashboard_launch_config(&[], "start");
        assert!(cfg.enabled);
        assert!(cfg.open_browser);
        assert!(cfg.persistent_supervisor);
        assert!(!cfg.node_binary.trim().is_empty());
        assert_eq!(cfg.host, "127.0.0.1");
        assert_eq!(cfg.port, 4173);
        assert_eq!(cfg.ready_timeout_ms, 36_000);
        assert_eq!(
            cfg.watchdog_interval_ms,
            DASHBOARD_WATCHDOG_INTERVAL_DEFAULT_MS
        );
    }

    #[test]
    fn dashboard_launch_config_respects_disable_flags() {
        let cfg = parse_dashboard_launch_config(
            &[
                "--dashboard-autoboot=0".to_string(),
                "--dashboard-open=0".to_string(),
                "--gateway-persist=0".to_string(),
                "--dashboard-host=0.0.0.0".to_string(),
                "--dashboard-port=4321".to_string(),
                "--dashboard-ready-timeout-ms=1200".to_string(),
                "--dashboard-watchdog-interval-ms=150".to_string(),
            ],
            "start",
        );
        assert!(!cfg.enabled);
        assert!(!cfg.open_browser);
        assert!(!cfg.persistent_supervisor);
        assert!(!cfg.node_binary.trim().is_empty());
        assert_eq!(cfg.host, "0.0.0.0");
        assert_eq!(cfg.port, 4321);
        assert_eq!(cfg.ready_timeout_ms, 1_500);
        assert_eq!(cfg.watchdog_interval_ms, DASHBOARD_WATCHDOG_INTERVAL_MIN_MS);
    }

    #[test]
    fn dashboard_launch_config_heals_empty_node_binary_flag() {
        let cfg = parse_dashboard_launch_config(&["--node-binary=".to_string()], "watchdog");
        assert!(
            !cfg.node_binary.trim().is_empty(),
            "empty node flag should fall back to resolver instead of disabling dashboard spawn"
        );
    }

    #[test]
    fn resolve_dashboard_executable_prefers_sibling_infring_ops_for_infringd() {
        let temp = tempfile::tempdir().expect("tempdir");
        let dir = temp.path();
        let current = dir.join("infringd");
        let sibling = dir.join("infring-ops");
        std::fs::write(&current, b"#!/bin/sh\n").expect("write current");
        std::fs::write(&sibling, b"#!/bin/sh\n").expect("write sibling");
        let resolved = resolve_dashboard_executable(&current);
        assert_eq!(resolved, sibling);
    }

    #[test]
    fn resolve_dashboard_executable_keeps_current_when_sibling_missing() {
        let temp = tempfile::tempdir().expect("tempdir");
        let current = temp.path().join("infringd");
        std::fs::write(&current, b"#!/bin/sh\n").expect("write current");
        let resolved = resolve_dashboard_executable(&current);
        assert_eq!(resolved, current);
    }

    #[test]
    fn resolve_dashboard_executable_prefers_sibling_infring_ops_for_openclaw_alias() {
        let temp = tempfile::tempdir().expect("tempdir");
        let dir = temp.path();
        let current = dir.join("openclaw-ops");
        let sibling = dir.join("infring-ops");
        std::fs::write(&current, b"#!/bin/sh\n").expect("write current");
        std::fs::write(&sibling, b"#!/bin/sh\n").expect("write sibling");
        let resolved = resolve_dashboard_executable(&current);
        assert_eq!(resolved, sibling);
    }
}
