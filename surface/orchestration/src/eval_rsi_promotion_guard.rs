use serde_json::{json, Value};
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_POLICY_PATH: &str =
    "surface/orchestration/fixtures/eval/eval_rsi_promotion_ladder.json";
const DEFAULT_REDTEAM_PATH: &str =
    "surface/orchestration/fixtures/eval/eval_holdout_red_team_cases.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/eval_rsi_promotion_ladder_current.json";
const DEFAULT_OUT_LATEST_PATH: &str = "artifacts/eval_rsi_promotion_ladder_latest.json";
const DEFAULT_MARKDOWN_PATH: &str = "local/workspace/reports/EVAL_RSI_PROMOTION_LADDER_CURRENT.md";

pub fn run_rsi_promotion_ladder_guard(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let policy_path = parse_flag(args, "policy").unwrap_or_else(|| DEFAULT_POLICY_PATH.to_string());
    let redteam_path =
        parse_flag(args, "redteam").unwrap_or_else(|| DEFAULT_REDTEAM_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let out_latest_path =
        parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_OUT_LATEST_PATH.to_string());
    let markdown_path =
        parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_MARKDOWN_PATH.to_string());

    let policy = read_json(&policy_path);
    let redteam = read_json(&redteam_path);
    let proof_rows = policy
        .get("required_proofs")
        .and_then(|node| node.as_array())
        .cloned()
        .unwrap_or_default();
    let redteam_cases = redteam
        .get("cases")
        .and_then(|node| node.as_array())
        .cloned()
        .unwrap_or_default();

    let proof_statuses: Vec<Value> = proof_rows.iter().map(evaluate_proof).collect();
    let blocked_proofs: Vec<Value> = proof_statuses
        .iter()
        .filter(|row| !parse_bool_from_path(row, &["passing"], false))
        .cloned()
        .collect();
    let redteam_failures: Vec<Value> = redteam_cases
        .iter()
        .filter(|case| redteam_case_failed(case))
        .map(|case| case_summary(case, "holdout_redteam_case_accepted_or_unsupported"))
        .collect();
    let required_failure_classes = [
        "hallucinated_issue",
        "wrong_tool_claim",
        "unsupported_root_cause",
        "non_actionable_recommendation",
    ];
    let missing_redteam_classes: Vec<String> = required_failure_classes
        .iter()
        .filter(|class| {
            !redteam_cases.iter().any(|case| {
                parse_string_from_path(case, &["failure_class"], "") == **class
                    && parse_bool_from_path(case, &["holdout"], false)
            })
        })
        .map(|class| class.to_string())
        .collect();

    let promotion_allowed = blocked_proofs.is_empty()
        && redteam_failures.is_empty()
        && missing_redteam_classes.is_empty();
    let policy_ok = Path::new(&policy_path).exists()
        && proof_rows.len() >= 5
        && !parse_string_from_path(&policy, &["policy", "promotion_decision"], "").is_empty();
    let runtime_block_ok = if blocked_proofs.is_empty() {
        promotion_allowed
    } else {
        !promotion_allowed
    };
    let redteam_ok = Path::new(&redteam_path).exists()
        && redteam_failures.is_empty()
        && missing_redteam_classes.is_empty();
    let checks = vec![
        json!({
            "id": "rsi_eval_promotion_ladder_policy_contract",
            "ok": policy_ok,
            "detail": format!("required_proofs={};policy_path={}", proof_rows.len(), policy_path),
        }),
        json!({
            "id": "rsi_eval_runtime_promotion_block_contract",
            "ok": runtime_block_ok,
            "detail": format!(
                "promotion_allowed={};blocked_proofs={}",
                promotion_allowed, blocked_proofs.len()
            ),
        }),
        json!({
            "id": "eval_holdout_redteam_suite_contract",
            "ok": redteam_ok,
            "detail": format!(
                "holdout_cases={};redteam_failures={};missing_failure_classes={}",
                redteam_cases.len(), redteam_failures.len(), missing_redteam_classes.len()
            ),
        }),
    ];
    let ok = checks.iter().all(|row| {
        row.get("ok")
            .and_then(|node| node.as_bool())
            .unwrap_or(false)
    });
    let report = json!({
        "type": "eval_rsi_promotion_ladder_guard",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "promotion_allowed": promotion_allowed,
        "checks": checks,
        "summary": {
            "required_proofs": proof_rows.len(),
            "blocked_proofs": blocked_proofs.len(),
            "holdout_redteam_cases": redteam_cases.len(),
            "redteam_failures": redteam_failures.len(),
            "missing_redteam_failure_classes": missing_redteam_classes.len()
        },
        "proof_statuses": proof_statuses,
        "blocked_proofs": blocked_proofs,
        "redteam_failures": redteam_failures,
        "missing_redteam_failure_classes": missing_redteam_classes,
        "sources": {
            "policy": policy_path,
            "redteam": redteam_path
        }
    });
    let markdown = format!(
        "# Eval RSI Promotion Ladder (Current)\n\n- generated_at: {}\n- ok: {}\n- promotion_allowed: {}\n- required_proofs: {}\n- blocked_proofs: {}\n- holdout_redteam_cases: {}\n- redteam_failures: {}\n",
        report.get("generated_at").and_then(|node| node.as_str()).unwrap_or(""),
        ok,
        promotion_allowed,
        proof_rows.len(),
        report.pointer("/summary/blocked_proofs").and_then(|node| node.as_u64()).unwrap_or(0),
        redteam_cases.len(),
        report.pointer("/summary/redteam_failures").and_then(|node| node.as_u64()).unwrap_or(0)
    );
    let write_ok = write_json(&out_latest_path, &report).is_ok()
        && write_json(&out_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write one or more RSI promotion ladder outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}

