// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::skills_plane (authoritative)

use crate::v8_kernel::{
    append_jsonl, attach_conduit, build_plane_conduit_enforcement, canonical_json_string,
    conduit_bypass_requested, emit_plane_receipt, load_json_or, parse_bool, parse_u64,
    plane_status, print_json, read_json, scoped_state_root, sha256_hex_str, write_json,
};
use crate::{clean, parse_args};
use semver::Version;
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

const STATE_ENV: &str = "SKILLS_PLANE_STATE_ROOT";
const STATE_SCOPE: &str = "skills_plane";

const SCAFFOLD_CONTRACT_PATH: &str = "planes/contracts/skills/skill_scaffold_contract_v1.json";
const ACTIVATION_CONTRACT_PATH: &str = "planes/contracts/skills/skill_activation_contract_v1.json";
const CHAIN_CONTRACT_PATH: &str = "planes/contracts/skills/skill_chain_contract_v1.json";
const DX_CONTRACT_PATH: &str = "planes/contracts/skills/skill_dx_contract_v1.json";
const GALLERY_CONTRACT_PATH: &str =
    "planes/contracts/skills/skill_gallery_governance_contract_v1.json";
const GALLERY_MANIFEST_PATH: &str = "planes/contracts/skills/skill_gallery_manifest_v1.json";
const REACT_MINIMAL_CONTRACT_PATH: &str =
    "planes/contracts/skills/react_minimal_profile_contract_v1.json";
const TOT_DELIBERATE_CONTRACT_PATH: &str =
    "planes/contracts/skills/tot_deliberate_profile_contract_v1.json";
const DEFAULT_SKILLS_ROOT: &str = "client/runtime/systems/skills/packages";

fn usage() {
    println!("Usage:");
    println!("  infring-ops skills-plane status");
    println!("  infring-ops skills-plane list [--skills-root=<path>] [--strict=1|0]");
    println!("  infring-ops skills-plane dashboard [--skills-root=<path>] [--strict=1|0]");
    println!("  infring-ops skills-plane create --name=<skill-name> [--skills-root=<path>] [--strict=1|0]");
    println!("  infring-ops skills-plane activate --skill=<id> --trigger=<text> [--skills-root=<path>] [--strict=1|0]");
    println!("  infring-ops skills-plane chain-validate [--chain-json=<json>|--chain-path=<path>] [--skills-root=<path>] [--strict=1|0]");
    println!("  infring-ops skills-plane install --skill-path=<path> [--strict=1|0]");
    println!("  infring-ops skills-plane rollback --skill=<id> [--target-version=<version>] [--strict=1|0]");
    println!("  infring-ops skills-plane quarantine --op=<status|quarantine|release> [--skill=<id>] [--reason=<text>] [--strict=1|0]");
    println!("  infring-ops skills-plane run --skill=<id> [--input=<text>] [--strict=1|0]");
    println!("  infring-ops skills-plane share --skill=<id> [--target=<text>] [--strict=1|0]");
    println!("  infring-ops skills-plane gallery --op=<ingest|list|load> [--manifest=<path>] [--gallery-root=<path>] [--skill=<id>] [--strict=1|0]");
    println!(
        "  infring-ops skills-plane react-minimal --task=<text> [--max-steps=<n>] [--strict=1|0]"
    );
    println!("  infring-ops skills-plane tot-deliberate --task=<text> [--strategy=<bfs|dfs>] [--max-depth=<n>] [--branching=<n>] [--strict=1|0]");
}

fn path_has_disallowed_tokens(raw: &str) -> bool {
    let token = raw.trim();
    token.is_empty()
        || token.chars().any(|ch| ch == '\0' || ch.is_control())
        || Path::new(token)
            .components()
            .any(|part| matches!(part, std::path::Component::ParentDir))
}

fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn quarantine_path(root: &Path) -> PathBuf {
    state_root(root).join("quarantine").join("skills.json")
}

fn emit(root: &Path, payload: Value) -> i32 {
    emit_plane_receipt(root, STATE_ENV, STATE_SCOPE, "skills_plane_error", payload)
}

fn slugify(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.trim().chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if matches!(ch, '-' | '_' | ' ' | '/') {
            if !out.ends_with('-') {
                out.push('-');
            }
        }
    }
    while out.ends_with('-') {
        out.pop();
    }
    if out.is_empty() {
        "skill".to_string()
    } else {
        out
    }
}

