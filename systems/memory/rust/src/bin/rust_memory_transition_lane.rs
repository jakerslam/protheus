#[path = "../lane_contracts.rs"]
mod lane_contracts;
#[path = "../legacy_bridge.rs"]
mod legacy_bridge;
#[path = "../transition_lane.rs"]
mod transition_lane;

use legacy_bridge::{detect_repo_root, run_legacy_bridge, LegacyBridgeSpec};

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<String>>();
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let explicit_root = std::env::var("PROTHEUS_ROOT").ok();
    let repo_root = detect_repo_root(explicit_root.as_deref(), &cwd);
    if let Some(code) = transition_lane::maybe_run(&repo_root, &args) {
        std::process::exit(code);
    }
    let code = run_legacy_bridge(
        LegacyBridgeSpec {
            lane_id: "rust_memory_transition_lane",
            legacy_script_rel: "systems/memory/rust_memory_transition_lane_legacy.js",
        },
        &args,
    );
    std::process::exit(code);
}
