// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::research_plane (authoritative)

use crate::research_batch6;
use crate::research_batch7;
use crate::research_batch8;
use crate::v8_kernel::{
    parse_bool, parse_u64, read_json, scoped_state_root, sha256_hex_str, write_receipt,
};
use crate::{clean, parse_args, ParsedArgs};
use crate::{crawl_console, crawl_middleware, crawl_pipeline, crawl_signals, crawl_spider};
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use serde_json::{json, Value};
use std::collections::{BTreeMap, VecDeque};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::Instant;

const STATE_ENV: &str = "RESEARCH_PLANE_STATE_ROOT";
const STATE_SCOPE: &str = "research_plane";

const CONTRACT_PATH: &str = "planes/contracts/research/research_plane_v1.json";
const POLICY_PATH: &str = "client/runtime/config/research_plane_policy.json";

fn usage() {
    println!("Usage:");
    println!("  protheus-ops research-plane status");
    println!("  protheus-ops research-plane diagnostics [--strict=1|0]");
    println!("  protheus-ops research-plane fetch --url=<url> [--mode=auto|http|stealth|browser] [--timeout-ms=<n>] [--max-bytes=<n>] [--strict=1|0]");
    println!("  protheus-ops research-plane fetch --stealth --url=<url> [--timeout-ms=<n>] [--strict=1|0]");
    println!("  protheus-ops research-plane recover-selectors [--html=<text>|--html-base64=<b64>|--html-path=<path>] [--selectors=a,b,c] [--target-text=<text>] [--strict=1|0]");
    println!("  protheus-ops research-plane crawl --seed-urls=<u1,u2> [--max-pages=<n>] [--max-concurrency=<n>] [--max-retries=<n>] [--per-domain-qps=<n>] [--checkpoint-path=<path>] [--resume=1|0] [--strict=1|0]");
    println!("  protheus-ops research-plane mcp-extract [--payload=<html>|--payload-path=<path>] [--source=<url>] [--query=<text>] [--strict=1|0]");
    println!("  protheus-ops research-plane spider|crawl-spider [--graph-json=<json>|--graph-path=<path>] --seed-urls=<u1,u2> [--allow-rules=a,b] [--deny-rules=a,b] [--allowed-domains=a,b] [--max-depth=<n>] [--max-links=<n>] [--strict=1|0]");
    println!("  protheus-ops research-plane middleware|crawl-middleware [--request-json=<json>] [--response-json=<json>] [--stack-json=<json>] [--strict=1|0]");
    println!("  protheus-ops research-plane pipeline|crawl-pipeline [--items-json=<json>|--items-path=<path>] [--pipeline-json=<json>|--pipeline-path=<path>] [--export-format=json|csv] [--export-path=<path>] [--strict=1|0]");
    println!("  protheus-ops research-plane signals|crawl-signals [--events-json=<json>] [--handlers-json=<json>] [--strict=1|0]");
    println!("  protheus-ops research-plane console|crawl-console --op=<status|stats|queue|pause|resume|enqueue> --auth-token=<token> [--url=<u>] [--strict=1|0]");
    println!("  protheus-ops research-plane template-governance [--manifest=<path>] [--templates-root=<dir>] [--strict=1|0]");
    println!("  protheus-ops research-plane goal-crawl --goal=<text> [--max-pages=<n>] [--max-discovery=<n>] [--catalog-json=<json>|--catalog-path=<path>] [--strict=1|0]");
    println!("  protheus-ops research-plane map-site --domain=<host|url> [--depth=<n>] [--graph-json=<json>|--graph-path=<path>] [--strict=1|0]");
    println!("  protheus-ops research-plane extract-structured [--payload=<html>|--payload-path=<path>] [--schema-json=<json>|--schema-path=<path>|--prompt=<text>] [--strict=1|0]");
    println!("  protheus-ops research-plane monitor --url=<url> [--content=<text>|--content-path=<path>] [--strict=1|0]");
    println!("  protheus-ops research-plane firecrawl-template-governance [--manifest=<path>] [--templates-root=<dir>] [--strict=1|0]");
    println!("  protheus-ops research-plane js-scrape --url=<url> [--mode=js-render|stealth-js] [--wait-ms=<n>] [--selector=<s>] [--form-json=<json>|--form-path=<path>] [--strict=1|0]");
    println!("  protheus-ops research-plane auth-session --op=<open|login|status|close> [--session-id=<id>] [--username=<u> --password=<p>] [--strict=1|0]");
    println!("  protheus-ops research-plane proxy-rotate [--proxies=a,b] [--attempt-signals=s1,s2] [--strict=1|0]");
    println!("  protheus-ops research-plane parallel-scrape-workers [--targets=u1,u2|--targets-file=<path>] [--session-ids=s1,s2] [--max-concurrency=<n>] [--max-retries=<n>] [--strict=1|0]");
    println!("  protheus-ops research-plane book-patterns-template-governance [--manifest=<path>] [--templates-root=<dir>] [--strict=1|0]");
    println!("  protheus-ops research-plane decode-news-url --url=<news-url> [--proxy-mode=none|http|https|socks] [--proxy=<url>|--proxies=a,b] [--interval-ms=<n>] [--backoff-ms=<n>] [--max-attempts=<n>] [--strict=1|0]");
    println!("  protheus-ops research-plane decode-news-urls [--urls=u1,u2|--urls-file=<path>] [--continue-on-error=1|0] [--proxy-mode=none|http|https|socks] [--proxy=<url>|--proxies=a,b] [--interval-ms=<n>] [--backoff-ms=<n>] [--max-attempts=<n>] [--strict=1|0]");
    println!("  protheus-ops research-plane decoder-template-governance [--manifest=<path>] [--templates-root=<dir>] [--strict=1|0]");
}

fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn latest_path(root: &Path) -> PathBuf {
    state_root(root).join("latest.json")
}

fn history_path(root: &Path) -> PathBuf {
    state_root(root).join("history.jsonl")
}

fn print_payload(payload: &Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(payload)
            .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"encode_failed\"}".to_string())
    );
}

fn emit(root: &Path, payload: Value) -> i32 {
    match write_receipt(root, STATE_ENV, STATE_SCOPE, payload) {
        Ok(out) => {
            print_payload(&out);
            if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                0
            } else {
                1
            }
        }
        Err(err) => {
            print_payload(&json!({
                "ok": false,
                "type": "research_plane_error",
                "error": clean(err, 240)
            }));
            1
        }
    }
}

fn load_json_or(root: &Path, rel: &str, fallback: Value) -> Value {
    read_json(&root.join(rel)).unwrap_or(fallback)
}

fn status(root: &Path) -> Value {
    json!({
        "ok": true,
        "type": "research_plane_status",
        "lane": "core/layer0/ops",
        "latest_path": latest_path(root).display().to_string(),
        "history_path": history_path(root).display().to_string(),
        "safety_counters_path": state_root(root).join("safety").join("gate_counters.json").display().to_string(),
        "latest": read_json(&latest_path(root))
    })
}

fn diagnostics(root: &Path) -> Value {
    let policy = load_json_or(
        root,
        POLICY_PATH,
        json!({
            "version": "v1",
            "kind": "research_plane_policy",
            "safety_plane": {
                "enabled": true,
                "required_modes": ["stealth", "browser"],
                "allow_actions": ["research_*:*", "research:*"],
                "max_requests_per_mode": {"stealth": 20000, "browser": 5000}
            }
        }),
    );
    let safety = read_json(&state_root(root).join("safety").join("gate_counters.json"))
        .unwrap_or_else(|| json!({"total":0_u64,"modes":{}}));
    json!({
        "ok": true,
        "type": "research_plane_diagnostics",
        "lane": "core/layer0/ops",
        "policy_path": POLICY_PATH,
        "contract_path": CONTRACT_PATH,
        "safety_plane_policy": policy.get("safety_plane").cloned().unwrap_or(Value::Null),
        "safety_plane_counters": safety,
        "developer_dx": {
            "stealth_entrypoint": "protheus research --stealth --url=<url>",
            "console_entrypoint": "protheus-ops research-plane console --op=stats --auth-token=<token>"
        },
        "claim_evidence": [
            {
                "id": "V6-RESEARCH-001.6",
                "claim": "developer_facing_stealth_diagnostics_surface_is_available_from_cli",
                "evidence": {"diagnostics": true}
            }
        ]
    })
}

