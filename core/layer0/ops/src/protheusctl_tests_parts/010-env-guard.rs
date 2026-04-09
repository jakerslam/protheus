use super::*;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::time::{SystemTime, UNIX_EPOCH};

fn env_guard() -> std::sync::MutexGuard<'static, ()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
        .lock()
        .expect("lock_env")
}

#[test]
fn dispatch_security_gate_exempt_allows_help_surface() {
    assert!(dispatch_security_gate_exempt(
        "client/runtime/systems/ops/protheus_command_list.ts",
        &[]
    ));
    assert!(dispatch_security_gate_exempt(
        "client/runtime/systems/ops/protheus_command_list.js",
        &[]
    ));
}

#[test]
fn dispatch_security_gate_exempt_rejects_non_help_surface() {
    assert!(!dispatch_security_gate_exempt(
        "client/runtime/systems/ops/protheus_setup_wizard.ts",
        &[]
    ));
}

#[test]
fn resolve_workspace_root_walks_up_to_repo_marker() {
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("clock")
        .as_nanos();
    let base = std::env::temp_dir().join(format!("protheusctl_root_resolve_{nonce}"));
    let nested = base.join("tmp").join("nested").join("cwd");
    fs::create_dir_all(base.join("core/layer0/ops")).expect("ops dir");
    fs::create_dir_all(base.join("client/runtime")).expect("client runtime dir");
    fs::create_dir_all(&nested).expect("nested dir");
    fs::write(
        base.join("core/layer0/ops/Cargo.toml"),
        "[package]\nname=\"dummy\"\n",
    )
    .expect("manifest");

    let resolved = resolve_workspace_root(&nested).expect("resolved");
    assert_eq!(resolved, base);
    let _ = fs::remove_dir_all(base);
}

#[test]
fn route_edge_swarm_maps_correctly() {
    let route = route_edge(&[
        "swarm".to_string(),
        "enroll".to_string(),
        "--owner=operator".to_string(),
    ]);
    assert_eq!(
        route.script_rel,
        "client/runtime/systems/spawn/mobile_edge_swarm_bridge.ts"
    );
    assert_eq!(route.args.first().map(String::as_str), Some("enroll"));
}

#[test]
fn core_shortcut_routes_rag_command() {
    let route = resolve_core_shortcuts("rag", &["search".to_string(), "--q=proof".to_string()])
        .expect("route");
    assert_eq!(route.script_rel, "core://rag");
    assert_eq!(route.args.first().map(String::as_str), Some("search"));
}

#[test]
fn core_shortcut_routes_swarm_command() {
    let route = resolve_core_shortcuts("swarm", &["test".to_string(), "recursive".to_string()])
        .expect("route");
    assert_eq!(route.script_rel, "core://swarm-runtime");
    assert_eq!(route.args, vec!["test", "recursive"]);
}

#[test]
fn core_shortcut_routes_memory_command() {
    let route = resolve_core_shortcuts("memory", &["search".to_string(), "--q=ledger".to_string()])
        .expect("route");
    assert_eq!(route.script_rel, "core://rag");
    assert_eq!(route.args.first().map(String::as_str), Some("memory"));
    assert_eq!(route.args.get(1).map(String::as_str), Some("search"));
}

#[test]
fn core_shortcut_routes_alpha_check_to_alpha_readiness_domain() {
    let route = resolve_core_shortcuts("alpha-check", &[]).expect("route");
    assert_eq!(route.script_rel, "core://alpha-readiness");
    assert_eq!(route.args, vec!["run"]);
}

