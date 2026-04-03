// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use serde_json::{json, Map, Value};
use std::collections::{HashMap, HashSet};
use std::fs;
use std::path::{Path, PathBuf};

const AGENT_SESSIONS_DIR_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/agent_sessions";
const MAX_MESSAGES: usize = 4000;
const PROMPT_SUGGESTION_CONTEXT_WINDOW: usize = 7;
const PROMPT_SUGGESTION_MAX_WORDS: usize = 10;
const PROMPT_SUGGESTION_MAX_COUNT: usize = 3;

include!("dashboard_agent_state_sessions_parts/001-part.rs");
include!("dashboard_agent_state_sessions_parts/002-part.rs");
include!("dashboard_agent_state_sessions_parts/003-part.rs");
