use serde_json::{json, Value};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_CASES_PATH: &str =
    "surface/orchestration/fixtures/eval/eval_contamination_cases.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/eval_contamination_guard_current.json";
const DEFAULT_OUT_LATEST_PATH: &str = "artifacts/eval_contamination_guard_latest.json";
const DEFAULT_MARKDOWN_PATH: &str = "local/workspace/reports/EVAL_CONTAMINATION_GUARD_CURRENT.md";

pub fn run_contamination_guard(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let cases_path = parse_flag(args, "cases").unwrap_or_else(|| DEFAULT_CASES_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let out_latest_path =
        parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_OUT_LATEST_PATH.to_string());
    let markdown_path =
        parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_MARKDOWN_PATH.to_string());

    let input = read_json(&cases_path);
    let cases = input
        .get("cases")
        .and_then(|node| node.as_array())
        .cloned()
        .unwrap_or_default();
    let thresholds = input
        .get("thresholds")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let current_date = parse_string_from_path(&thresholds, &["current_date"], "2026-04-24");
    let min_cases = parse_u64_from_path(&thresholds, &["min_cases"], 4);
    let min_fresh_holdout_cases = parse_u64_from_path(&thresholds, &["min_fresh_holdout_cases"], 2);
    let max_public_exposure =
        parse_f64_from_path(&thresholds, &["max_active_public_exposure_risk"], 0.6);
    let max_training_risk = parse_f64_from_path(
        &thresholds,
        &["max_active_training_contamination_risk"],
        0.6,
    );

    let mut missing_metadata = Vec::new();
    let mut active_overexposed = Vec::new();
    let mut stale_active = Vec::new();
    let mut rotated_cases = Vec::new();
    let mut fresh_holdouts = 0_u64;
    let mut leak_matches = Vec::new();
    let mut duplicate_hashes = BTreeMap::new();
    let mut seen_hashes = BTreeMap::new();

    for case in cases.iter() {
        if !has_required_metadata(case) {
            missing_metadata.push(case_summary(case, "missing_contamination_metadata"));
        }
        let case_id = parse_string_from_path(case, &["id"], "unknown");
        let status = parse_string_from_path(case, &["status"], "active");
        let source_hash = parse_string_from_path(case, &["duplicate_source_hash"], "");
        if !source_hash.is_empty() {
            seen_hashes
                .entry(source_hash)
                .and_modify(|ids: &mut Vec<String>| ids.push(case_id.clone()))
                .or_insert_with(|| vec![case_id.clone()]);
        }

        let public_risk = parse_f64_from_path(case, &["public_exposure_risk"], 1.0);
        let training_risk = parse_f64_from_path(case, &["training_contamination_risk"], 1.0);
        let retired = status == "retired" || status == "quarantined";
        let stale = date_before(
            &parse_string_from_path(case, &["retirement_date"], ""),
            &current_date,
        );
        if retired {
            rotated_cases.push(case_summary(case, "retired_or_quarantined"));
        } else {
            if public_risk > max_public_exposure || training_risk > max_training_risk {
                active_overexposed.push(case_summary(case, "active_contamination_risk_too_high"));
            }
            if stale {
                stale_active.push(case_summary(case, "active_case_past_retirement_date"));
            }
        }
        if status == "holdout"
            && !stale
            && public_risk <= max_public_exposure
            && training_risk <= max_training_risk
        {
            fresh_holdouts = fresh_holdouts.saturating_add(1);
        }
        if has_leak_fingerprint(case) {
            leak_matches.push(case_summary(case, "leak_or_duplicate_fingerprint_match"));
        }
    }

    for (hash, ids) in seen_hashes {
        if ids.len() > 1 {
            duplicate_hashes.insert(hash, ids);
        }
    }

    let metadata_ok = cases.len() as u64 >= min_cases && missing_metadata.is_empty();
    let rotation_ok = active_overexposed.is_empty()
        && stale_active.is_empty()
        && !rotated_cases.is_empty()
        && fresh_holdouts >= min_fresh_holdout_cases;
    let leak_ok = leak_matches.is_empty() && duplicate_hashes.is_empty();
    let checks = vec![
        json!({
            "id": "eval_contamination_fixture_present",
            "ok": Path::new(&cases_path).exists(),
            "detail": cases_path,
        }),
        json!({
            "id": "eval_contamination_metadata_contract",
            "ok": metadata_ok,
            "detail": format!(
                "cases={};min_cases={};missing_metadata={}",
                cases.len(), min_cases, missing_metadata.len()
            ),
        }),
        json!({
            "id": "stale_eval_rotation_policy_contract",
            "ok": rotation_ok,
            "detail": format!(
                "rotated_cases={};stale_active={};active_overexposed={};fresh_holdouts={};min_fresh_holdouts={}",
                rotated_cases.len(), stale_active.len(), active_overexposed.len(), fresh_holdouts, min_fresh_holdout_cases
            ),
        }),
        json!({
            "id": "eval_leak_duplicate_guard_contract",
            "ok": leak_ok,
            "detail": format!(
                "leak_matches={};duplicate_source_hashes={}",
                leak_matches.len(), duplicate_hashes.len()
            ),
        }),
    ];
    let ok = checks.iter().all(|row| {
        row.get("ok")
            .and_then(|node| node.as_bool())
            .unwrap_or(false)
    });
    let report = json!({
        "type": "eval_contamination_guard",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "promotion_blocked": !ok,
        "checks": checks,
        "summary": {
            "cases": cases.len(),
            "missing_metadata": missing_metadata.len(),
            "rotated_cases": rotated_cases.len(),
            "active_overexposed": active_overexposed.len(),
            "stale_active": stale_active.len(),
            "fresh_holdouts": fresh_holdouts,
            "leak_matches": leak_matches.len(),
            "duplicate_source_hashes": duplicate_hashes.len()
        },
        "missing_metadata": missing_metadata,
        "rotated_cases": rotated_cases,
        "active_overexposed": active_overexposed,
        "stale_active": stale_active,
        "leak_matches": leak_matches,
        "duplicate_source_hashes": duplicate_hashes,
        "sources": {
            "cases": cases_path
        }
    });
    let markdown = format!(
        "# Eval Contamination Guard (Current)\n\n- generated_at: {}\n- ok: {}\n- cases: {}\n- fresh_holdouts: {}\n- rotated_cases: {}\n- promotion_blocked: {}\n",
        report.get("generated_at").and_then(|node| node.as_str()).unwrap_or(""),
        ok,
        cases.len(),
        fresh_holdouts,
        report
            .get("summary")
            .and_then(|node| node.get("rotated_cases"))
            .and_then(|node| node.as_u64())
            .unwrap_or(0),
        !ok
    );
    let write_ok = write_json(&out_latest_path, &report).is_ok()
        && write_json(&out_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write one or more contamination outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}

