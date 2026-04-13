use crate::capability::{ToolCapabilityStatus, ToolReasonCode};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const DASHBOARD_STATE_OVERRIDE_ENV: &str = "INFRING_TOOLING_DASHBOARD_STATE_ROOT";

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum ToolBackendClass {
    RetrievalPlane,
    WorkspaceFs,
    AgentRuntime,
    GovernedTerminal,
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ToolBackendHealth {
    pub backend: String,
    pub backend_class: ToolBackendClass,
    pub status: ToolCapabilityStatus,
    pub reason_code: ToolReasonCode,
    pub reason: String,
    pub source: String,
    pub daemon_healthy: Option<bool>,
    pub ws_healthy: Option<bool>,
    pub auth_healthy: Option<bool>,
    pub resident_ipc_authoritative: bool,
}

#[derive(Debug, Default, Deserialize)]
struct ServerStatus {
    ok: Option<bool>,
    ws_bridge_enabled: Option<bool>,
    ws_bridge_error: Option<String>,
}

#[derive(Debug, Default, Deserialize)]
struct ProviderRegistry {
    providers: BTreeMap<String, ProviderStatus>,
}

#[derive(Debug, Default, Deserialize)]
struct ProviderStatus {
    auth_status: Option<String>,
    reachable: Option<bool>,
    needs_key: Option<bool>,
    is_local: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
struct TerminalBrokerState {
    history: Vec<TerminalHistoryRow>,
}

#[derive(Debug, Default, Deserialize)]
struct TerminalHistoryRow {
    ok: Option<bool>,
}

fn env_true(keys: &[&str]) -> bool {
    keys.iter().any(|key| {
        matches!(
            env::var(key)
                .unwrap_or_default()
                .trim()
                .to_ascii_lowercase()
                .as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}

fn release_channel() -> String {
    env::var("INFRING_RELEASE_CHANNEL")
        .or_else(|_| env::var("PROTHEUS_RELEASE_CHANNEL"))
        .unwrap_or_else(|_| "stable".to_string())
        .trim()
        .to_ascii_lowercase()
}

fn production_release_channel() -> bool {
    matches!(
        release_channel().as_str(),
        "stable" | "production" | "prod" | "ga" | "release"
    )
}

fn resident_ipc_authoritative() -> bool {
    if production_release_channel() {
        return true;
    }
    !env_true(&[
        "INFRING_OPS_ALLOW_PROCESS_FALLBACK",
        "PROTHEUS_OPS_ALLOW_PROCESS_FALLBACK",
        "INFRING_SDK_ALLOW_PROCESS_TRANSPORT",
        "INFRING_OPS_FORCE_LEGACY_PROCESS_RUNNER",
        "PROTHEUS_OPS_FORCE_LEGACY_PROCESS_RUNNER",
    ])
}

fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let raw = fs::read_to_string(path).ok()?;
    serde_json::from_str::<T>(&raw).ok()
}

fn override_dashboard_state_root() -> Option<PathBuf> {
    if let Ok(override_root) = env::var(DASHBOARD_STATE_OVERRIDE_ENV) {
        let candidate = PathBuf::from(override_root);
        if candidate.join("server_status.json").exists()
            || candidate.join("provider_registry.json").exists()
            || candidate.join("terminal_broker.json").exists()
        {
            return Some(candidate);
        }
        if candidate
            .join("client/runtime/local/state/ui/infring_dashboard")
            .exists()
        {
            return Some(candidate);
        }
    }
    None
}

fn looks_like_workspace_root(candidate: &Path) -> bool {
    candidate.join("client").exists()
        && candidate.join("core").exists()
        && candidate.join("docs/workspace").exists()
}

fn find_workspace_root() -> Option<PathBuf> {
    let mut candidates = Vec::<PathBuf>::new();
    if let Ok(cwd) = env::current_dir() {
        candidates.push(cwd);
    }
    if let Ok(manifest_dir) = env::var("CARGO_MANIFEST_DIR") {
        candidates.push(PathBuf::from(manifest_dir));
    }
    for candidate in candidates {
        for ancestor in candidate.ancestors() {
            if looks_like_workspace_root(ancestor) {
                return Some(ancestor.to_path_buf());
            }
            if ancestor
                .join("client/runtime/local/state/ui/infring_dashboard")
                .exists()
            {
                return Some(ancestor.to_path_buf());
            }
        }
    }
    None
}

fn dashboard_state_root() -> Option<PathBuf> {
    if let Some(override_root) = override_dashboard_state_root() {
        if override_root.join("server_status.json").exists()
            || override_root.join("provider_registry.json").exists()
            || override_root.join("terminal_broker.json").exists()
        {
            return Some(override_root);
        }
        return Some(override_root.join("client/runtime/local/state/ui/infring_dashboard"));
    }
    let root = find_workspace_root()?;
    Some(root.join("client/runtime/local/state/ui/infring_dashboard"))
}

fn backend_class_for(backend: &str) -> ToolBackendClass {
    match backend.trim().to_ascii_lowercase().as_str() {
        "retrieval_plane" => ToolBackendClass::RetrievalPlane,
        "workspace_fs" => ToolBackendClass::WorkspaceFs,
        "agent_runtime" => ToolBackendClass::AgentRuntime,
        "governed_terminal" => ToolBackendClass::GovernedTerminal,
        _ => ToolBackendClass::Unknown,
    }
}

fn retrieval_plane_health(state_root: &Path) -> ToolBackendHealth {
    let provider_path = state_root.join("provider_registry.json");
    let Some(registry) = read_json::<ProviderRegistry>(&provider_path) else {
        return ToolBackendHealth {
            backend: "retrieval_plane".to_string(),
            backend_class: ToolBackendClass::RetrievalPlane,
            status: ToolCapabilityStatus::Unavailable,
            reason_code: ToolReasonCode::TransportUnavailable,
            reason: "provider_registry_missing".to_string(),
            source: provider_path.display().to_string(),
            daemon_healthy: Some(resident_ipc_authoritative()),
            ws_healthy: None,
            auth_healthy: Some(false),
            resident_ipc_authoritative: resident_ipc_authoritative(),
        };
    };
    let providers = registry.providers.values().collect::<Vec<_>>();
    if providers.is_empty() {
        return ToolBackendHealth {
            backend: "retrieval_plane".to_string(),
            backend_class: ToolBackendClass::RetrievalPlane,
            status: ToolCapabilityStatus::Unavailable,
            reason_code: ToolReasonCode::TransportUnavailable,
            reason: "provider_registry_empty".to_string(),
            source: provider_path.display().to_string(),
            daemon_healthy: Some(resident_ipc_authoritative()),
            ws_healthy: None,
            auth_healthy: Some(false),
            resident_ipc_authoritative: resident_ipc_authoritative(),
        };
    }
    let auth_ready = providers.iter().any(|provider| {
        let auth_status = provider.auth_status.as_deref().unwrap_or("").trim();
        auth_status.eq_ignore_ascii_case("configured")
            || provider.needs_key == Some(false)
            || provider.is_local == Some(true)
    });
    let reachable = providers
        .iter()
        .any(|provider| provider.reachable == Some(true));
    let fully_healthy = providers.iter().all(|provider| {
        provider.reachable == Some(true)
            && (provider
                .auth_status
                .as_deref()
                .unwrap_or("")
                .eq_ignore_ascii_case("configured")
                || provider.needs_key == Some(false)
                || provider.is_local == Some(true))
    });
    let (status, reason_code, reason) = if !auth_ready {
        (
            ToolCapabilityStatus::Blocked,
            ToolReasonCode::AuthRequired,
            "provider_auth_missing".to_string(),
        )
    } else if !reachable {
        (
            ToolCapabilityStatus::Unavailable,
            ToolReasonCode::TransportUnavailable,
            "provider_unreachable".to_string(),
        )
    } else if !fully_healthy {
        (
            ToolCapabilityStatus::Degraded,
            ToolReasonCode::BackendDegraded,
            "provider_partial_degradation".to_string(),
        )
    } else {
        (
            ToolCapabilityStatus::Available,
            ToolReasonCode::Ok,
            "provider_reachable".to_string(),
        )
    };
    ToolBackendHealth {
        backend: "retrieval_plane".to_string(),
        backend_class: ToolBackendClass::RetrievalPlane,
        status,
        reason_code,
        reason,
        source: provider_path.display().to_string(),
        daemon_healthy: Some(resident_ipc_authoritative()),
        ws_healthy: None,
        auth_healthy: Some(auth_ready),
        resident_ipc_authoritative: resident_ipc_authoritative(),
    }
}

fn workspace_fs_health(root: &Path) -> ToolBackendHealth {
    let workspace_ready = root.join("client").exists() && root.join("core").exists();
    let (status, reason_code, reason) = if workspace_ready {
        (
            ToolCapabilityStatus::Available,
            ToolReasonCode::Ok,
            "workspace_access_ok".to_string(),
        )
    } else {
        (
            ToolCapabilityStatus::Unavailable,
            ToolReasonCode::TransportUnavailable,
            "workspace_root_unavailable".to_string(),
        )
    };
    ToolBackendHealth {
        backend: "workspace_fs".to_string(),
        backend_class: ToolBackendClass::WorkspaceFs,
        status,
        reason_code,
        reason,
        source: root.display().to_string(),
        daemon_healthy: None,
        ws_healthy: None,
        auth_healthy: None,
        resident_ipc_authoritative: resident_ipc_authoritative(),
    }
}

fn agent_runtime_health(state_root: &Path) -> ToolBackendHealth {
    let server_path = state_root.join("server_status.json");
    let Some(status_row) = read_json::<ServerStatus>(&server_path) else {
        return ToolBackendHealth {
            backend: "agent_runtime".to_string(),
            backend_class: ToolBackendClass::AgentRuntime,
            status: ToolCapabilityStatus::Unavailable,
            reason_code: ToolReasonCode::DaemonUnavailable,
            reason: "dashboard_server_status_missing".to_string(),
            source: server_path.display().to_string(),
            daemon_healthy: Some(false),
            ws_healthy: Some(false),
            auth_healthy: None,
            resident_ipc_authoritative: resident_ipc_authoritative(),
        };
    };
    let daemon_ok = status_row.ok.unwrap_or(false);
    let ws_ok = status_row.ws_bridge_enabled.unwrap_or(false);
    let ws_error = status_row
        .ws_bridge_error
        .as_deref()
        .unwrap_or("")
        .trim()
        .chars()
        .take(160)
        .collect::<String>();
    let (status, reason_code, reason) = if !daemon_ok {
        (
            ToolCapabilityStatus::Unavailable,
            ToolReasonCode::DaemonUnavailable,
            "dashboard_server_unavailable".to_string(),
        )
    } else if !ws_ok {
        (
            ToolCapabilityStatus::Unavailable,
            ToolReasonCode::WebsocketUnavailable,
            if ws_error.is_empty() {
                "ws_bridge_disabled".to_string()
            } else {
                format!("ws_bridge_disabled:{ws_error}")
            },
        )
    } else {
        (
            ToolCapabilityStatus::Available,
            ToolReasonCode::Ok,
            "agent_runtime_ready".to_string(),
        )
    };
    ToolBackendHealth {
        backend: "agent_runtime".to_string(),
        backend_class: ToolBackendClass::AgentRuntime,
        status,
        reason_code,
        reason,
        source: server_path.display().to_string(),
        daemon_healthy: Some(daemon_ok),
        ws_healthy: Some(ws_ok),
        auth_healthy: None,
        resident_ipc_authoritative: resident_ipc_authoritative(),
    }
}

fn governed_terminal_health(state_root: &Path) -> ToolBackendHealth {
    let terminal_path = state_root.join("terminal_broker.json");
    let fallback_active = !resident_ipc_authoritative();
    if fallback_active {
        return ToolBackendHealth {
            backend: "governed_terminal".to_string(),
            backend_class: ToolBackendClass::GovernedTerminal,
            status: ToolCapabilityStatus::Degraded,
            reason_code: ToolReasonCode::BackendDegraded,
            reason: "process_transport_fallback_active".to_string(),
            source: terminal_path.display().to_string(),
            daemon_healthy: Some(false),
            ws_healthy: None,
            auth_healthy: None,
            resident_ipc_authoritative: false,
        };
    }
    let Some(state) = read_json::<TerminalBrokerState>(&terminal_path) else {
        return ToolBackendHealth {
            backend: "governed_terminal".to_string(),
            backend_class: ToolBackendClass::GovernedTerminal,
            status: ToolCapabilityStatus::Degraded,
            reason_code: ToolReasonCode::BackendDegraded,
            reason: "terminal_broker_state_missing".to_string(),
            source: terminal_path.display().to_string(),
            daemon_healthy: Some(true),
            ws_healthy: None,
            auth_healthy: None,
            resident_ipc_authoritative: true,
        };
    };
    let last_ok = state.history.last().and_then(|row| row.ok).unwrap_or(true);
    let (status, reason_code, reason) = if state.history.is_empty() {
        (
            ToolCapabilityStatus::Degraded,
            ToolReasonCode::BackendDegraded,
            "terminal_broker_history_empty".to_string(),
        )
    } else if !last_ok {
        (
            ToolCapabilityStatus::Degraded,
            ToolReasonCode::BackendDegraded,
            "terminal_broker_last_run_failed".to_string(),
        )
    } else {
        (
            ToolCapabilityStatus::Available,
            ToolReasonCode::Ok,
            "terminal_broker_ready".to_string(),
        )
    };
    ToolBackendHealth {
        backend: "governed_terminal".to_string(),
        backend_class: ToolBackendClass::GovernedTerminal,
        status,
        reason_code,
        reason,
        source: terminal_path.display().to_string(),
        daemon_healthy: Some(true),
        ws_healthy: None,
        auth_healthy: None,
        resident_ipc_authoritative: true,
    }
}

pub fn live_backend_registry() -> Vec<ToolBackendHealth> {
    let state_root = dashboard_state_root();
    let workspace_root = find_workspace_root().unwrap_or_else(|| PathBuf::from("."));
    let state_root_fallback =
        workspace_root.join("client/runtime/local/state/ui/infring_dashboard");
    let state_root = state_root.unwrap_or(state_root_fallback);
    vec![
        retrieval_plane_health(&state_root),
        workspace_fs_health(&workspace_root),
        agent_runtime_health(&state_root),
        governed_terminal_health(&state_root),
    ]
}

pub fn live_backend_status_for(backend: &str) -> ToolBackendHealth {
    let normalized = backend.trim().to_ascii_lowercase();
    live_backend_registry()
        .into_iter()
        .find(|row| row.backend == normalized)
        .unwrap_or(ToolBackendHealth {
            backend: normalized,
            backend_class: backend_class_for(backend),
            status: ToolCapabilityStatus::Unavailable,
            reason_code: ToolReasonCode::UnknownTool,
            reason: "unknown_backend".to_string(),
            source: "runtime".to_string(),
            daemon_healthy: None,
            ws_healthy: None,
            auth_healthy: None,
            resident_ipc_authoritative: resident_ipc_authoritative(),
        })
}
