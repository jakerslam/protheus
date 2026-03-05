#[path = "../legacy_bridge.rs"]
mod legacy_bridge;

use legacy_bridge::{run_legacy_bridge, LegacyBridgeSpec};

fn main() {
    let args = std::env::args().skip(1).collect::<Vec<String>>();
    let code = run_legacy_bridge(
        LegacyBridgeSpec {
            lane_id: "rust_memory_transition_lane",
            legacy_script_rel: "systems/memory/rust_memory_transition_lane_legacy.js",
        },
        &args,
    );
    std::process::exit(code);
}
