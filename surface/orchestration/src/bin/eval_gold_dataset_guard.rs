use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

const DEFAULT_DATASET_PATH: &str = "surface/orchestration/fixtures/eval/eval_gold_dataset_v1.jsonl";
const DEFAULT_TAXONOMY_PATH: &str = "surface/orchestration/fixtures/eval/eval_issue_taxonomy_v1.json";
const DEFAULT_OUT_PATH: &str = "core/local/artifacts/eval_gold_dataset_schema_guard_current.json";
const DEFAULT_OUT_LATEST_PATH: &str = "artifacts/eval_gold_dataset_schema_guard_latest.json";

#[derive(Debug, Default)]
struct ClassCoverage {
    positive: usize,
    negative: usize,
}

fn parse_flag(args: &[String], key: &str) -> Option<String> {
    let inline = format!("--{key}=");
    for (idx, arg) in args.iter().enumerate() {
        if let Some(value) = arg.strip_prefix(&inline) {
            return Some(value.to_string());
        }
        if arg == &format!("--{key}") {
            return args.get(idx + 1).cloned();
        }
    }
    None
}

fn now_ms() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis())
        .unwrap_or(0)
}

fn ensure_parent(path: &str) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn read_json(path: &str) -> io::Result<Value> {
    let raw = fs::read_to_string(path)?;
    serde_json::from_str::<Value>(&raw)
        .map_err(|err| io::Error::new(io::ErrorKind::InvalidData, err.to_string()))
}

fn read_jsonl(path: &str) -> io::Result<Vec<Value>> {
    let raw = fs::read_to_string(path)?;
    let mut rows = Vec::new();
    for (idx, line) in raw.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let row = serde_json::from_str::<Value>(trimmed).map_err(|err| {
            io::Error::new(
                io::ErrorKind::InvalidData,
                format!("jsonl row {}: {}", idx + 1, err),
            )
        })?;
        rows.push(row);
    }
    Ok(rows)
}

fn str_field<'a>(row: &'a Value, key: &str) -> &'a str {
    row.get(key).and_then(Value::as_str).unwrap_or("")
}

fn labels(row: &Value) -> Option<&Value> {
    row.get("labels").filter(|value| value.is_object())
}

fn contains_secret_like_text(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    lower.contains("github_pat_")
        || lower.contains("ghp_")
        || lower.contains("xoxb-")
        || lower.contains("xoxp-")
        || lower.contains("sk-")
        || lower.contains("http://")
        || lower.contains("https://")
}

fn critical_classes(taxonomy: &Value) -> BTreeSet<String> {
    taxonomy
        .get("classes")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter(|row| row.get("critical").and_then(Value::as_bool).unwrap_or(false))
        .filter_map(|row| row.get("id").and_then(Value::as_str))
        .map(ToString::to_string)
        .collect()
}

fn severity_values(taxonomy: &Value) -> BTreeSet<String> {
    taxonomy
        .get("severity_scale")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(ToString::to_string)
        .collect()
}

