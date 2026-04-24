use serde_json::{json, Value};
use std::collections::BTreeSet;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_CASES_PATH: &str = "surface/orchestration/fixtures/eval/eval_metamorphic_cases.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/eval_metamorphic_guard_current.json";
const DEFAULT_OUT_LATEST_PATH: &str = "artifacts/eval_metamorphic_guard_latest.json";
const DEFAULT_MARKDOWN_PATH: &str = "local/workspace/reports/EVAL_METAMORPHIC_GUARD_CURRENT.md";

pub fn run_metamorphic_guard(args: &[String]) -> i32 {
    let strict = parse_bool_flag(args, "strict", false);
    let cases_path = parse_flag(args, "cases").unwrap_or_else(|| DEFAULT_CASES_PATH.to_string());
    let out_path = parse_flag(args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let out_latest_path =
        parse_flag(args, "out-latest").unwrap_or_else(|| DEFAULT_OUT_LATEST_PATH.to_string());
    let markdown_path =
        parse_flag(args, "out-markdown").unwrap_or_else(|| DEFAULT_MARKDOWN_PATH.to_string());

    let input = read_json(&cases_path);
    let groups = input
        .get("groups")
        .and_then(|node| node.as_array())
        .cloned()
        .unwrap_or_default();
    let thresholds = input
        .get("thresholds")
        .cloned()
        .unwrap_or_else(|| json!({}));
    let min_groups = parse_u64_from_path(&thresholds, &["min_groups"], 5);
    let min_variants_per_group = parse_u64_from_path(&thresholds, &["min_variants_per_group"], 2);
    let max_inconsistent_groups = parse_u64_from_path(&thresholds, &["max_inconsistent_groups"], 0);
    let max_brittle_failures =
        parse_u64_from_path(&thresholds, &["max_brittle_assumption_failures"], 0);

    let mut generated_variants = Vec::new();
    let mut missing_family_coverage = Vec::new();
    let mut inconsistent_groups = Vec::new();
    let mut brittle_failures = Vec::new();
    let mut total_variants = 0_u64;
    let mut perturbation_families = BTreeSet::new();

    for group in groups.iter() {
        let group_id = parse_string_from_path(group, &["id"], "unknown");
        let base_prompt = parse_string_from_path(group, &["base_prompt"], "");
        let expected = group.get("expected").cloned().unwrap_or_else(|| json!({}));
        let variants = group
            .get("variants")
            .and_then(|node| node.as_array())
            .cloned()
            .unwrap_or_default();
        total_variants = total_variants.saturating_add(variants.len() as u64);
        if variants.len() as u64 >= min_variants_per_group {
            for variant in variants.iter() {
                let family = parse_string_from_path(variant, &["perturbation_family"], "unknown");
                perturbation_families.insert(family);
                generated_variants.push(json!({
                    "group_id": group_id,
                    "variant_id": parse_string_from_path(variant, &["id"], "unknown"),
                    "prompt": format!("{} {}", base_prompt, parse_string_from_path(variant, &["prompt_suffix"], "")),
                    "perturbation_family": parse_string_from_path(variant, &["perturbation_family"], "unknown")
                }));
                if !variant_matches_expected(variant, &expected) {
                    inconsistent_groups.push(variant_summary(
                        group,
                        variant,
                        "metamorphic_behavior_mismatch",
                    ));
                }
                if brittle_assumption_failure(variant) {
                    brittle_failures.push(variant_summary(
                        group,
                        variant,
                        "brittle_route_or_schema_assumption",
                    ));
                }
            }
        }
    }

    for required in [
        "local_file",
        "web",
        "policy_denied",
        "empty_result",
        "frustrated_followup",
        "tool_list",
        "path_style",
        "ordering",
        "missing_capability",
    ] {
        if !perturbation_families.contains(required) {
            missing_family_coverage.push(required.to_string());
        }
    }

    let generator_ok = Path::new(&cases_path).exists()
        && groups.len() as u64 >= min_groups
        && total_variants >= min_groups.saturating_mul(min_variants_per_group)
        && missing_family_coverage.is_empty();
    let consistency_ok = inconsistent_groups.len() as u64 <= max_inconsistent_groups;
    let brittle_ok = brittle_failures.len() as u64 <= max_brittle_failures;
    let checks = vec![
        json!({
            "id": "metamorphic_prompt_perturbation_generator_contract",
            "ok": generator_ok,
            "detail": format!(
                "groups={};min_groups={};generated_variants={};missing_family_coverage={}",
                groups.len(), min_groups, generated_variants.len(), missing_family_coverage.len()
            ),
        }),
        json!({
            "id": "metamorphic_consistency_scoring_contract",
            "ok": consistency_ok,
            "detail": format!(
                "inconsistent_groups={};max_inconsistent_groups={}",
                inconsistent_groups.len(), max_inconsistent_groups
            ),
        }),
        json!({
            "id": "metamorphic_brittle_assumption_guard_contract",
            "ok": brittle_ok,
            "detail": format!(
                "brittle_failures={};max_brittle_failures={}",
                brittle_failures.len(), max_brittle_failures
            ),
        }),
    ];
    let ok = checks.iter().all(|row| {
        row.get("ok")
            .and_then(|node| node.as_bool())
            .unwrap_or(false)
    });
    let report = json!({
        "type": "eval_metamorphic_guard",
        "schema_version": 1,
        "generated_at": now_iso_like(),
        "ok": ok,
        "checks": checks,
        "summary": {
            "groups": groups.len(),
            "generated_variants": generated_variants.len(),
            "perturbation_families": perturbation_families.len(),
            "missing_family_coverage": missing_family_coverage.len(),
            "inconsistent_groups": inconsistent_groups.len(),
            "brittle_failures": brittle_failures.len()
        },
        "generated_prompt_variants": generated_variants,
        "missing_family_coverage": missing_family_coverage,
        "inconsistent_groups": inconsistent_groups,
        "brittle_failures": brittle_failures,
        "sources": {
            "cases": cases_path
        }
    });
    let markdown = format!(
        "# Eval Metamorphic Guard (Current)\n\n- generated_at: {}\n- ok: {}\n- groups: {}\n- generated_variants: {}\n- inconsistent_groups: {}\n- brittle_failures: {}\n",
        report.get("generated_at").and_then(|node| node.as_str()).unwrap_or(""),
        ok,
        groups.len(),
        report.pointer("/summary/generated_variants").and_then(|node| node.as_u64()).unwrap_or(0),
        inconsistent_groups.len(),
        brittle_failures.len()
    );
    let write_ok = write_json(&out_latest_path, &report).is_ok()
        && write_json(&out_path, &report).is_ok()
        && write_text(&markdown_path, &markdown).is_ok();
    if !write_ok {
        eprintln!("eval_runtime: failed to write one or more metamorphic outputs");
        return 2;
    }
    print_structured(&report);
    if strict && !ok {
        return 1;
    }
    0
}

fn variant_matches_expected(variant: &Value, expected: &Value) -> bool {
    let predicted = variant.get("predicted").unwrap_or(&Value::Null);
    ["route", "workflow", "tool_family", "final_answer_class"]
        .iter()
        .all(|field| {
            parse_string_from_path(predicted, &[*field], "")
                == parse_string_from_path(expected, &[*field], "")
        })
}

fn brittle_assumption_failure(variant: &Value) -> bool {
    let predicted = variant.get("predicted").unwrap_or(&Value::Null);
    parse_bool_from_path(predicted, &["schema_error"], false)
        || parse_bool_from_path(predicted, &["tool_list_order_sensitive"], false)
        || parse_bool_from_path(predicted, &["path_style_sensitive"], false)
        || parse_bool_from_path(predicted, &["missing_capability_unhandled"], false)
}

fn variant_summary(group: &Value, variant: &Value, reason: &str) -> Value {
    json!({
        "group_id": parse_string_from_path(group, &["id"], "unknown"),
        "variant_id": parse_string_from_path(variant, &["id"], "unknown"),
        "perturbation_family": parse_string_from_path(variant, &["perturbation_family"], "unknown"),
        "reason": reason,
        "expected": group.get("expected").cloned().unwrap_or_else(|| json!({})),
        "predicted": variant.get("predicted").cloned().unwrap_or_else(|| json!({}))
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

fn parse_bool_from_path(value: &Value, path: &[&str], default: bool) -> bool {
    value
        .pointer(&format!("/{}", path.join("/")))
        .and_then(|node| node.as_bool())
        .unwrap_or(default)
}

fn parse_u64_from_path(value: &Value, path: &[&str], default: u64) -> u64 {
    value
        .pointer(&format!("/{}", path.join("/")))
        .and_then(|node| node.as_u64())
        .unwrap_or(default)
}

fn now_iso_like() -> String {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0);
    format!("unix_ms:{}", millis)
}
