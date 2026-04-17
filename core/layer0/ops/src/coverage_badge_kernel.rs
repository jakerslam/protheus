// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_TS_PATH: &str = "coverage/ts/coverage-summary.json";
const DEFAULT_RUST_PATH: &str = "coverage/rust-summary.txt";
const DEFAULT_CONTRACTS_PATH: &str = "local/state/ops/web_tooling/runtime_snapshot/latest.json";
const DEFAULT_OUT_JSON: &str = "coverage/combined-summary.json";
const DEFAULT_OUT_BADGE: &str = "docs/client/badges/coverage.svg";

#[derive(Debug, Clone)]
struct Config {
    ts: PathBuf,
    rust: PathBuf,
    contracts: PathBuf,
    out_json: PathBuf,
    out_badge: PathBuf,
}

fn usage() {
    println!("coverage-badge-kernel commands:");
    println!(
        "  protheus-ops coverage-badge-kernel [run] [--ts=<path>] [--rust=<path>] [--contracts=<path>] [--out-json=<path>] [--out-badge=<path>]"
    );
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn parse_flag(argv: &[String], prefix: &str) -> Option<String> {
    argv.iter().find_map(|token| {
        token
            .strip_prefix(prefix)
            .map(str::trim)
            .map(ToString::to_string)
    })
}

fn parse_config(root: &Path, argv: &[String]) -> Config {
    let ts_rel = parse_flag(argv, "--ts=").unwrap_or_else(|| DEFAULT_TS_PATH.to_string());
    let rust_rel = parse_flag(argv, "--rust=").unwrap_or_else(|| DEFAULT_RUST_PATH.to_string());
    let contracts_rel =
        parse_flag(argv, "--contracts=").unwrap_or_else(|| DEFAULT_CONTRACTS_PATH.to_string());
    let out_json_rel =
        parse_flag(argv, "--out-json=").unwrap_or_else(|| DEFAULT_OUT_JSON.to_string());
    let out_badge_rel =
        parse_flag(argv, "--out-badge=").unwrap_or_else(|| DEFAULT_OUT_BADGE.to_string());

    Config {
        ts: root.join(ts_rel),
        rust: root.join(rust_rel),
        contracts: root.join(contracts_rel),
        out_json: root.join(out_json_rel),
        out_badge: root.join(out_badge_rel),
    }
}

fn clamp_percent(v: f64) -> f64 {
    if !v.is_finite() {
        return 0.0;
    }
    v.clamp(0.0, 100.0)
}

fn read_json_safe(path: &Path) -> Value {
    let raw = fs::read_to_string(path).unwrap_or_default();
    serde_json::from_str::<Value>(&raw).unwrap_or_else(|_| json!({}))
}

fn read_text_safe(path: &Path) -> String {
    fs::read_to_string(path).unwrap_or_default()
}

fn parse_ts_coverage(path: &Path) -> Value {
    let raw = read_json_safe(path);
    let total = raw.get("total").and_then(Value::as_object);

    let lines = total
        .and_then(|obj| obj.get("lines"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let statements = total
        .and_then(|obj| obj.get("statements"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let functions = total
        .and_then(|obj| obj.get("functions"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let branches = total
        .and_then(|obj| obj.get("branches"))
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();

    let lines_pct = lines
        .get("pct")
        .and_then(Value::as_f64)
        .or_else(|| statements.get("pct").and_then(Value::as_f64))
        .unwrap_or(0.0);

    json!({
        "pct": clamp_percent(lines_pct),
        "lines_total": lines.get("total").and_then(Value::as_u64).unwrap_or(0),
        "lines_covered": lines.get("covered").and_then(Value::as_u64).unwrap_or(0),
        "statements_pct": clamp_percent(statements.get("pct").and_then(Value::as_f64).unwrap_or(0.0)),
        "functions_pct": clamp_percent(functions.get("pct").and_then(Value::as_f64).unwrap_or(0.0)),
        "branches_pct": clamp_percent(branches.get("pct").and_then(Value::as_f64).unwrap_or(0.0)),
    })
}

fn extract_first_percent(text: &str) -> f64 {
    let mut digits = String::new();
    let mut seen_dot = false;
    for ch in text.chars() {
        if ch.is_ascii_digit() {
            digits.push(ch);
            continue;
        }
        if ch == '.' && !seen_dot {
            seen_dot = true;
            digits.push(ch);
            continue;
        }
        if ch == '%' && !digits.is_empty() {
            return digits.parse::<f64>().unwrap_or(0.0);
        }
        if !digits.is_empty() {
            digits.clear();
            seen_dot = false;
        }
    }
    0.0
}

fn parse_rust_coverage(path: &Path) -> Value {
    let text = read_text_safe(path);
    let mut pct = 0.0;

    for line in text.lines() {
        if !line.to_ascii_lowercase().contains("total") {
            continue;
        }
        let candidate = extract_first_percent(line);
        if candidate > 0.0 {
            pct = candidate;
        }
    }

    if pct <= 0.0 {
        pct = extract_first_percent(&text);
    }

    json!({ "pct": clamp_percent(pct) })
}

fn parse_tooling_contract_coverage(path: &Path) -> Value {
    let snapshot = read_json_safe(path);
    let compact = snapshot.to_string().to_ascii_lowercase();
    let required = ["openai", "openrouter", "xai", "tts"];
    let mut present = 0usize;
    let matrix = required
        .iter()
        .map(|provider| {
            let has_provider = compact.contains(&format!("\"{}\"", provider));
            if has_provider {
                present += 1;
            }
            json!({
                "provider": provider,
                "present": has_provider
            })
        })
        .collect::<Vec<_>>();
    let pct = if required.is_empty() {
        0.0
    } else {
        (present as f64 / required.len() as f64) * 100.0
    };
    json!({
        "pct": clamp_percent(pct),
        "required": required.len(),
        "present": present,
        "matrix": matrix
    })
}

fn pick_color(pct: f64) -> &'static str {
    if pct >= 95.0 {
        "#22c55e"
    } else if pct >= 90.0 {
        "#84cc16"
    } else if pct >= 80.0 {
        "#f59e0b"
    } else {
        "#ef4444"
    }
}

fn xml_escape(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;")
}

fn build_badge(label: &str, value: &str, color: &str) -> String {
    let left = 78;
    let right = 62;
    let width = left + right;
    let safe_label = xml_escape(label);
    let safe_value = xml_escape(value);
    [
        format!(
            "<svg xmlns=\"http://www.w3.org/2000/svg\" width=\"{width}\" height=\"20\" role=\"img\" aria-label=\"{safe_label}: {safe_value}\">"
        ),
        "<title>coverage</title>".to_string(),
        format!("<rect width=\"{left}\" height=\"20\" fill=\"#334155\"/>"),
        format!("<rect x=\"{left}\" width=\"{right}\" height=\"20\" fill=\"{color}\"/>"),
        format!("<text x=\"39\" y=\"14\" fill=\"#fff\" text-anchor=\"middle\" font-family=\"Verdana,Geneva,DejaVu Sans,sans-serif\" font-size=\"11\">{safe_label}</text>"),
        format!(
            "<text x=\"{}\" y=\"14\" fill=\"#fff\" text-anchor=\"middle\" font-family=\"Verdana,Geneva,DejaVu Sans,sans-serif\" font-size=\"11\">{safe_value}</text>",
            left + right / 2
        ),
        "</svg>".to_string(),
        String::new(),
    ]
    .join("\n")
}

fn ensure_parent(path: &Path) -> Result<(), String> {
    match path.parent() {
        Some(parent) => fs::create_dir_all(parent).map_err(|err| err.to_string()),
        None => Ok(()),
    }
}

fn round2(value: f64) -> f64 {
    (value * 100.0).round() / 100.0
}

fn run_command(root: &Path, argv: &[String]) -> Result<Value, String> {
    let config = parse_config(root, argv);
    let ts = parse_ts_coverage(&config.ts);
    let rust = parse_rust_coverage(&config.rust);
    let tooling_contracts = parse_tooling_contract_coverage(&config.contracts);

    let ts_pct = ts.get("pct").and_then(Value::as_f64).unwrap_or(0.0);
    let rust_pct = rust.get("pct").and_then(Value::as_f64).unwrap_or(0.0);
    let tooling_contract_pct = tooling_contracts
        .get("pct")
        .and_then(Value::as_f64)
        .unwrap_or(0.0);
    let combined_pct = round2((ts_pct + rust_pct + tooling_contract_pct) / 3.0);

    let payload = json!({
        "ok": true,
        "type": "coverage_merge_summary",
        "ts": ts,
        "rust": rust,
        "tooling_contracts": tooling_contracts,
        "combined_pct": combined_pct,
        "threshold_95_ok": combined_pct >= 95.0,
        "components": {
            "ts_pct": ts_pct,
            "rust_pct": rust_pct,
            "tooling_contract_pct": tooling_contract_pct
        },
        "ts_path": config.ts.to_string_lossy().to_string(),
        "rust_path": config.rust.to_string_lossy().to_string(),
        "contracts_path": config.contracts.to_string_lossy().to_string(),
    });

    ensure_parent(&config.out_json)?;
    fs::write(
        &config.out_json,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&payload).map_err(|err| err.to_string())?
        ),
    )
    .map_err(|err| err.to_string())?;

    let badge = build_badge(
        "coverage",
        &format!("{combined_pct:.2}%"),
        pick_color(combined_pct),
    );
    ensure_parent(&config.out_badge)?;
    fs::write(&config.out_badge, badge).map_err(|err| err.to_string())?;

    Ok(payload)
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if argv.iter().any(|token| token == "--help" || token == "-h") {
        usage();
        return 0;
    }

    let cmd = argv
        .iter()
        .find(|token| !token.starts_with("--"))
        .map(|token| token.to_ascii_lowercase())
        .unwrap_or_else(|| "run".to_string());

    if !matches!(cmd.as_str(), "run") {
        usage();
        print_json_line(&json!({
            "ok": false,
            "type": "coverage_merge_summary",
            "error": "unknown_command",
            "command": cmd,
        }));
        return 2;
    }

    match run_command(root, argv) {
        Ok(payload) => {
            print_json_line(&payload);
            0
        }
        Err(err) => {
            print_json_line(&json!({
                "ok": false,
                "type": "coverage_merge_summary",
                "error": err,
            }));
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{extract_first_percent, parse_rust_coverage};
    use std::fs;

    #[test]
    fn extract_first_percent_reads_basic_token() {
        assert_eq!(extract_first_percent("TOTAL 100 20 80.5%"), 80.5);
    }

    #[test]
    fn parse_rust_coverage_falls_back_to_first_percent() {
        let temp = tempfile::tempdir().expect("tempdir");
        let path = temp.path().join("summary.txt");
        fs::write(&path, "header\n87.25%\n").expect("write");
        let parsed = parse_rust_coverage(&path);
        let pct = parsed
            .get("pct")
            .and_then(serde_json::Value::as_f64)
            .unwrap_or(0.0);
        assert_eq!(pct, 87.25);
    }
}
