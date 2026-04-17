// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use base64::Engine;
use serde_json::{json, Value};
use std::path::Path;

fn usage() {
    println!("protheusd-launcher-kernel commands:");
    println!("  protheus-ops protheusd-launcher-kernel gate [--payload-base64=<base64_json>]");
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn parse_payload(argv: &[String]) -> Value {
    let maybe_base64 = argv
        .iter()
        .find_map(|token| token.strip_prefix("--payload-base64="))
        .map(str::trim)
        .filter(|raw| !raw.is_empty());

    let Some(raw) = maybe_base64 else {
        return json!({});
    };

    let decoded = base64::engine::general_purpose::STANDARD
        .decode(raw)
        .unwrap_or_default();
    let decoded_text = String::from_utf8(decoded).unwrap_or_default();
    serde_json::from_str::<Value>(&decoded_text).unwrap_or_else(|_| json!({}))
}

fn parse_args_from_payload(payload: &Value) -> Vec<String> {
    payload
        .get("argv")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(str::trim)
                .filter(|row| !row.is_empty())
                .map(ToString::to_string)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn has_flag(argv: &[String], needle: &str) -> bool {
    argv.iter().any(|token| token == needle)
}

fn with_execution_receipt(mut out: Value, command: &str, status: &str) -> Value {
    out["execution_receipt"] = json!({
        "lane": "protheusd_launcher_kernel",
        "command": command,
        "status": status,
        "source": "OPENCLAW-TOOLING-WEB-100",
        "tool_runtime_class": "receipt_wrapped"
    });
    out
}

fn run_gate(argv: &[String]) -> Value {
    let payload = parse_payload(argv);
    let args = parse_args_from_payload(&payload);

    let strict = std::env::var("PROTHEUS_CONDUIT_STRICT")
        .ok()
        .as_deref()
        .map(str::trim)
        .filter(|raw| !raw.is_empty())
        .map(|raw| raw != "0")
        .unwrap_or(true);

    let conduit_missing = std::env::var("PROTHEUS_CONDUIT_AVAILABLE")
        .ok()
        .as_deref()
        .map(str::trim)
        .map(|raw| raw == "0")
        .unwrap_or(false);

    let allow_legacy_fallback = has_flag(&args, "--allow-legacy-fallback");

    if strict && conduit_missing && !allow_legacy_fallback {
        return with_execution_receipt(
            json!({
                "ok": false,
                "type": "protheusd_launcher_gate",
                "error": "conduit_required_strict",
                "strict": strict,
                "conduit_missing": conduit_missing,
                "allow_legacy_fallback": allow_legacy_fallback,
                "pass_args": args,
                "exit_code": 2,
            }),
            "gate",
            "error",
        );
    }

    with_execution_receipt(
        json!({
            "ok": true,
            "type": "protheusd_launcher_gate",
            "strict": strict,
            "conduit_missing": conduit_missing,
            "allow_legacy_fallback": allow_legacy_fallback,
            "pass_args": args,
            "exit_code": 0,
        }),
        "gate",
        "success",
    )
}

pub fn run(_root: &Path, argv: &[String]) -> i32 {
    let cmd = argv
        .iter()
        .find(|token| !token.starts_with("--"))
        .map(|token| token.to_ascii_lowercase())
        .unwrap_or_else(|| "gate".to_string());

    if cmd == "help" || cmd == "--help" || cmd == "-h" {
        usage();
        return 0;
    }

    if cmd != "gate" {
        usage();
        print_json_line(&with_execution_receipt(
            json!({
                "ok": false,
                "type": "protheusd_launcher_gate",
                "error": "unknown_command",
                "command": cmd,
            }),
            &cmd,
            "error",
        ));
        return 2;
    }

    let out = run_gate(argv);
    let exit = out.get("exit_code").and_then(Value::as_i64).unwrap_or(1) as i32;
    print_json_line(&out);
    exit
}

#[cfg(test)]
mod tests {
    use super::{has_flag, parse_args_from_payload};
    use serde_json::json;

    #[test]
    fn parse_args_reads_payload_argv() {
        let parsed = parse_args_from_payload(&json!({"argv": ["a", "", "b"]}));
        assert_eq!(parsed, vec!["a", "b"]);
    }

    #[test]
    fn has_flag_detects_flag() {
        let argv = vec!["--a".to_string(), "--allow-legacy-fallback".to_string()];
        assert!(has_flag(&argv, "--allow-legacy-fallback"));
    }
}
