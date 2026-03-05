#[path = "../idle_dream_lane.rs"]
mod idle_dream_lane;
#[path = "../legacy_bridge.rs"]
mod legacy_bridge;

use legacy_bridge::{detect_repo_root, run_legacy_bridge, LegacyBridgeSpec};

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<String>>();
    let cwd = std::env::current_dir().unwrap_or_else(|_| std::path::PathBuf::from("."));
    let explicit_root = std::env::var("PROTHEUS_ROOT").ok();
    let repo_root = detect_repo_root(explicit_root.as_deref(), &cwd);
    if let Some(code) = idle_dream_lane::maybe_run(&repo_root, &args) {
        std::process::exit(code);
    }
    let code = run_legacy_bridge(
        LegacyBridgeSpec {
            lane_id: "idle_dream_cycle",
            legacy_script_rel: "systems/memory/idle_dream_cycle_legacy.js",
        },
        &args,
    );
    std::process::exit(code);
}