#[test]
fn core_shortcut_routes_alpha_check_flags_default_to_run_subcommand() {
    let route = resolve_core_shortcuts("alpha-check", &["--strict=1".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://alpha-readiness");
    assert_eq!(route.args, vec!["run", "--strict=1"]);
}

#[test]
fn status_dashboard_token_strip_removes_wrapper_flags() {
    let args = strip_status_dashboard_tokens(vec![
        "--dashboard".to_string(),
        "--web".to_string(),
        "--host=127.0.0.1".to_string(),
        "--port=4173".to_string(),
    ]);
    assert_eq!(args, vec!["--host=127.0.0.1", "--port=4173"]);
}

#[test]
fn core_shortcut_routes_start_to_daemon_control() {
    let route = resolve_core_shortcuts("start", &["--mode=persistent".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://daemon-control");
    assert_eq!(route.args, vec!["start", "--mode=persistent"]);
}

#[test]
fn core_shortcut_routes_gateway_default_to_daemon_start() {
    let route = resolve_core_shortcuts("gateway", &[]).expect("route");
    assert_eq!(route.script_rel, "core://daemon-control");
    assert_eq!(route.args, vec!["start"]);
}

#[test]
fn core_shortcut_routes_gateway_stop_to_daemon_stop() {
    let route = resolve_core_shortcuts("gateway", &["stop".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://daemon-control");
    assert_eq!(route.args, vec!["stop"]);
}

#[test]
fn core_shortcut_routes_gateway_preserves_start_flags() {
    let route =
        resolve_core_shortcuts("gateway", &["--dashboard-open=0".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://daemon-control");
    assert_eq!(route.args, vec!["start", "--dashboard-open=0"]);
}

#[test]
fn core_shortcut_routes_gateway_preserves_persistence_flag() {
    let route =
        resolve_core_shortcuts("gateway", &["--gateway-persist=0".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://daemon-control");
    assert_eq!(route.args, vec!["start", "--gateway-persist=0"]);
}

#[test]
fn core_shortcut_routes_verity_default_to_drift_status() {
    let route = resolve_core_shortcuts("verity", &[]).expect("route");
    assert_eq!(route.script_rel, "core://verity-plane");
    assert_eq!(route.args, vec!["status"]);
}

#[test]
fn core_shortcut_routes_verity_status_alias_to_drift_status() {
    let route = resolve_core_shortcuts("verity", &["status".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://verity-plane");
    assert_eq!(route.args, vec!["status"]);
}

#[test]
fn core_shortcut_routes_chat_with_files() {
    let route = resolve_core_shortcuts(
        "chat",
        &[
            "with".to_string(),
            "files".to_string(),
            "receipts".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://rag");
    assert_eq!(route.args.first().map(String::as_str), Some("chat"));
    assert_eq!(route.args.get(1).map(String::as_str), Some("receipts"));
}

#[test]
fn core_shortcut_routes_chat_nano_to_rag_domain() {
    let route = resolve_core_shortcuts("chat", &["nano".to_string(), "--q=hello".to_string()])
        .expect("route");
    assert_eq!(route.script_rel, "core://rag");
    assert_eq!(route.args, vec!["chat", "nano", "--q=hello"]);
}

#[test]
fn core_shortcut_routes_train_nano_to_rag_domain() {
    let route = resolve_core_shortcuts("train", &["nano".to_string(), "--depth=12".to_string()])
        .expect("route");
    assert_eq!(route.script_rel, "core://rag");
    assert_eq!(route.args, vec!["train", "nano", "--depth=12"]);
}

#[test]
fn core_shortcut_routes_nano_fork_to_rag_domain() {
    let route = resolve_core_shortcuts(
        "nano",
        &["fork".to_string(), "--target=.nanochat/fork".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://rag");
    assert_eq!(route.args, vec!["nano", "fork", "--target=.nanochat/fork"]);
}

#[test]
fn core_shortcut_routes_eval_enable_neuralavb() {
    let route = resolve_core_shortcuts(
        "eval",
        &[
            "enable".to_string(),
            "neuralavb".to_string(),
            "--enabled=1".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://eval-plane");
    assert_eq!(route.args, vec!["enable-neuralavb", "--enabled=1"]);
}

#[test]
fn core_shortcut_routes_experiment_loop() {
    let route = resolve_core_shortcuts(
        "experiment",
        &[
            "loop".to_string(),
            "--run-cost-usd=8".to_string(),
            "--baseline-cost-usd=20".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://eval-plane");
    assert_eq!(
        route.args,
        vec![
            "experiment-loop",
            "--run-cost-usd=8",
            "--baseline-cost-usd=20"
        ]
    );
}

#[test]
fn core_shortcut_routes_rl_upgrade_infring_v2() {
    let route = resolve_core_shortcuts(
        "rl",
        &[
            "upgrade".to_string(),
            "infring-v2".to_string(),
            "--iterations=6".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://eval-plane");
    assert_eq!(route.args, vec!["rl-upgrade", "--iterations=6"]);
}

#[test]
fn core_shortcut_routes_model_optimize_minimax() {
    let route = resolve_core_shortcuts(
        "model",
        &[
            "optimize".to_string(),
            "minimax".to_string(),
            "--compact-lines=20".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://model-router");
    assert_eq!(
        route.args,
        vec!["optimize", "--profile=minimax", "--compact-lines=20"]
    );
}

#[test]
fn core_shortcut_routes_model_use_cheap_to_model_router() {
    let route = resolve_core_shortcuts(
        "model",
        &[
            "use".to_string(),
            "cheap".to_string(),
            "--compact-lines=24".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://model-router");
    assert_eq!(
        route.args,
        vec!["optimize", "--profile=minimax", "--compact-lines=24"]
    );
}

#[test]
fn core_shortcut_routes_model_use_bitnet_to_model_router() {
    let route = resolve_core_shortcuts(
        "model",
        &[
            "use".to_string(),
            "bitnet".to_string(),
            "--source-model=hf://infring/bitnet-base".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://model-router");
    assert_eq!(
        route.args,
        vec!["bitnet-use", "--source-model=hf://infring/bitnet-base"]
    );
}

#[test]
fn core_shortcut_routes_agent_reset_to_model_router() {
    let route = resolve_core_shortcuts(
        "agent",
        &["reset".to_string(), "--scope=routing".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://model-router");
    assert_eq!(route.args, vec!["reset-agent", "--scope=routing"]);
}

#[test]
fn core_shortcut_routes_economy_to_core_domain() {
    let route = resolve_core_shortcuts(
        "economy",
        &[
            "enable".to_string(),
            "all".to_string(),
            "--apply=1".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://llm-economy-organ");
    assert_eq!(route.args, vec!["enable", "all", "--apply=1"]);
}

#[test]
fn core_shortcut_routes_economy_upgrade_trading_hand() {
    let route = resolve_core_shortcuts(
        "economy",
        &[
            "upgrade".to_string(),
            "trading-hand".to_string(),
            "--mode=paper".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://llm-economy-organ");
    assert_eq!(route.args, vec!["upgrade-trading-hand", "--mode=paper"]);
}

#[test]
fn core_shortcut_routes_agent_debate_bullbear_to_economy() {
    let route = resolve_core_shortcuts(
        "agent",
        &[
            "debate".to_string(),
            "bullbear".to_string(),
            "--symbol=BTCUSD".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://llm-economy-organ");
    assert_eq!(route.args, vec!["debate-bullbear", "--symbol=BTCUSD"]);
}

#[test]
fn core_shortcut_routes_network_join_hyperspace() {
    let route = resolve_core_shortcuts(
        "network",
        &[
            "join".to_string(),
            "hyperspace".to_string(),
            "--node=alpha".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://network-protocol");
    assert_eq!(route.args, vec!["join-hyperspace", "--node=alpha"]);
}

#[test]
fn core_shortcut_routes_network_dashboard_to_hyperspace_core_lane() {
    let route = resolve_core_shortcuts("network", &["dashboard".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://network-protocol");
    assert_eq!(route.args, vec!["dashboard"]);
}

#[test]
fn core_shortcut_routes_network_ignite_bitcoin() {
    let route = resolve_core_shortcuts(
        "network",
        &[
            "ignite".to_string(),
            "bitcoin".to_string(),
            "--apply=1".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://network-protocol");
    assert_eq!(route.args, vec!["ignite-bitcoin", "--apply=1"]);
}

#[test]
fn core_shortcut_routes_network_status_to_network_protocol() {
    let route = resolve_core_shortcuts("network", &["status".to_string()]).expect("route");
    assert_eq!(route.script_rel, "core://network-protocol");
    assert_eq!(route.args, vec!["status"]);
}

#[test]
fn core_shortcut_routes_network_merkle_root_to_network_protocol() {
    let route = resolve_core_shortcuts(
        "network",
        &[
            "merkle-root".to_string(),
            "--account=shadow:alpha".to_string(),
            "--proof=1".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://network-protocol");
    assert_eq!(
        route.args,
        vec!["merkle-root", "--account=shadow:alpha", "--proof=1"]
    );
}

#[test]
fn core_shortcut_routes_enterprise_compliance_export_to_core_lane() {
    let route = resolve_core_shortcuts(
        "enterprise",
        &[
            "compliance".to_string(),
            "export".to_string(),
            "--profile=auditor".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://enterprise-hardening");
    assert_eq!(route.args, vec!["export-compliance", "--profile=auditor"]);
}

#[test]
fn core_shortcut_routes_enterprise_scale_to_core_lane() {
    let route = resolve_core_shortcuts(
        "enterprise",
        &["scale".to_string(), "--target-nodes=10000".to_string()],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://enterprise-hardening");
    assert_eq!(route.args, vec!["certify-scale", "--target-nodes=10000"]);
}

#[test]
fn core_shortcut_routes_enterprise_enable_bedrock_to_core_lane() {
    let route = resolve_core_shortcuts(
        "enterprise",
        &[
            "enable".to_string(),
            "bedrock".to_string(),
            "--region=us-west-2".to_string(),
        ],
    )
    .expect("route");
    assert_eq!(route.script_rel, "core://enterprise-hardening");
    assert_eq!(route.args, vec!["enable-bedrock", "--region=us-west-2"]);
}
