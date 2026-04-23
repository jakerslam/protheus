// Layer ownership: core/layer2/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use crate::ops_lane_runtime::{run_lane, standard_lane_spec};
use std::path::Path;

const USAGE: &[&str] = &[
    "Usage:",
    "  infring-ops observability-automation-engine run|status|monitor|schedule|incident [--policy=<path>] [--state-path=<path>] [--strict=1|0]",
];

pub fn run(root: &Path, argv: &[String]) -> i32 {
    run_lane(
        root,
        argv,
        &standard_lane_spec(
            "observability_automation_engine",
            "observability_automation_engine",
            "infring-ops observability-automation-engine",
            USAGE,
        ),
    )
}