fn parse_headers(node: Option<&Value>) -> BTreeMap<String, String> {
    let mut out = BTreeMap::<String, String>::new();
    if let Some(Value::Object(map)) = node {
        for (k, v) in map {
            if let Some(val) = v.as_str() {
                let key = k.trim();
                let value = val.trim();
                if !key.is_empty() && !value.is_empty() {
                    out.insert(key.to_string(), value.to_string());
                }
            }
        }
    }
    out
}

fn protection_detected(body: &str, status_code: i64, signals: &[String]) -> bool {
    if matches!(status_code, 401 | 403 | 429 | 503) {
        return true;
    }
    let body_lc = body.to_ascii_lowercase();
    signals
        .iter()
        .any(|signal| body_lc.contains(&signal.to_ascii_lowercase()))
}

fn fetch_file_url(url: &str, max_bytes: usize) -> Value {
    let path = url.trim_start_matches("file://");
    let read = fs::read(path);
    match read {
        Ok(bytes) => {
            let clipped = bytes.iter().take(max_bytes).copied().collect::<Vec<_>>();
            let body = String::from_utf8_lossy(&clipped).to_string();
            json!({
                "ok": true,
                "status": 200,
                "body": body,
                "headers": {},
                "error": Value::Null
            })
        }
        Err(err) => json!({
            "ok": false,
            "status": 0,
            "body": "",
            "headers": {},
            "error": format!("file_read_failed:{err}")
        }),
    }
}

fn fetch_with_curl(
    url: &str,
    mode: &str,
    timeout_ms: u64,
    headers: &BTreeMap<String, String>,
    max_bytes: usize,
) -> Value {
    if url.starts_with("file://") {
        let started = Instant::now();
        let mut row = fetch_file_url(url, max_bytes);
        row["elapsed_ms"] = json!(started.elapsed().as_millis());
        row["mode"] = Value::String(mode.to_string());
        return row;
    }

    let timeout_sec = ((timeout_ms as f64) / 1000.0).ceil() as u64;
    let mut args = vec![
        "-sS".to_string(),
        "-L".to_string(),
        "--max-time".to_string(),
        timeout_sec.max(1).to_string(),
        "--compressed".to_string(),
        "-A".to_string(),
        "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0.0.0 Safari/537.36".to_string(),
    ];
    for (key, value) in headers {
        args.push("-H".to_string());
        args.push(format!("{key}: {value}"));
    }
    args.push("-w".to_string());
    args.push("\n__STATUS__:%{http_code}".to_string());
    args.push(url.to_string());

    let started = Instant::now();
    let output = Command::new("curl").args(args).output();
    match output {
        Ok(run) => {
            let stdout = String::from_utf8_lossy(&run.stdout).to_string();
            let stderr = String::from_utf8_lossy(&run.stderr).to_string();
            let marker = "\n__STATUS__:";
            let (body_raw, status_raw) = match stdout.rsplit_once(marker) {
                Some((body, status)) => (body.to_string(), status.trim().to_string()),
                None => (stdout, "0".to_string()),
            };
            let body = body_raw.chars().take(max_bytes).collect::<String>();
            let status = status_raw.parse::<i64>().unwrap_or(0);
            json!({
                "ok": run.status.success(),
                "status": status,
                "body": body,
                "headers": {},
                "error": if run.status.success() { Value::Null } else { Value::String(clean(stderr, 220)) },
                "elapsed_ms": started.elapsed().as_millis(),
                "mode": mode
            })
        }
        Err(err) => json!({
            "ok": false,
            "status": 0,
            "body": "",
            "headers": {},
            "error": format!("curl_spawn_failed:{err}"),
            "mode": mode
        }),
    }
}

#[cfg(test)]
mod research_plane_usage_tests {
    use super::*;

    #[test]
    fn fetch_with_curl_file_urls_emit_mode_and_elapsed_ms() {
        let temp = tempfile::tempdir().expect("tempdir");
        let page = temp.path().join("page.html");
        fs::write(&page, "<html>ok</html>").expect("write page");
        let out = fetch_with_curl(
            &format!("file://{}", page.display()),
            "stealth",
            1_000,
            &BTreeMap::new(),
            64,
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("mode").and_then(Value::as_str), Some("stealth"));
        assert!(out.get("elapsed_ms").and_then(Value::as_u64).is_some());
    }
}

