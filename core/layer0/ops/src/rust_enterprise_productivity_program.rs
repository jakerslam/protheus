// SPDX-License-Identifier: Apache-2.0
use crate::ops_lane_runtime::{run_lane, LaneSpec};
use std::path::Path;

const USAGE: &[&str] = &[
    "Usage:",
    "  infring-ops rust-enterprise-productivity-program list [--policy=<path>]",
    "  infring-ops rust-enterprise-productivity-program run --id=<id> [--apply=1|0]",
    "  infring-ops rust-enterprise-productivity-program status [--id=<id>]",
    "  Optional telemetry flags: --trace-id=<id> --call-id=<id> --request-id=<id> --source=<tag>",
];

pub fn run(root: &Path, argv: &[String]) -> i32 {
    run_lane(
        root,
        argv,
        &LaneSpec {
            lane_id: "rust_enterprise_productivity_program",
            lane_type: "rust_enterprise_productivity_program",
            replacement: "infring-ops rust-enterprise-productivity-program",
            usage: USAGE,
            passthrough_flags: &[
                "apply",
                "strict",
                "policy",
                "id",
                "limit",
                "trace-id",
                "call-id",
                "request-id",
                "source",
            ],
        },
    )
}