fn has_required_metadata(case: &Value) -> bool {
    [
        "source_date",
        "public_exposure_risk",
        "training_contamination_risk",
        "duplicate_source_hash",
        "retirement_date",
        "status",
    ]
    .iter()
    .all(|field| case.get(field).is_some())
}

fn has_leak_fingerprint(case: &Value) -> bool {
    [
        "public_benchmark_fingerprint",
        "prior_gold_patch_fingerprint",
        "checked_in_answer_key_fingerprint",
    ]
    .iter()
    .any(|field| {
        let value = parse_string_from_path(case, &[*field], "");
        !value.is_empty() && value != "none"
    })
}

fn date_before(lhs: &str, rhs: &str) -> bool {
    !lhs.is_empty() && lhs < rhs
}

fn case_summary(case: &Value, reason: &str) -> Value {
    json!({
        "id": parse_string_from_path(case, &["id"], "unknown"),
        "status": parse_string_from_path(case, &["status"], "unknown"),
        "reason": reason,
        "source_date": parse_string_from_path(case, &["source_date"], "unknown"),
        "retirement_date": parse_string_from_path(case, &["retirement_date"], "unknown"),
        "public_exposure_risk": parse_f64_from_path(case, &["public_exposure_risk"], 1.0),
        "training_contamination_risk": parse_f64_from_path(case, &["training_contamination_risk"], 1.0)
    })
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

fn parse_u64_from_path(value: &Value, path: &[&str], default: u64) -> u64 {
    value
        .pointer(&format!("/{}", path.join("/")))
        .and_then(|node| node.as_u64())
        .unwrap_or(default)
}

fn parse_f64_from_path(value: &Value, path: &[&str], default: f64) -> f64 {
    value
        .pointer(&format!("/{}", path.join("/")))
        .and_then(|node| node.as_f64())
        .unwrap_or(default)
}

fn now_iso_like() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("unix_ms:{}", millis)
}
