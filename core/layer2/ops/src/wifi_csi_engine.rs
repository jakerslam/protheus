// Layer ownership: core/layer2/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use crate::ops_lane_runtime::{lane_spec, run_lane};
use std::path::Path;

const USAGE: &[&str] = &[
    "Usage:",
    "  protheus-ops wifi-csi-engine run|status|detect|module-enable|module-disable [--policy=<path>] [--state-path=<path>] [--strict=1|0]",
];

pub fn run(root: &Path, argv: &[String]) -> i32 {
    run_lane(
        root,
        argv,
        &lane_spec(
            "wifi_csi_engine",
            "wifi_csi_engine",
            "protheus-ops wifi-csi-engine",
            USAGE,
            &["strict", "policy", "local-only", "state-path"],
        ),
    )
}