fn evaluate_proof(proof: &Value) -> Value {
    let id = parse_string_from_path(proof, &["id"], "unknown");
    let artifact_path = parse_string_from_path(proof, &["artifact_path"], "");
    let artifact = read_json(&artifact_path);
    let exists = Path::new(&artifact_path).exists();
    let ok_pointer = parse_string_from_path(proof, &["ok_pointer"], "/ok");
    let block_pointer = parse_string_from_path(proof, &["block_if_true_pointer"], "");
    let artifact_ok = proof_pointer_ok(&artifact, &ok_pointer);
    let blocked_by_pointer = if block_pointer.is_empty() {
        false
    } else {
        bool_pointer(&artifact, &block_pointer, false)
    };
    let passing = exists && artifact_ok && !blocked_by_pointer;
    json!({
        "id": id,
        "artifact_path": artifact_path,
        "exists": exists,
        "artifact_ok": artifact_ok,
        "blocked_by_pointer": blocked_by_pointer,
        "passing": passing
    })
}

fn redteam_case_failed(case: &Value) -> bool {
    let expected_action = parse_string_from_path(case, &["expected_action"], "reject");
    let predicted_action = parse_string_from_path(case, &["predicted", "action"], "accept");
    let issue_ready = parse_bool_from_path(case, &["predicted", "issue_ready"], true);
    expected_action == "reject" && (predicted_action != "reject" || issue_ready)
}

fn case_summary(case: &Value, reason: &str) -> Value {
    json!({
        "id": parse_string_from_path(case, &["id"], "unknown"),
        "failure_class": parse_string_from_path(case, &["failure_class"], "unknown"),
        "reason": reason,
        "expected_action": parse_string_from_path(case, &["expected_action"], "reject"),
        "predicted": case.get("predicted").cloned().unwrap_or_else(|| json!({}))
    })
}

fn bool_pointer(value: &Value, pointer: &str, default: bool) -> bool {
    value
        .pointer(pointer)
        .and_then(|node| node.as_bool())
        .unwrap_or(default)
}

fn proof_pointer_ok(value: &Value, pointer: &str) -> bool {
    value
        .pointer(pointer)
        .map(|node| match node {
            Value::Bool(flag) => *flag,
            Value::Null => false,
            Value::Array(rows) => !rows.is_empty(),
            Value::Object(map) => !map.is_empty(),
            Value::String(text) => !text.is_empty(),
            Value::Number(_) => true,
        })
        .unwrap_or(false)
}

fn parse_flag(args: &[String], name: &str) -> Option<String> {
    let prefix = format!("--{}=", name);
    args.iter()
        .find_map(|arg| arg.strip_prefix(&prefix).map(|value| value.to_string()))
}

fn parse_bool_flag(args: &[String], name: &str, default: bool) -> bool {
    parse_flag(args, name)
        .and_then(|value| match value.as_str() {
            "1" | "true" | "yes" => Some(true),
            "0" | "false" | "no" => Some(false),
            _ => None,
        })
        .unwrap_or(default)
}

fn read_json(path: &str) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|content| serde_json::from_str(&content).ok())
        .unwrap_or_else(|| json!({}))
}

fn write_json(path: &str, value: &Value) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    let content = serde_json::to_string_pretty(value)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err))?;
    fs::write(path, format!("{}\n", content))
}

fn write_text(path: &str, content: &str) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, content)
}

fn print_structured(value: &Value) {
    match serde_json::to_string(value) {
        Ok(content) => {
            let _ = writeln!(io::stdout(), "{}", content);
        }
        Err(err) => {
            let _ = writeln!(io::stderr(), "failed to serialize report: {}", err);
        }
    }
}

fn parse_string_from_path(value: &Value, path: &[&str], default: &str) -> String {
    value
        .pointer(&format!("/{}", path.join("/")))
        .and_then(|node| node.as_str())
        .unwrap_or(default)
        .to_string()
}

fn parse_bool_from_path(value: &Value, path: &[&str], default: bool) -> bool {
    value
        .pointer(&format!("/{}", path.join("/")))
        .and_then(|node| node.as_bool())
        .unwrap_or(default)
}

fn now_iso_like() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("unix_ms:{}", millis)
}
