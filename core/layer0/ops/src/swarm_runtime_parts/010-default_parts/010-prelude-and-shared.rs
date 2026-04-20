// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
use crate::{deterministic_receipt_hash, now_iso};
use serde::{Deserialize, Serialize};
use serde_json::{json, Map, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
