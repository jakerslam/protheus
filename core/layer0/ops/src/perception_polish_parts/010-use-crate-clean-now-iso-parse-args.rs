// SPDX-License-Identifier: Apache-2.0
use crate::{clean, now_iso, parse_args};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::path::{Path, PathBuf};

const IDS: [&str; 4] = [
    "V4-OBS-011",
    "V4-ILLUSION-001",
    "V4-AESTHETIC-001",
    "V4-AESTHETIC-002",
];

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Item {
    pub id: String,
    pub title: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Paths {
    pub state_path: PathBuf,
    pub latest_path: PathBuf,
    pub receipts_path: PathBuf,
    pub history_path: PathBuf,
    pub flags_path: PathBuf,
    pub observability_panel_path: PathBuf,
    pub reasoning_footer_path: PathBuf,
    pub tone_policy_path: PathBuf,
    pub post_reveal_easter_egg_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Policy {
    pub version: String,
    pub enabled: bool,
    pub strict_default: bool,
    pub items: Vec<Item>,
    pub paths: Paths,
    pub policy_path: PathBuf,
}

fn normalize_id(v: &str) -> String {
    let id = clean(v.replace('`', ""), 80).to_ascii_uppercase();
    if IDS.iter().any(|x| *x == id) {
        id
    } else {
        String::new()
    }
}

fn to_bool(v: Option<&str>, fallback: bool) -> bool {
    let Some(raw) = v else {
        return fallback;
    };
    match raw.trim().to_ascii_lowercase().as_str() {
        "1" | "true" | "yes" | "on" => true,
        "0" | "false" | "no" | "off" => false,
        _ => fallback,
    }
}

fn read_json(path: &Path) -> Value {
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str::<Value>(&raw).unwrap_or(Value::Null),
        Err(_) => Value::Null,
    }
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("create_dir_failed:{}:{e}", parent.display()))?;
    }
    Ok(())
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        chrono::Utc::now().timestamp_millis()
    ));
    let mut payload =
        serde_json::to_string_pretty(value).map_err(|e| format!("encode_json_failed:{e}"))?;
    payload.push('\n');
    fs::write(&tmp, payload).map_err(|e| format!("write_tmp_failed:{}:{e}", tmp.display()))?;
    fs::rename(&tmp, path).map_err(|e| format!("rename_tmp_failed:{}:{e}", path.display()))
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    ensure_parent(path)?;
    let mut payload = serde_json::to_string(row).map_err(|e| format!("encode_row_failed:{e}"))?;
    payload.push('\n');
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .and_then(|mut f| std::io::Write::write_all(&mut f, payload.as_bytes()))
        .map_err(|e| format!("append_jsonl_failed:{}:{e}", path.display()))
}

fn resolve_path(root: &Path, raw: Option<&Value>, fallback_rel: &str) -> PathBuf {
    let fallback = root.join(fallback_rel);
    let Some(raw) = raw.and_then(Value::as_str) else {
        return fallback;
    };
    let text = clean(raw, 400);
    if text.is_empty() {
        return fallback;
    }
    let pb = PathBuf::from(text);
    if pb.is_absolute() {
        pb
    } else {
        root.join(pb)
    }
}

fn rel_path(root: &Path, abs: &Path) -> String {
    abs.strip_prefix(root)
        .unwrap_or(abs)
        .to_string_lossy()
        .replace('\\', "/")
}

fn stable_hash(input: &str, len: usize) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    let hex = hex::encode(hasher.finalize());
    hex[..len.min(hex.len())].to_string()
}

pub fn default_policy(root: &Path) -> Policy {
    Policy {
        version: "1.0".to_string(),
        enabled: true,
        strict_default: true,
        items: IDS
            .iter()
            .map(|id| Item {
                id: (*id).to_string(),
                title: (*id).to_string(),
            })
            .collect(),
        paths: Paths {
            state_path: root.join("local/state/ops/perception_polish_program/state.json"),
            latest_path: root.join("local/state/ops/perception_polish_program/latest.json"),
            receipts_path: root.join("local/state/ops/perception_polish_program/receipts.jsonl"),
            history_path: root.join("local/state/ops/perception_polish_program/history.jsonl"),
            flags_path: root.join("client/runtime/config/feature_flags/perception_flags.json"),
            observability_panel_path: root
                .join("local/state/ops/infring_top/observability_panel.json"),
            reasoning_footer_path: root
                .join("local/state/ops/infring_top/reasoning_mirror_footer.txt"),
            tone_policy_path: root.join("client/runtime/config/perception_tone_policy.json"),
            post_reveal_easter_egg_path: root
                .join("docs/client/blog/the_fort_was_empty_easter_egg.md"),
        },
        policy_path: root.join("client/runtime/config/perception_polish_program_policy.json"),
    }
}