fn validate_rows(
    rows: &[Value],
    critical: &BTreeSet<String>,
    severities: &BTreeSet<String>,
) -> (Vec<Value>, BTreeMap<String, ClassCoverage>) {
    let mut failures = Vec::new();
    let mut coverage = BTreeMap::<String, ClassCoverage>::new();
    let mut ids = BTreeSet::new();

    for (idx, row) in rows.iter().enumerate() {
        let row_num = idx + 1;
        for field in ["id", "source_event_id", "ts", "prompt", "assistant_text"] {
            if str_field(row, field).trim().is_empty() {
                failures.push(json!({"row": row_num, "reason": format!("missing_or_empty:{field}")}));
            }
        }
        let id = str_field(row, "id").to_string();
        if !id.is_empty() && !ids.insert(id.clone()) {
            failures.push(json!({"row": row_num, "reason": format!("duplicate_id:{id}")}));
        }
        for field in ["prompt", "assistant_text"] {
            if contains_secret_like_text(str_field(row, field)) {
                failures.push(json!({"row": row_num, "reason": format!("sensitive_or_unredacted_content:{field}")}));
            }
        }
        let Some(label_value) = labels(row) else {
            failures.push(json!({"row": row_num, "reason": "missing_labels"}));
            continue;
        };
        let issue_class = str_field(label_value, "issue_class").to_string();
        let severity = str_field(label_value, "severity").to_string();
        let is_failure = label_value.get("is_failure").and_then(Value::as_bool);
        let expected_fix = str_field(label_value, "expected_fix");
        if !critical.contains(&issue_class) {
            failures.push(json!({"row": row_num, "reason": format!("unknown_or_noncritical_issue_class:{issue_class}")}));
        }
        if !severities.contains(&severity) {
            failures.push(json!({"row": row_num, "reason": format!("invalid_severity:{severity}")}));
        }
        if expected_fix.trim().len() < 12 {
            failures.push(json!({"row": row_num, "reason": "expected_fix_too_short"}));
        }
        match is_failure {
            Some(true) => coverage.entry(issue_class).or_default().positive += 1,
            Some(false) => coverage.entry(issue_class).or_default().negative += 1,
            None => failures.push(json!({"row": row_num, "reason": "is_failure_not_boolean"})),
        }
    }

    for class_id in critical {
        let entry = coverage.entry(class_id.clone()).or_default();
        if entry.positive == 0 {
            failures.push(json!({"row": 0, "reason": format!("missing_positive_example:{class_id}")}));
        }
        if entry.negative == 0 {
            failures.push(json!({"row": 0, "reason": format!("missing_negative_example:{class_id}")}));
        }
    }

    (failures, coverage)
}

fn run() -> io::Result<(bool, Value)> {
    let args: Vec<String> = env::args().skip(1).collect();
    let dataset_path = parse_flag(&args, "dataset").unwrap_or_else(|| DEFAULT_DATASET_PATH.to_string());
    let taxonomy_path = parse_flag(&args, "taxonomy").unwrap_or_else(|| DEFAULT_TAXONOMY_PATH.to_string());
    let out_path = parse_flag(&args, "out").unwrap_or_else(|| DEFAULT_OUT_PATH.to_string());
    let latest_path = parse_flag(&args, "out-latest").unwrap_or_else(|| DEFAULT_OUT_LATEST_PATH.to_string());

    let taxonomy = read_json(&taxonomy_path)?;
    let rows = read_jsonl(&dataset_path)?;
    let critical = critical_classes(&taxonomy);
    let severities = severity_values(&taxonomy);
    let (failures, coverage) = validate_rows(&rows, &critical, &severities);
    let coverage_rows = coverage
        .iter()
        .map(|(class_id, row)| {
            json!({
                "issue_class": class_id,
                "positive_examples": row.positive,
                "negative_examples": row.negative,
                "ok": row.positive > 0 && row.negative > 0
            })
        })
        .collect::<Vec<_>>();
    let ok = failures.is_empty() && !rows.is_empty() && !critical.is_empty();
    let report = json!({
        "type": "eval_gold_dataset_schema_guard",
        "schema_version": 1,
        "generated_at_ms": now_ms(),
        "ok": ok,
        "owner": "surface_orchestration_control_plane",
        "dataset_path": dataset_path,
        "taxonomy_path": taxonomy_path,
        "summary": {
            "dataset_rows": rows.len(),
            "required_critical_classes": critical.len(),
            "classes_with_positive_and_negative": coverage_rows.iter().filter(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false)).count(),
            "failure_count": failures.len()
        },
        "coverage": coverage_rows,
        "failures": failures
    });
    ensure_parent(&out_path)?;
    ensure_parent(&latest_path)?;
    let payload = format!("{}\n", serde_json::to_string_pretty(&report).unwrap_or_default());
    fs::write(&out_path, &payload)?;
    fs::write(&latest_path, payload)?;
    Ok((ok, report))
}

fn main() -> ExitCode {
    match run() {
        Ok((ok, report)) => {
            let _ = writeln!(io::stdout(), "{}", serde_json::to_string(&report).unwrap_or_default());
            if ok { ExitCode::SUCCESS } else { ExitCode::from(1) }
        }
        Err(err) => {
            let _ = writeln!(io::stderr(), "eval gold dataset guard failed: {err}");
            ExitCode::from(1)
        }
    }
}