#[derive(Clone, Copy)]
struct SkillVersion {
    major: u64,
    minor: u64,
    patch: u64,
    legacy: bool,
}

fn parse_skill_version(raw: &str) -> Option<SkillVersion> {
    let token = raw.trim();
    if token.is_empty() {
        return None;
    }
    if let Some(rest) = token.strip_prefix('v') {
        if !rest.is_empty() && rest.chars().all(|ch| ch.is_ascii_digit()) {
            let major = rest.parse::<u64>().ok()?;
            return Some(SkillVersion {
                major,
                minor: 0,
                patch: 0,
                legacy: true,
            });
        }
    }
    let normalized = token.strip_prefix('v').unwrap_or(token);
    if let Ok(version) = Version::parse(normalized) {
        return Some(SkillVersion {
            major: version.major,
            minor: version.minor,
            patch: version.patch,
            legacy: false,
        });
    }
    let mut pieces = normalized.split('.');
    let major = pieces.next()?.parse::<u64>().ok()?;
    let minor = pieces.next()?.parse::<u64>().ok()?;
    let patch = pieces.next()?.parse::<u64>().ok()?;
    if pieces.next().is_some() {
        return None;
    }
    Some(SkillVersion {
        major,
        minor,
        patch,
        legacy: false,
    })
}

fn version_cmp(left: SkillVersion, right: SkillVersion) -> std::cmp::Ordering {
    (left.major, left.minor, left.patch).cmp(&(right.major, right.minor, right.patch))
}

fn parse_skill_version_value(raw: &str) -> Value {
    match parse_skill_version(raw) {
        Some(v) => json!({
            "ok": true,
            "major": v.major,
            "minor": v.minor,
            "patch": v.patch,
            "legacy": v.legacy
        }),
        None => json!({
            "ok": false
        }),
    }
}

fn version_token(raw: &str) -> String {
    let mut out = String::new();
    for ch in raw.chars() {
        if ch.is_ascii_alphanumeric() {
            out.push(ch.to_ascii_lowercase());
        } else if matches!(ch, '.' | '-' | '_') && !out.ends_with('_') {
            out.push('_');
        }
    }
    out.trim_matches('_').to_string()
}

fn default_migration_lane_path(root: &Path, skill_id: &str, from: &str, to: &str) -> PathBuf {
    let from_token = if from.trim().is_empty() {
        "new".to_string()
    } else {
        version_token(from)
    };
    let to_token = if to.trim().is_empty() {
        "unknown".to_string()
    } else {
        version_token(to)
    };
    state_root(root)
        .join("migrations")
        .join("lanes")
        .join(format!(
            "{}_{}_to_{}.json",
            slugify(skill_id),
            from_token,
            to_token
        ))
}

fn default_migration_lane_path_with_policy(
    root: &Path,
    skill_id: &str,
    from: &str,
    to: &str,
    migration_lane: &str,
) -> PathBuf {
    let lane_token = slugify(migration_lane);
    if lane_token.is_empty() {
        return default_migration_lane_path(root, skill_id, from, to);
    }
    let from_token = if from.trim().is_empty() {
        "new".to_string()
    } else {
        version_token(from)
    };
    let to_token = if to.trim().is_empty() {
        "unknown".to_string()
    } else {
        version_token(to)
    };
    state_root(root)
        .join("migrations")
        .join("lanes")
        .join(lane_token)
        .join(format!(
            "{}_{}_to_{}.json",
            slugify(skill_id),
            from_token,
            to_token
        ))
}

fn rollback_checkpoint_path(root: &Path, skill_id: &str) -> PathBuf {
    state_root(root)
        .join("migrations")
        .join("checkpoints")
        .join(format!("{}.json", slugify(skill_id)))
}

fn load_backward_compat_policy(root: &Path) -> Value {
    let default_policy = json!({
        "policy": "semver_major",
        "min_version": "v1",
        "migration_lane": "skill_forced_migration",
        "receipt_required": true
    });
    load_json_or(
        root,
        "planes/contracts/srs/V8-SKILL-002.json",
        json!({"backward_compat": default_policy.clone()}),
    )
    .get("backward_compat")
    .cloned()
    .unwrap_or(default_policy)
}

