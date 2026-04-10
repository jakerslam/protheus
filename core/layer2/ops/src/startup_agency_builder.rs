// Layer ownership: core/layer2/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use crate::ops_lane_runtime::{run_lane, standard_lane_spec};
use std::path::Path;

const USAGE: &[&str] = &[
    "Usage:",
    "  protheus-ops startup-agency-builder run|status|bootstrap|division|role [--policy=<path>] [--state-path=<path>] [--strict=1|0]",
];

pub fn run(root: &Path, argv: &[String]) -> i32 {
    run_lane(
        root,
        argv,
        &standard_lane_spec(
            "startup_agency_builder",
            "startup_agency_builder",
            "protheus-ops startup-agency-builder",
            USAGE,
        ),
    )
}
