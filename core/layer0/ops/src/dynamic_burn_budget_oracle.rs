// SPDX-License-Identifier: Apache-2.0
use crate::ops_lane_runtime::{run_lane, LaneSpec};
use std::path::Path;

const USAGE: &[&str] = &[
    "Usage:",
    "  infring-ops dynamic-burn-budget-oracle run [--policy=<path>] [--mock-file=<path>]",
    "  infring-ops dynamic-burn-budget-oracle status [--policy=<path>]",
    "  Optional telemetry flags: --trace-id=<id> --call-id=<id> --request-id=<id> --source=<tag>",
];

pub fn run(root: &Path, argv: &[String]) -> i32 {
    run_lane(
        root,
        argv,
        &LaneSpec {
            lane_id: "dynamic_burn_budget_oracle",
            lane_type: "dynamic_burn_budget_oracle",
            replacement: "infring-ops dynamic-burn-budget-oracle",
            usage: USAGE,
            passthrough_flags: &[
                "apply",
                "strict",
                "policy",
                "mock-file",
                "mock-json",
                "trace-id",
                "call-id",
                "request-id",
                "source",
            ],
        },
    )
}
