mod startup_compaction_tests {
    use super::{
        dashboard_start_payload_ready, should_refresh_supervisor, supervisor_payload_healthy,
        supervisor_payload_running,
    };
    use serde_json::json;

    #[test]
    fn supervisor_refresh_skips_when_already_active_for_plain_start() {
        assert!(!should_refresh_supervisor(false, true));
    }

    #[test]
    fn supervisor_refresh_happens_when_inactive_or_forced() {
        assert!(should_refresh_supervisor(false, false));
        assert!(should_refresh_supervisor(true, true));
    }

    #[test]
    fn supervisor_payload_running_falls_back_to_active() {
        assert!(supervisor_payload_running(&json!({
            "ok": true,
            "active": true
        })));
        assert!(!supervisor_payload_running(&json!({
            "ok": true,
            "active": false
        })));
    }

    #[test]
    fn supervisor_payload_healthy_requires_running_and_active() {
        assert!(supervisor_payload_healthy(&json!({
            "ok": true,
            "active": true,
            "running": true
        })));
        assert!(!supervisor_payload_healthy(&json!({
            "ok": true,
            "active": true,
            "running": false
        })));
    }

    #[test]
    fn dashboard_start_payload_ready_requires_running_when_dashboard_is_enabled() {
        assert!(dashboard_start_payload_ready(&json!({
            "enabled": true,
            "running": true
        })));
        assert!(!dashboard_start_payload_ready(&json!({
            "enabled": true,
            "running": false,
            "error": "dashboard_healthz_not_ready"
        })));
        assert!(dashboard_start_payload_ready(&json!({
            "enabled": false,
            "running": false
        })));
    }

    #[test]
    fn dashboard_start_payload_ready_reads_restart_started_payload() {
        assert!(!dashboard_start_payload_ready(&json!({
            "stopped": {"ok": true},
            "started": {
                "enabled": true,
                "running": false,
                "error": "dashboard_healthz_not_ready"
            }
        })));
        assert!(dashboard_start_payload_ready(&json!({
            "stopped": {"ok": true},
            "started": {
                "enabled": true,
                "running": true
            }
        })));
    }
}