fn evaluate_skill_run_backward_compat(root: &Path, skill: &str) -> Result<Value, String> {
    let registry_path = state_root(root).join("registry.json");
    let registry = read_json(&registry_path).unwrap_or_else(|| json!({"installed": {}}));
    let installed = registry
        .get("installed")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let Some(entry) = installed.get(skill) else {
        return Err("skill_not_installed".to_string());
    };
    let installed_version_raw = entry
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or("")
        .to_string();
    let Some(installed_version) = parse_skill_version(&installed_version_raw) else {
        return Err("skill_version_invalid".to_string());
    };

    let policy = load_backward_compat_policy(root);
    let policy_name = policy
        .get("policy")
        .and_then(Value::as_str)
        .unwrap_or("semver_major")
        .to_string();
    let min_version_raw = policy
        .get("min_version")
        .and_then(Value::as_str)
        .unwrap_or("v1")
        .to_string();
    let Some(min_version) = parse_skill_version(&min_version_raw) else {
        return Err("compat_min_version_invalid".to_string());
    };

    let gate_passed = if policy_name == "semver_major" {
        installed_version.major >= min_version.major
    } else {
        version_cmp(installed_version, min_version) != std::cmp::Ordering::Less
    };
    if !gate_passed {
        return Err("skill_version_below_minimum".to_string());
    }

    Ok(json!({
        "policy": policy_name,
        "min_version": min_version_raw,
        "installed_version": installed_version_raw,
        "migration_lane": policy.get("migration_lane").cloned().unwrap_or(Value::Null),
        "receipt_required": policy.get("receipt_required").cloned().unwrap_or(Value::Bool(true)),
        "installed_version_parsed": {
            "major": installed_version.major,
            "minor": installed_version.minor,
            "patch": installed_version.patch,
            "legacy": installed_version.legacy
        },
        "min_version_parsed": {
            "major": min_version.major,
            "minor": min_version.minor,
            "patch": min_version.patch,
            "legacy": min_version.legacy
        },
        "compatibility_gate_passed": true
    }))
}

fn conduit_enforcement(
    root: &Path,
    parsed: &crate::ParsedArgs,
    strict: bool,
    action: &str,
) -> Value {
    let bypass_requested = conduit_bypass_requested(&parsed.flags);
    build_plane_conduit_enforcement(
        root,
        STATE_ENV,
        STATE_SCOPE,
        strict,
        action,
        "skills_conduit_enforcement",
        "core/layer0/ops/skills_plane",
        bypass_requested,
        "skill_install_run_share_actions_route_through_layer0_conduit_with_deterministic_audit_receipts",
        &["V6-SKILLS-001.4"],
    )
}

fn status(root: &Path) -> Value {
    plane_status(root, STATE_ENV, STATE_SCOPE, "skills_plane_status")
}

fn load_jsonl(path: &Path) -> Vec<Value> {
    let raw = fs::read_to_string(path).unwrap_or_default();
    raw.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect()
}

fn skills_root_default(parsed: &crate::ParsedArgs) -> String {
    let raw = parsed
        .flags
        .get("skills-root")
        .cloned()
        .unwrap_or_else(|| DEFAULT_SKILLS_ROOT.to_string());
    if path_has_disallowed_tokens(&raw) {
        DEFAULT_SKILLS_ROOT.to_string()
    } else {
        raw
    }
}

fn skills_root(root: &Path, parsed: &crate::ParsedArgs) -> PathBuf {
    let rel_or_abs = parsed
        .flags
        .get("skills-root")
        .cloned()
        .unwrap_or_else(|| DEFAULT_SKILLS_ROOT.to_string());
    if path_has_disallowed_tokens(&rel_or_abs) {
        return root.join(DEFAULT_SKILLS_ROOT);
    }
    if Path::new(&rel_or_abs).is_absolute() {
        PathBuf::from(rel_or_abs)
    } else {
        root.join(rel_or_abs)
    }
}

fn write_file(path: &Path, body: &str) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("mkdir_failed:{}:{err}", parent.display()))?;
    }
    fs::write(path, body.as_bytes()).map_err(|err| format!("write_failed:{}:{err}", path.display()))
}