fn fetch_auto(
    root: &Path,
    url: &str,
    selected_mode: &str,
    timeout_ms: u64,
    max_bytes: usize,
    policy: &Value,
    contract: &Value,
    strict: bool,
) -> Value {
    let signals = contract
        .get("protection_signals")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|v| v.as_str().map(ToString::to_string))
        .collect::<Vec<_>>();
    let headers_http = parse_headers(policy.get("headers").and_then(|h| h.get("http")));
    let headers_stealth = parse_headers(policy.get("headers").and_then(|h| h.get("stealth")));
    let headers_browser = parse_headers(policy.get("headers").and_then(|h| h.get("browser")));

    let mut attempts = Vec::<Value>::new();
    let mut safety_receipts = Vec::<Value>::new();
    let mut run_mode = |mode: &str, headers: &BTreeMap<String, String>| -> Value {
        let safety = research_batch6::safety_gate_receipt(
            root,
            policy,
            mode,
            "research_fetch:auto",
            url,
            strict,
        );
        let safety_ok = safety.get("ok").and_then(Value::as_bool).unwrap_or(false);
        safety_receipts.push(safety.clone());
        if strict && !safety_ok {
            let out = json!({
                "mode": mode,
                "ok": false,
                "status": 0,
                "protected": true,
                "error": "safety_plane_denied",
                "body_sha256": Value::Null,
                "safety_receipt_hash": safety.get("receipt_hash").cloned().unwrap_or(Value::Null)
            });
            attempts.push(out);
            return json!({
                "ok": false,
                "status": 0,
                "body": "",
                "headers": {},
                "error": "safety_plane_denied",
                "mode": mode
            });
        }
        let row = fetch_with_curl(url, mode, timeout_ms, headers, max_bytes);
        let status = row.get("status").and_then(Value::as_i64).unwrap_or(0);
        let body = row.get("body").and_then(Value::as_str).unwrap_or_default();
        let protected = protection_detected(body, status, &signals);
        let out = json!({
            "mode": mode,
            "ok": row.get("ok").and_then(Value::as_bool).unwrap_or(false),
            "status": status,
            "protected": protected,
            "error": row.get("error").cloned().unwrap_or(Value::Null),
            "body_sha256": sha256_hex_str(body),
            "safety_receipt_hash": safety.get("receipt_hash").cloned().unwrap_or(Value::Null)
        });
        attempts.push(out.clone());
        row
    };

    let mut selected = selected_mode.to_ascii_lowercase();
    let final_row = if selected == "auto" {
        let first = run_mode("http", &headers_http);
        let first_ok = first.get("ok").and_then(Value::as_bool).unwrap_or(false);
        let first_status = first.get("status").and_then(Value::as_i64).unwrap_or(0);
        let first_body = first
            .get("body")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let first_protected = protection_detected(first_body, first_status, &signals);
        if first_ok && !first_protected {
            selected = "http".to_string();
            first
        } else {
            let second = run_mode("stealth", &headers_stealth);
            let second_ok = second.get("ok").and_then(Value::as_bool).unwrap_or(false);
            let second_status = second.get("status").and_then(Value::as_i64).unwrap_or(0);
            let second_body = second
                .get("body")
                .and_then(Value::as_str)
                .unwrap_or_default();
            let second_protected = protection_detected(second_body, second_status, &signals);
            if second_ok && !second_protected {
                selected = "stealth".to_string();
                second
            } else {
                selected = "browser".to_string();
                run_mode("browser", &headers_browser)
            }
        }
    } else if selected == "http" {
        run_mode("http", &headers_http)
    } else if selected == "stealth" {
        run_mode("stealth", &headers_stealth)
    } else {
        selected = "browser".to_string();
        run_mode("browser", &headers_browser)
    };

    let status = final_row.get("status").and_then(Value::as_i64).unwrap_or(0);
    let body = final_row
        .get("body")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let ok = final_row
        .get("ok")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let protected = protection_detected(&body, status, &signals);
    json!({
        "ok": ok && !protected && (200..=299).contains(&status),
        "selected_mode": selected,
        "status": status,
        "body": body,
        "body_sha256": sha256_hex_str(final_row.get("body").and_then(Value::as_str).unwrap_or_default()),
        "attempts": attempts,
        "error": final_row.get("error").cloned().unwrap_or(Value::Null),
        "protected": protected,
        "safety_plane_receipts": safety_receipts
    })
}
