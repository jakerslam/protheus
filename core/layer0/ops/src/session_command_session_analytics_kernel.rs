// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
//
// Imported pattern contract (RTK intake):
// - source: local/workspace/vendor/rtk/src/discover/provider.rs
// - source: local/workspace/vendor/rtk/src/discover/mod.rs
// - source: local/workspace/vendor/rtk/src/analytics/session_cmd.rs
// - concept: provider transcript extraction + session-level command adoption analytics.

use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Map, Value};
use std::collections::HashMap;
use std::path::Path;

use crate::contract_lane_utils as lane_utils;
use crate::session_command_discovery_kernel::{
    classify_command_detail_for_kernel, classify_command_list_for_kernel,
    split_command_chain_for_kernel,
};
use crate::{deterministic_receipt_hash, now_iso};

#[derive(Debug, Clone)]
struct ExtractedCommand {
    command: String,
    output_len: Option<usize>,
    output_preview: Option<String>,
    is_error: bool,
    sequence_index: usize,
}

include!("session_command_session_analytics_kernel_parts/010-extract-and-cli.rs");
include!("session_command_session_analytics_kernel_parts/011-adoption-and-suggestions.rs");
include!("session_command_session_analytics_kernel_parts/020-run-and-tests.rs");
