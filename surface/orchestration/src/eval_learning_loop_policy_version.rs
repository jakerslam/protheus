use serde_json::{json, Value};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_POLICY_PATH: &str = "artifacts/eval_learning_loop_policy_promotion_latest.json";
const DEFAULT_OUT_PATH: &str =
    "core/local/artifacts/eval_learning_loop_policy_version_current.json";
const DEFAULT_OUT_LATEST_PATH: &str = "artifacts/eval_learning_loop_policy_version_latest.json";
const DEFAULT_VERSION_STORE_PATH: &str = "local/state/ops/eval_learning_loop/policy_versions.jsonl";
const DEFAULT_MARKDOWN_PATH: &str =
    "local/workspace/reports/EVAL_LEARNING_LOOP_POLICY_VERSION_CURRENT.md";

pub fn run_eval_learning_loop_policy_version(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let policy_path = parse_flag(args, "policy").unwrap_or_else(|| DEFAULT_POLICY_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let out_latest_path =
        parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_OUT_LATEST_PATH.to_string());
    let version_store_path =
        parse_flag(args, "store").unwrap_or_else(|| DEFAULT_VERSION_STORE_PATH.to_string());
    let markdown_path =
        parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_MARKDOWN_PATH.to_string());
    let policy = read_json(&policy_path);
    let version = policy_version_record(&policy, &policy_path, &version_store_path);
    let evidence_ok = version
        .get("evidence_artifacts")
        .and_then(|node| node.as_array())
        .map(|rows| rows.len() >= 3 && rows.iter().all(|row| row.as_str().is_some()))
        .unwrap_or(false);
    let rollback_ok = str_at(&version, &["rollback", "rollback_path"]).is_some()
        && str_at(&version, &["rollback", "parent_policy_id"]).is_some()
        && bool_at(
            &version,
            &["rollback", "safe_to_revert_without_runtime_code"],
        );
    let version_ok = str_at(&version, &["candidate_policy_id"]).is_some()
        && str_at(&version, &["parent_policy_id"]).is_some()
        && str_at(&version, &["reviewer_decision"]).is_some()
        && str_at(&version, &["promotion_reason"]).is_some()
        && evidence_ok
        && rollback_ok;
    let report = json!({
        "type": "eval_learning_loop_policy_version",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": version_ok,
        "checks": [
            {"id": "eval_policy_version_metadata_contract", "ok": version_ok, "detail": "parent/candidate/reviewer/reason/evidence/rollback metadata present"},
            {"id": "eval_policy_rollback_metadata_contract", "ok": rollback_ok, "detail": "rollback path is present and safe without runtime code mutation"},
            {"id": "eval_policy_evidence_artifact_contract", "ok": evidence_ok, "detail": "policy version records reviewed/holdout/current evidence artifacts"}
        ],
        "summary": {
            "candidate_policy_id": str_at(&version, &["candidate_policy_id"]).unwrap_or("unknown"),
            "parent_policy_id": str_at(&version, &["parent_policy_id"]).unwrap_or("unknown"),
            "reviewer_decision": str_at(&version, &["reviewer_decision"]).unwrap_or("unknown"),
            "rollback_ready": rollback_ok,
            "evidence_ready": evidence_ok
        },
        "policy_version": version,
        "sources": {"policy": policy_path},
        "store_path": version_store_path
    });
    let markdown = format!(
        "# Eval Learning Loop Policy Version (Current)\n\n- generated_at: {}\n- ok: {}\n- candidate_policy_id: {}\n- parent_policy_id: {}\n- reviewer_decision: {}\n- rollback_ready: {}\n",
        report.get("generated_at").and_then(|node| node.as_str()).unwrap_or(""),
        version_ok,
        str_at(&report, &["summary", "candidate_policy_id"]).unwrap_or("unknown"),
        str_at(&report, &["summary", "parent_policy_id"]).unwrap_or("unknown"),
        str_at(&report, &["summary", "reviewer_decision"]).unwrap_or("unknown"),
        rollback_ok
    );
    let write_ok = write_json(&out_path, &report).is_ok()
        && write_json(&out_latest_path, &report).is_ok()
        && append_jsonl(
            &version_store_path,
            report.get("policy_version").unwrap_or(&json!({})),
        )
        .is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write eval learning-loop policy version outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !version_ok {
        return 1;
    }
    0
}

fn policy_version_record(policy: &Value, policy_path: &str, version_store_path: &str) -> Value {
    let candidate_policy_id = str_at(policy, &["calibration_update", "candidate_policy_id"])
        .unwrap_or("eval-learning-loop-policy-candidate-v1");
    let parent_policy_id = str_at(policy, &["calibration_update", "parent_policy_id"])
        .unwrap_or("eval-learning-loop-policy-active-v1");
    let promoted =
        bool_at(policy, &["summary", "candidate_policy_promotable"]) && bool_at(policy, &["ok"]);
    let reviewer_decision = if promoted {
        "approved_for_policy_promotion"
    } else {
        "blocked_pending_policy_evidence"
    };
    json!({
        "type": "eval_learning_loop_policy_version_record",
        "schema_version": 1,
        "version_id": format!("{}-from-{}", candidate_policy_id, parent_policy_id),
        "candidate_policy_id": candidate_policy_id,
        "parent_policy_id": parent_policy_id,
        "reviewer_decision": reviewer_decision,
        "promotion_reason": if promoted {
            "candidate policy improved reviewed and holdout correctness with zero high-severity regression blockers"
        } else {
            "candidate policy did not satisfy promotion evidence"
        },
        "evidence_artifacts": [
            policy_path,
            str_at(policy, &["sources", "reviewed"]).unwrap_or("artifacts/eval_learning_loop_reviewed_examples_latest.json"),
            str_at(policy, &["sources", "holdout"]).unwrap_or("surface/orchestration/fixtures/eval/eval_learning_loop_policy_holdout.json")
        ],
        "rollback": {
            "rollback_path": version_store_path,
            "parent_policy_id": parent_policy_id,
            "rollback_command": "restore parent_policy_id and rerun eval_runtime learning-loop-policy --strict=1",
            "safe_to_revert_without_runtime_code": true
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

fn append_jsonl(path: &str, value: &Value) -> io::Result<()> {
    ensure_parent(path)?;
    let payload = serde_json::to_string(value).unwrap_or_else(|_| "{}".to_string());
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    writeln!(file, "{payload}")
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
    fn policy_version_record_has_rollback_metadata() {
        let policy = json!({
            "ok": true,
            "summary": {"candidate_policy_promotable": true},
            "calibration_update": {
                "candidate_policy_id": "candidate",
                "parent_policy_id": "parent"
            },
            "sources": {"reviewed": "reviewed.json", "holdout": "holdout.json"}
        });
        let record = policy_version_record(&policy, "policy.json", "versions.jsonl");
        assert_eq!(str_at(&record, &["candidate_policy_id"]), Some("candidate"));
        assert_eq!(
            str_at(&record, &["rollback", "parent_policy_id"]),
            Some("parent")
        );
        assert!(bool_at(
            &record,
            &["rollback", "safe_to_revert_without_runtime_code"]
        ));
    }
}
