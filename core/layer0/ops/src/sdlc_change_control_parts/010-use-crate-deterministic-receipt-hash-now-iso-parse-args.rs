// SPDX-License-Identifier: Apache-2.0
use crate::{deterministic_receipt_hash, now_iso, parse_args};
use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};

const LANE_ID: &str = "sdlc_change_control";
const DEFAULT_POLICY_REL: &str = "client/runtime/config/sdlc_change_control_policy.json";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum RiskClass {
    Standard,
    Major,
    HighRisk,
}

impl RiskClass {
    fn as_str(&self) -> &'static str {
        match self {
            RiskClass::Standard => "standard",
            RiskClass::Major => "major",
            RiskClass::HighRisk => "high-risk",
        }
    }

    fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "standard" => Some(RiskClass::Standard),
            "major" => Some(RiskClass::Major),
            "high-risk" | "high_risk" | "highrisk" => Some(RiskClass::HighRisk),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
struct Policy {
    strict_default: bool,
    high_risk_path_prefixes: Vec<String>,
    major_path_prefixes: Vec<String>,
    required_approvers_major: usize,
    required_approvers_high_risk: usize,
    require_rfc_for_major: bool,
    require_adr_for_high_risk: bool,
    require_rollback_drill_for_high_risk: bool,
    require_approval_receipts_for_major: bool,
    latest_path: PathBuf,
    history_path: PathBuf,
    policy_path: PathBuf,
}

#[derive(Debug, Clone, Default)]
struct ChangeControlFields {
    risk_class_raw: String,
    rfc_link: String,
    adr_link: String,
    rollback_owner: String,
    rollback_plan: String,
    approvers: Vec<String>,
    approval_receipts: Vec<String>,
    rollback_drill_receipt: String,
}

fn usage() {
    println!("Usage:");
    println!("  protheus-ops sdlc-change-control run [--strict=1|0] [--policy=<path>] [--pr-body-path=<path>] [--changed-paths-path=<path>]");
    println!("  protheus-ops sdlc-change-control status [--policy=<path>]");
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn bool_flag(raw: Option<&String>, fallback: bool) -> bool {
    match raw.map(|v| v.trim().to_ascii_lowercase()) {
        Some(v) if matches!(v.as_str(), "1" | "true" | "yes" | "on") => true,
        Some(v) if matches!(v.as_str(), "0" | "false" | "no" | "off") => false,
        _ => fallback,
    }
}

fn resolve_path(root: &Path, raw: Option<&str>, fallback: &str) -> PathBuf {
    let token = raw.unwrap_or(fallback).trim();
    if token.is_empty() {
        return root.join(fallback);
    }
    let candidate = PathBuf::from(token);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn split_csv(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>()
}

fn canonical_key(line: &str) -> String {
    line.chars()
        .map(|c| c.to_ascii_lowercase())
        .filter(|c| c.is_ascii_alphanumeric() || *c == ' ' || *c == '-' || *c == '_')
        .collect::<String>()
        .replace('_', " ")
        .trim()
        .to_string()
}

fn parse_pr_body_fields(body: &str) -> ChangeControlFields {
    let mut fields = ChangeControlFields::default();

    for line in body.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let normalized = trimmed
            .trim_start_matches('-')
            .trim_start_matches('*')
            .trim();
        let Some((k, v)) = normalized.split_once(':') else {
            continue;
        };
        let key = canonical_key(k);
        let value = v.trim().to_string();

        match key.as_str() {
            "risk class" => fields.risk_class_raw = value,
            "rfc" | "rfc link" | "rfc ref" => fields.rfc_link = value,
            "adr" | "adr link" | "adr ref" => fields.adr_link = value,
            "rollback owner" => fields.rollback_owner = value,
            "rollback plan" => fields.rollback_plan = value,
            "approvers" => fields.approvers = split_csv(&value),
            "approval receipts" | "approval receipt" => {
                fields.approval_receipts = split_csv(&value)
            }
            "rollback drill receipt" => fields.rollback_drill_receipt = value,
            _ => {}
        }
    }

    fields
}

fn load_changed_paths(path: &Path) -> Vec<String> {
    let Ok(raw) = fs::read_to_string(path) else {
        return Vec::new();
    };
    raw.lines()
        .map(|line| line.trim().replace('\\', "/"))
        .filter(|line| !line.is_empty())
        .collect::<Vec<_>>()
}

fn parse_nonempty_string_array(value: Option<&Value>) -> Option<Vec<String>> {
    value
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|v| v.trim().replace('\\', "/"))
                .filter(|v| !v.is_empty())
                .collect::<Vec<_>>()
        })
        .filter(|rows| !rows.is_empty())
}

fn insert_check(checks: &mut BTreeMap<String, Value>, key: &str, value: Value) {
    checks.insert(key.to_string(), value);
}

fn insert_presence_check(checks: &mut BTreeMap<String, Value>, key: &str, value: &str) -> bool {
    let ok = ref_is_present(value);
    insert_check(checks, key, json!({ "ok": ok, "value": value }));
    ok
}

fn insert_required_ref_check(
    checks: &mut BTreeMap<String, Value>,
    key: &str,
    root: &Path,
    required: bool,
    value: &str,
) -> bool {
    let ok = !required || ref_exists(root, value);
    insert_check(
        checks,
        key,
        json!({
            "ok": ok,
            "required": required,
            "value": value
        }),
    );
    ok
}

fn insert_approver_check(
    checks: &mut BTreeMap<String, Value>,
    required_count: usize,
    approvers: &[String],
) -> bool {
    let ok = approvers.len() >= required_count;
    insert_check(
        checks,
        "approver_requirement",
        json!({
            "ok": ok,
            "required_count": required_count,
            "actual_count": approvers.len(),
            "approvers": approvers
        }),
    );
    ok
}

fn insert_approval_receipts_check(
    checks: &mut BTreeMap<String, Value>,
    root: &Path,
    required: bool,
    receipts: &[String],
) -> bool {
    let ok = !required || (!receipts.is_empty() && receipts.iter().all(|receipt| ref_exists(root, receipt)));
    insert_check(
        checks,
        "approval_receipts_requirement",
        json!({
            "ok": ok,
            "required": required,
            "receipts": receipts
        }),
    );
    ok
}

fn starts_with_any(path: &str, prefixes: &[String]) -> bool {
    prefixes.iter().any(|prefix| path.starts_with(prefix))
}

fn infer_risk_class(changed_paths: &[String], policy: &Policy) -> RiskClass {
    if changed_paths
        .iter()
        .any(|path| starts_with_any(path, &policy.high_risk_path_prefixes))
    {
        return RiskClass::HighRisk;
    }
    if changed_paths
        .iter()
        .any(|path| starts_with_any(path, &policy.major_path_prefixes))
    {
        return RiskClass::Major;
    }
    RiskClass::Standard
}
