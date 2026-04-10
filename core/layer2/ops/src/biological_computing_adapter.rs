// Layer ownership: core/layer2/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use crate::ops_lane_runtime::{lane_spec, run_lane};
use std::path::Path;

const USAGE: &[&str] = &[
    "Usage:",
    "  protheus-ops biological-computing-adapter run|status|observe|stimulate|fallback [--policy=<path>] [--state-path=<path>] [--strict=1|0]",
];

pub fn run(root: &Path, argv: &[String]) -> i32 {
    run_lane(
        root,
        argv,
        &lane_spec(
            "biological_computing_adapter",
            "biological_computing_adapter",
            "protheus-ops biological-computing-adapter",
            USAGE,
            &["strict", "policy", "state-path", "consent"],
        ),
    )
}
