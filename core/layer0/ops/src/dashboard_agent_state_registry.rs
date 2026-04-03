// Layer ownership: core/layer0/ops (authoritative)
// SPDX-License-Identifier: Apache-2.0
use chrono::{DateTime, Utc};
use serde_json::{json, Map, Value};
use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

const AGENT_PROFILES_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/agent_profiles.json";
const ARCHIVED_AGENTS_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/archived_agents.json";
const AGENT_CONTRACTS_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/agent_contracts.json";
const AGENT_SESSIONS_DIR_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/agent_sessions";
const DEFAULT_EXPIRY_SECONDS: i64 = 86_400;
const DEFAULT_IDLE_TIMEOUT_SECONDS: i64 = 3_600;
const MAX_EXPIRY_SECONDS: i64 = 31 * 24 * 60 * 60;
const MAX_IDLE_TIMEOUT_SECONDS: i64 = 31 * 24 * 60 * 60;

include!("dashboard_agent_state_registry_parts/001-part.rs");
include!("dashboard_agent_state_registry_parts/002-part.rs");
include!("dashboard_agent_state_registry_parts/003-part.rs");
