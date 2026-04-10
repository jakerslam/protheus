// Layer ownership: core/layer2/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use crate::ops_lane_runtime::{lane_spec, run_lane};
use std::path::Path;

const USAGE: &[&str] = &[
    "Usage:",
    "  protheus-ops company-layer-orchestration run|status|org-chart|budget|ticket|heartbeat [--policy=<path>] [--state-path=<path>] [--strict=1|0]",
];

pub fn run(root: &Path, argv: &[String]) -> i32 {
    run_lane(
        root,
        argv,
        &lane_spec(
            "company_layer_orchestration",
            "company_layer_orchestration",
            "protheus-ops company-layer-orchestration",
            USAGE,
            &["strict", "policy", "state-path", "budget"],
        ),
    )
}
