
const STATE_ENV: &str = "PERSIST_PLANE_STATE_ROOT";
const STATE_SCOPE: &str = "persist_plane";

const SCHEDULE_CONTRACT_PATH: &str = "planes/contracts/persist/schedule_contract_v1.json";
const MOBILE_CONTRACT_PATH: &str = "planes/contracts/persist/mobile_cockpit_contract_v1.json";
const CONTINUITY_CONTRACT_PATH: &str = "planes/contracts/persist/continuity_contract_v1.json";
const CONNECTOR_CONTRACT_PATH: &str =
    "planes/contracts/persist/connector_onboarding_contract_v1.json";
const COWORK_CONTRACT_PATH: &str = "planes/contracts/persist/cowork_background_contract_v1.json";
const MOBILE_DAEMON_CONTRACT_PATH: &str =
    "planes/contracts/mobile/mobile_daemon_bitnet_contract_v1.json";

#[path = "persist_plane_connector.rs"]
mod persist_plane_connector;
#[path = "persist_plane_continuity.rs"]
mod persist_plane_continuity;
#[path = "persist_plane_cowork.rs"]
mod persist_plane_cowork;

use persist_plane_connector::run_connector;
use persist_plane_continuity::run_continuity;
use persist_plane_cowork::run_cowork;

fn usage() {
    println!("Usage:");
    println!("  protheus-ops persist-plane status");
    println!(
        "  protheus-ops persist-plane schedule --op=<upsert|list|kickoff> [--job=<id>] [--cron=<expr>] [--workflow=<id>] [--owner=<id>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops persist-plane mobile-cockpit --op=<publish|status|intervene> [--session-id=<id>] [--device=<id>] [--action=<pause|resume|abort>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops persist-plane continuity --op=<checkpoint|reconstruct|status|validate> [--session-id=<id>] [--context-json=<json>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops persist-plane connector --op=<add|list|status|remove> [--provider=<slack|gmail|drive>] [--policy-template=<id>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops persist-plane cowork --op=<delegate|tick|status|list> [--task=<text>] [--parent=<id>] [--child=<id>] [--mode=<co-work|sub-agent>] [--budget-ms=<n>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops persist-plane mobile-daemon --op=<enable|status|handoff> [--platform=<android|ios>] [--edge-backend=<bitnet>] [--sensor-lanes=<camera,mic,gps>] [--handoff=<edge|cloud>] [--strict=1|0]"
    );
}

fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn emit(root: &Path, payload: Value) -> i32 {
    emit_plane_receipt(root, STATE_ENV, STATE_SCOPE, "persist_plane_error", payload)
}

fn status(root: &Path) -> Value {
    plane_status(root, STATE_ENV, STATE_SCOPE, "persist_plane_status")
}

fn claim_ids_for_action(action: &str) -> Vec<&'static str> {
    match action {
        "schedule" => vec!["V6-PERSIST-001.1", "V6-PERSIST-001.6"],
        "mobile-cockpit" => vec!["V6-PERSIST-001.2", "V6-PERSIST-001.6"],
        "continuity" => vec!["V6-PERSIST-001.3", "V6-PERSIST-001.6"],
        "connector" => vec!["V6-PERSIST-001.4", "V6-PERSIST-001.6"],
        "cowork" | "co-work" => vec!["V6-PERSIST-001.5", "V6-PERSIST-001.6"],
        "mobile-daemon" => vec!["V7-MOBILE-001.1", "V6-PERSIST-001.6"],
        _ => vec!["V6-PERSIST-001.6"],
    }
}

fn conduit_enforcement(
    root: &Path,
    parsed: &crate::ParsedArgs,
    strict: bool,
    action: &str,
) -> Value {
    let bypass_requested = conduit_bypass_requested(&parsed.flags);
    let claim_ids = claim_ids_for_action(action);
    build_plane_conduit_enforcement(
        root,
        STATE_ENV,
        STATE_SCOPE,
        strict,
        action,
        "persist_conduit_enforcement",
        "core/layer0/ops/persist_plane",
        bypass_requested,
        "persist_controls_route_through_layer0_conduit_with_fail_closed_denials",
        &claim_ids,
    )
}

fn persist_state_path(root: &Path, parts: &[&str]) -> PathBuf {
    let mut path = state_root(root);
    for part in parts {
        path.push(part);
    }
    path
}

fn schedules_path(root: &Path) -> PathBuf {
    persist_state_path(root, &["schedules", "registry.json"])
}

fn mobile_path(root: &Path) -> PathBuf {
    persist_state_path(root, &["mobile", "latest.json"])
}

fn continuity_dir(root: &Path) -> PathBuf {
    persist_state_path(root, &["continuity"])
}

fn continuity_snapshot_path(root: &Path, session_id: &str) -> PathBuf {
    continuity_dir(root)
        .join("snapshots")
        .join(format!("{session_id}.json"))
}

fn continuity_reconstruct_path(root: &Path, session_id: &str) -> PathBuf {
    continuity_dir(root)
        .join("reconstructed")
        .join(format!("{session_id}.json"))
}

fn connectors_path(root: &Path) -> PathBuf {
    persist_state_path(root, &["connectors", "registry.json"])
}

fn cowork_path(root: &Path) -> PathBuf {
    persist_state_path(root, &["cowork", "runs.json"])
}

fn mobile_daemon_path(root: &Path) -> PathBuf {
    persist_state_path(root, &["mobile", "daemon_profile.json"])
}

fn parse_json_flag(raw: Option<&String>) -> Option<Value> {
    raw.and_then(|text| serde_json::from_str::<Value>(text).ok())
}

fn clean_id(raw: Option<&str>, fallback: &str) -> String {
    let mut out = String::new();
    if let Some(v) = raw {
        for ch in v.trim().chars() {
            if out.len() >= 96 {
                break;
            }
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                out.push(ch.to_ascii_lowercase());
            } else {
                out.push('-');
            }
        }
    }
    let trimmed = out.trim_matches('-');
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}
