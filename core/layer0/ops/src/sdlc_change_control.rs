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

fn ref_is_present(raw: &str) -> bool {
    let value = raw.trim();
    !value.is_empty() && !matches!(value.to_ascii_lowercase().as_str(), "n/a" | "none" | "tbd")
}

fn looks_like_url(raw: &str) -> bool {
    let value = raw.trim().to_ascii_lowercase();
    value.starts_with("http://") || value.starts_with("https://")
}

fn ref_exists(root: &Path, raw: &str) -> bool {
    if !ref_is_present(raw) {
        return false;
    }
    if looks_like_url(raw) {
        return true;
    }
    root.join(raw.trim()).exists()
}

fn ensure_parent(path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
}

fn write_text_atomic(path: &Path, text: &str) -> Result<(), String> {
    ensure_parent(path);
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    fs::write(&tmp, text).map_err(|e| format!("write_tmp_failed:{}:{e}", path.display()))?;
    fs::rename(&tmp, path).map_err(|e| format!("rename_tmp_failed:{}:{e}", path.display()))
}

fn append_jsonl(path: &Path, value: &Value) -> Result<(), String> {
    ensure_parent(path);
    let mut f = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("open_jsonl_failed:{}:{e}", path.display()))?;
    let line = serde_json::to_string(value).map_err(|e| format!("encode_jsonl_failed:{e}"))?;
    f.write_all(line.as_bytes())
        .and_then(|_| f.write_all(b"\n"))
        .map_err(|e| format!("append_jsonl_failed:{}:{e}", path.display()))
}

fn load_policy(root: &Path, policy_override: Option<&String>) -> Policy {
    let policy_path = policy_override
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join(DEFAULT_POLICY_REL));

    let raw = fs::read_to_string(&policy_path)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .unwrap_or_else(|| json!({}));

    let high_risk_path_prefixes = parse_nonempty_string_array(raw.get("high_risk_path_prefixes"))
        .unwrap_or_else(|| {
            vec![
                "core/layer0/security/".to_string(),
                "core/layer2/conduit/".to_string(),
                "client/runtime/systems/security/".to_string(),
                "client/runtime/config/protheus_conduit_policy.json".to_string(),
                "client/runtime/config/rust_source_of_truth_policy.json".to_string(),
            ]
        });

    let major_path_prefixes = parse_nonempty_string_array(raw.get("major_path_prefixes"))
        .unwrap_or_else(|| {
            vec![
                "core/layer0/ops/".to_string(),
                "client/runtime/systems/ops/".to_string(),
                ".github/workflows/".to_string(),
                "client/runtime/config/".to_string(),
            ]
        });

    let outputs = raw.get("outputs").and_then(Value::as_object);

    Policy {
        strict_default: raw
            .get("strict_default")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        high_risk_path_prefixes,
        major_path_prefixes,
        required_approvers_major: raw
            .get("required_approvers_major")
            .and_then(Value::as_u64)
            .unwrap_or(1) as usize,
        required_approvers_high_risk: raw
            .get("required_approvers_high_risk")
            .and_then(Value::as_u64)
            .unwrap_or(2) as usize,
        require_rfc_for_major: raw
            .get("require_rfc_for_major")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        require_adr_for_high_risk: raw
            .get("require_adr_for_high_risk")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        require_rollback_drill_for_high_risk: raw
            .get("require_rollback_drill_for_high_risk")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        require_approval_receipts_for_major: raw
            .get("require_approval_receipts_for_major")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        latest_path: resolve_path(
            root,
            outputs
                .and_then(|o| o.get("latest_path"))
                .and_then(Value::as_str),
            "local/state/ops/sdlc_change_control/latest.json",
        ),
        history_path: resolve_path(
            root,
            outputs
                .and_then(|o| o.get("history_path"))
                .and_then(Value::as_str),
            "local/state/ops/sdlc_change_control/history.jsonl",
        ),
        policy_path,
    }
}

