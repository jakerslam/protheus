use protheus_tooling_core_v1::{live_backend_registry, ToolCapabilityStatus};
use std::fs::{create_dir_all, write};
use tempfile::tempdir;

const DASHBOARD_STATE_OVERRIDE_ENV: &str = "INFRING_TOOLING_DASHBOARD_STATE_ROOT";

#[test]
fn live_backend_registry_reads_dashboard_state_health() {
    let temp = tempdir().expect("tempdir");
    let dashboard = temp
        .path()
        .join("client/runtime/local/state/ui/infring_dashboard");
    create_dir_all(&dashboard).expect("mkdirs");
    write(
        dashboard.join("server_status.json"),
        r#"{"ok":true,"ws_bridge_enabled":true,"ws_bridge_error":""}"#,
    )
    .expect("server");
    write(
        dashboard.join("provider_registry.json"),
        r#"{"providers":{"ollama":{"auth_status":"configured","reachable":true,"needs_key":false,"is_local":true}}}"#,
    )
    .expect("providers");
    write(
        dashboard.join("terminal_broker.json"),
        r#"{"history":[{"ok":true}]}"#,
    )
    .expect("terminal");
    std::env::set_var(
        DASHBOARD_STATE_OVERRIDE_ENV,
        dashboard.display().to_string(),
    );
    let registry = live_backend_registry();
    std::env::remove_var(DASHBOARD_STATE_OVERRIDE_ENV);
    assert!(registry.iter().any(|row| {
        row.backend == "retrieval_plane"
            && row.status == ToolCapabilityStatus::Available
            && row.auth_healthy == Some(true)
    }));
    assert!(registry.iter().any(|row| {
        row.backend == "agent_runtime"
            && row.status == ToolCapabilityStatus::Available
            && row.ws_healthy == Some(true)
    }));
}
