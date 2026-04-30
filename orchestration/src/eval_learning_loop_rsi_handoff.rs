use serde_json::{json, Value};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_VERSION_PATH: &str = "artifacts/eval_learning_loop_policy_version_latest.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/eval_learning_loop_rsi_handoff_current.json";
const DEFAULT_OUT_LATEST_PATH: &str = "artifacts/eval_learning_loop_rsi_handoff_latest.json";
const DEFAULT_MARKDOWN_PATH: &str =
    "local/workspace/reports/EVAL_LEARNING_LOOP_RSI_HANDOFF_CURRENT.md";

pub fn run_eval_learning_loop_rsi_handoff(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let version_path =
        parse_flag(args, "version").unwrap_or_else(|| DEFAULT_VERSION_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let out_latest_path =
        parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_OUT_LATEST_PATH.to_string());
    let markdown_path =
        parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_MARKDOWN_PATH.to_string());
    let version_report = read_json(&version_path);
    let handoff = rsi_handoff_record(&version_report, &version_path);
    let boundary_ok = bool_at(&handoff, &["proposal_allowed"])
        && !bool_at(&handoff, &["auto_apply_allowed"])
        && bool_at(&handoff, &["required_gates", "normal_tests"])
        && bool_at(&handoff, &["required_gates", "proof_gates"])
        && bool_at(&handoff, &["required_gates", "human_approval"])
        && bool_at(&handoff, &["required_gates", "rollback_readiness"]);
    let evidence_ok = str_at(&handoff, &["policy_version_artifact"]).is_some()
        && str_at(&handoff, &["candidate_policy_id"]).is_some()
        && str_at(&handoff, &["parent_policy_id"]).is_some();
    let ok = boundary_ok && evidence_ok;
    let report = json!({
        "type": "eval_learning_loop_rsi_handoff",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "checks": [
            {"id": "eval_rsi_handoff_boundary_contract", "ok": boundary_ok, "detail": "proposal allowed only after policy version promotion; auto-apply remains disabled"},
            {"id": "eval_rsi_handoff_evidence_contract", "ok": evidence_ok, "detail": "candidate/parent policy and version artifact are present"},
            {"id": "eval_rsi_handoff_required_gates_contract", "ok": boundary_ok, "detail": "normal tests, proof gates, human approval, and rollback readiness are mandatory"}
        ],
        "summary": {
            "proposal_allowed": bool_at(&handoff, &["proposal_allowed"]),
            "auto_apply_allowed": bool_at(&handoff, &["auto_apply_allowed"]),
            "human_approval_required": bool_at(&handoff, &["required_gates", "human_approval"]),
            "rollback_readiness_required": bool_at(&handoff, &["required_gates", "rollback_readiness"])
        },
        "handoff": handoff,
        "sources": {"version": version_path}
    });
    let markdown = format!(
        "# Eval Learning Loop RSI Handoff (Current)\n\n- generated_at: {}\n- ok: {}\n- proposal_allowed: {}\n- auto_apply_allowed: {}\n- human_approval_required: {}\n- rollback_readiness_required: {}\n",
        report.get("generated_at").and_then(|node| node.as_str()).unwrap_or(""),
        ok,
        bool_at(&report, &["summary", "proposal_allowed"]),
        bool_at(&report, &["summary", "auto_apply_allowed"]),
        bool_at(&report, &["summary", "human_approval_required"]),
        bool_at(&report, &["summary", "rollback_readiness_required"])
    );
    let write_ok = write_json(&out_path, &report).is_ok()
        && write_json(&out_latest_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write eval learning-loop RSI handoff outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}

fn rsi_handoff_record(version_report: &Value, version_path: &str) -> Value {
    let version = version_report
        .get("policy_version")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let promoted = bool_at(version_report, &["ok"])
        && str_at(&version, &["reviewer_decision"]) == Some("approved_for_policy_promotion");
    json!({
        "type": "eval_learning_loop_rsi_handoff_record",
        "schema_version": 1,
        "proposal_allowed": promoted,
        "auto_apply_allowed": false,
        "candidate_policy_id": str_at(&version, &["candidate_policy_id"]).unwrap_or("unknown"),
        "parent_policy_id": str_at(&version, &["parent_policy_id"]).unwrap_or("unknown"),
        "policy_version_artifact": version_path,
        "proposal_scope": if promoted { "eval_policy_patch_proposal_only" } else { "blocked" },
        "blocked_actions": [
            "runtime_code_auto_apply",
            "policy_auto_promote_without_human_review",
            "proof_gate_bypass",
            "rollback_bypass"
        ],
        "required_gates": {
            "normal_tests": true,
            "proof_gates": true,
            "human_approval": true,
            "rollback_readiness": bool_at(&version, &["rollback", "safe_to_revert_without_runtime_code"]),
            "policy_version_evidence": true
        },
        "handoff_reason": if promoted {
            "policy version promotion evidence exists; RSI may draft a patch proposal but cannot apply it"
        } else {
            "policy promotion evidence missing; RSI patch proposal boundary is closed"
        },
        "generated_at": now_iso_like()
    })
}

fn now_iso_like() -> String {
    let ms = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis())
        .unwrap_or(0);
    format!("unix_ms:{ms}")
}

fn parse_flag(args: &[String], key: &str) -> Option<String> {
    let inline_prefix = format!("--{key}=");
    for (idx, arg) in args.iter().enumerate() {
        if let Some(value) = arg.strip_prefix(&inline_prefix) {
            return Some(value.to_string());
        }
        if arg == &format!("--{key}") {
            if let Some(value) = args.get(idx + 1) {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn parse_bool_flag(args: &[String], key: &str, default: bool) -> bool {
    match parse_flag(args, key).as_deref() {
        Some("1" | "true" | "TRUE" | "yes" | "on") => true,
        Some("0" | "false" | "FALSE" | "no" | "off") => false,
        _ => default,
    }
}

fn str_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut cursor = value;
    for segment in path {
        cursor = cursor.get(*segment)?;
    }
    cursor
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
}

fn bool_at(value: &Value, path: &[&str]) -> bool {
    let mut cursor = value;
    for segment in path {
        let Some(next) = cursor.get(*segment) else {
            return false;
        };
        cursor = next;
    }
    cursor.as_bool().unwrap_or(false)
}

fn read_json(path: &str) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| json!({}))
}

fn ensure_parent(path: &str) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn write_json(path: &str, value: &Value) -> io::Result<()> {
    ensure_parent(path)?;
    let payload = serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string());
    fs::write(path, format!("{payload}\n"))
}

fn write_text(path: &str, value: &str) -> io::Result<()> {
    ensure_parent(path)?;
    fs::write(path, value)
}

fn print_structured(report: &Value) {
    if let Ok(serialized) = serde_json::to_string(report) {
        let _ = writeln!(io::stdout(), "{serialized}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rsi_handoff_blocks_auto_apply_even_when_proposal_allowed() {
        let version_report = json!({
            "ok": true,
            "policy_version": {
                "candidate_policy_id": "candidate",
                "parent_policy_id": "parent",
                "reviewer_decision": "approved_for_policy_promotion",
                "rollback": {"safe_to_revert_without_runtime_code": true}
            }
        });
        let handoff = rsi_handoff_record(&version_report, "version.json");
        assert!(bool_at(&handoff, &["proposal_allowed"]));
        assert!(!bool_at(&handoff, &["auto_apply_allowed"]));
        assert!(bool_at(&handoff, &["required_gates", "human_approval"]));
    }
}