fn evaluate(root: &Path, policy: &Policy, pr_body_path: &Path, changed_paths_path: &Path) -> Value {
    let pr_body = fs::read_to_string(pr_body_path).unwrap_or_default();
    let fields = parse_pr_body_fields(&pr_body);
    let changed_paths = load_changed_paths(changed_paths_path);

    let inferred = infer_risk_class(&changed_paths, policy);
    let declared = RiskClass::parse(&fields.risk_class_raw).unwrap_or(RiskClass::Standard);

    let mut checks = BTreeMap::<String, Value>::new();
    let declared_valid = RiskClass::parse(&fields.risk_class_raw).is_some();
    insert_check(
        &mut checks,
        "declared_risk_class_valid",
        json!({
            "ok": declared_valid,
            "declared": fields.risk_class_raw,
            "allowed": ["standard", "major", "high-risk"]
        }),
    );
    insert_check(
        &mut checks,
        "declared_not_understated",
        json!({
            "ok": declared >= inferred,
            "declared": declared.as_str(),
            "inferred": inferred.as_str()
        }),
    );

    let rollback_plan_ok = insert_presence_check(
        &mut checks,
        "rollback_plan_present",
        &fields.rollback_plan,
    );
    let rollback_owner_ok = insert_presence_check(
        &mut checks,
        "rollback_owner_present",
        &fields.rollback_owner,
    );

    let require_rfc = declared >= RiskClass::Major && policy.require_rfc_for_major;
    insert_required_ref_check(
        &mut checks,
        "rfc_link_requirement",
        root,
        require_rfc,
        &fields.rfc_link,
    );

    let require_adr = declared == RiskClass::HighRisk && policy.require_adr_for_high_risk;
    insert_required_ref_check(
        &mut checks,
        "adr_link_requirement",
        root,
        require_adr,
        &fields.adr_link,
    );

    let approver_req = if declared == RiskClass::HighRisk {
        policy.required_approvers_high_risk
    } else if declared == RiskClass::Major {
        policy.required_approvers_major
    } else {
        0
    };
    insert_approver_check(&mut checks, approver_req, &fields.approvers);

    let require_approval_receipts =
        declared >= RiskClass::Major && policy.require_approval_receipts_for_major;
    let approval_receipts_ok = insert_approval_receipts_check(
        &mut checks,
        root,
        require_approval_receipts,
        &fields.approval_receipts,
    );

    let require_rollback_drill =
        declared == RiskClass::HighRisk && policy.require_rollback_drill_for_high_risk;
    let rollback_drill_ok = insert_required_ref_check(
        &mut checks,
        "rollback_drill_requirement",
        root,
        require_rollback_drill,
        &fields.rollback_drill_receipt,
    );

    let blocking_checks = checks
        .iter()
        .filter_map(|(k, v)| {
            if v.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                None
            } else {
                Some(k.clone())
            }
        })
        .collect::<Vec<_>>();

    let ok = blocking_checks.is_empty();

    json!({
        "ok": ok,
        "type": "sdlc_change_control_run",
        "schema_id": "sdlc_change_control",
        "schema_version": "1.0",
        "lane": LANE_ID,
        "ts": now_iso(),
        "declared_risk_class": declared.as_str(),
        "inferred_risk_class": inferred.as_str(),
        "checks": checks,
        "blocking_checks": blocking_checks,
        "inputs": {
            "pr_body_path": pr_body_path,
            "changed_paths_path": changed_paths_path,
            "changed_paths_count": changed_paths.len()
        },
        "claim_evidence": [
            {
                "id": "sdlc_change_class_enforcement",
                "claim": "risk_classes_enforce_rfc_adr_approvals_and_rollback_ownership",
                "evidence": {
                    "declared": declared.as_str(),
                    "inferred": inferred.as_str(),
                    "approver_requirement": approver_req,
                    "approver_count": fields.approvers.len(),
                    "rollback_owner_present": rollback_owner_ok,
                    "rollback_plan_present": rollback_plan_ok
                }
            },
            {
                "id": "sdlc_high_risk_merge_gate",
                "claim": "high_risk_changes_fail_closed_without_approval_receipts_and_rollback_drill_evidence",
                "evidence": {
                    "high_risk": declared == RiskClass::HighRisk,
                    "approval_receipts_ok": approval_receipts_ok,
                    "rollback_drill_ok": rollback_drill_ok
                }
            }
        ]
    })
}

fn run_cmd(
    root: &Path,
    policy: &Policy,
    strict: bool,
    pr_body_path: &Path,
    changed_paths_path: &Path,
) -> Result<(Value, i32), String> {
    let mut payload = evaluate(root, policy, pr_body_path, changed_paths_path);
    payload["strict"] = Value::Bool(strict);
    payload["policy_path"] = Value::String(policy.policy_path.to_string_lossy().to_string());
    payload["receipt_hash"] = Value::String(deterministic_receipt_hash(&payload));

    write_text_atomic(
        &policy.latest_path,
        &format!(
            "{}\n",
            serde_json::to_string_pretty(&payload)
                .map_err(|e| format!("encode_latest_failed:{e}"))?
        ),
    )?;
    append_jsonl(&policy.history_path, &payload)?;

    let code = if strict && !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        1
    } else {
        0
    };

    Ok((payload, code))
}

fn status_cmd(policy: &Policy) -> Value {
    let latest = fs::read_to_string(&policy.latest_path)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .unwrap_or_else(|| {
            json!({
                "ok": false,
                "type": "sdlc_change_control_status",
                "error": "latest_missing"
            })
        });

    let mut out = json!({
        "ok": latest.get("ok").and_then(Value::as_bool).unwrap_or(false),
        "type": "sdlc_change_control_status",
        "lane": LANE_ID,
        "ts": now_iso(),
        "latest": latest,
        "policy_path": policy.policy_path,
        "latest_path": policy.latest_path,
        "history_path": policy.history_path
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

fn cli_error_receipt(argv: &[String], err: &str, code: i32) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "sdlc_change_control_cli_error",
        "lane": LANE_ID,
        "ts": now_iso(),
        "argv": argv,
        "error": err,
        "exit_code": code
    });
    out["receipt_hash"] = Value::String(deterministic_receipt_hash(&out));
    out
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let cmd = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(cmd.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }

    let policy = load_policy(root, parsed.flags.get("policy"));
    let strict = bool_flag(parsed.flags.get("strict"), policy.strict_default);
    let pr_body_path = resolve_path(
        root,
        parsed.flags.get("pr-body-path").map(String::as_str),
        "local/state/ops/sdlc_change_control/pr_body.md",
    );
    let changed_paths_path = resolve_path(
        root,
        parsed.flags.get("changed-paths-path").map(String::as_str),
        "local/state/ops/sdlc_change_control/changed_paths.txt",
    );

    match cmd.as_str() {
        "run" => match run_cmd(root, &policy, strict, &pr_body_path, &changed_paths_path) {
            Ok((payload, code)) => {
                print_json_line(&payload);
                code
            }
            Err(err) => {
                print_json_line(&cli_error_receipt(argv, &format!("run_failed:{err}"), 1));
                1
            }
        },
        "status" => {
            print_json_line(&status_cmd(&policy));
            0
        }
        _ => {
            usage();
            print_json_line(&cli_error_receipt(argv, "unknown_command", 2));
            2
        }
    }
}

#[cfg(test)]
include!("sdlc_change_control_parts/020-tests.rs");
