// SPDX-License-Identifier: Apache-2.0
use crate::ops_lane_runtime::{run_lane, LaneSpec};
use std::path::Path;

const USAGE: &[&str] = &[
    "Usage:",
    "  infring-ops infring-control-plane <command> [flags]",
    "  infring-ops infring-control-plane status",
    "  Optional telemetry flags: --trace-id=<id> --call-id=<id> --request-id=<id> --source=<tag>",
];

pub fn run(root: &Path, argv: &[String]) -> i32 {
    run_lane(
        root,
        argv,
        &LaneSpec {
            lane_id: "infring_control_plane",
            lane_type: "infring_control_plane",
            replacement: "infring-ops infring-control-plane",
            usage: USAGE,
            passthrough_flags: &[
                "apply", "strict", "policy", "id", "limit", "statuses", "max", "action", "to",
                "trace-id", "call-id", "request-id", "source",
            ],
        },
    )
}
