// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::healthcare_plane (authoritative)
use crate::v8_kernel::{
    append_jsonl, build_conduit_enforcement, canonical_json_string, conduit_bypass_requested,
    emit_attached_plane_receipt, history_path, latest_path, parse_bool, parse_f64,
    parse_json_or_empty, read_json, read_jsonl, scoped_state_root, sha256_hex_str, write_json,
};
use crate::{clean, now_iso, parse_args};
use serde_json::{json, Map, Value};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
