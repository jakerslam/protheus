// Layer ownership: core/layer2/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use crate::ops_lane_runtime::{run_lane, standard_lane_spec};
use std::path::Path;

const USAGE: &[&str] = &[
    "Usage:",
    "  protheus-ops context-doctor run|status|scan|export-json|export-png [--policy=<path>] [--state-path=<path>] [--strict=1|0]",
];

pub fn run(root: &Path, argv: &[String]) -> i32 {
    run_lane(
        root,
        argv,
        &standard_lane_spec(
            "context_doctor",
            "context_doctor",
            "protheus-ops context-doctor",
            USAGE,
        ),
    )
}
