// Layer ownership: core/layer2/tooling (authoritative canonical tool/evidence substrate).
use crate::backend_registry::{live_backend_registry, ToolBackendHealth};
use crate::capability::{
    all_capabilities_for_callers, capability_probe_for, grouped_capabilities_for_callers,
    ToolCapability, ToolCapabilityCatalogGroup, ToolCapabilityProbe, ToolCapabilityStatus,
    ToolReasonCode,
};
use crate::request_validation::{clean_text, repair_and_validate_args};
use crate::schemas::{NormalizedToolMetrics, NormalizedToolResult, NormalizedToolStatus};
use crate::{deterministic_hash, now_ms};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};
use std::fs::{create_dir_all, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;
