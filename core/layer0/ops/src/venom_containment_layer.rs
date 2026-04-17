// SPDX-License-Identifier: Apache-2.0
use crate::ops_lane_runtime::{run_lane, LaneSpec};
use std::path::Path;

const USAGE: &[&str] = &[
    "Usage:",
    "  protheus-ops venom-containment-layer evaluate [--session-id=<id>] [--apply=1|0]",
    "  protheus-ops venom-containment-layer status",
    "  Optional telemetry flags: --trace-id=<id> --call-id=<id> --request-id=<id> --source=<tag>",
];

pub fn run(root: &Path, argv: &[String]) -> i32 {
    run_lane(
        root,
        argv,
        &LaneSpec {
            lane_id: "venom_containment_layer",
            lane_type: "venom_containment_layer",
            replacement: "protheus-ops venom-containment-layer",
            usage: USAGE,
            passthrough_flags: &[
                "apply",
                "strict",
                "policy",
                "session-id",
                "source",
                "action",
                "risk",
                "trace-id",
                "call-id",
                "request-id",
            ],
        },
    )
}
