// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
// WEB CONDUIT + SAFETY: fail-closed routed fetch with deterministic receipts.

use chrono::{DateTime, Utc};
use regex::Regex;
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::OnceLock;

use crate::parse_args;
use crate::web_conduit_provider_runtime::{
    fetch_cache_key, fetch_provider_catalog_snapshot, fetch_provider_chain_from_request,
    fetch_provider_registration_contract, load_fetch_cache, load_search_cache,
    normalized_search_filters, provider_catalog_snapshot, provider_chain_from_request,
    provider_circuit_open_until, provider_health_snapshot, recent_tool_attempt_replay_guard,
    record_provider_attempt, resolve_provider_credential_source_with_env,
    resolve_search_cache_ttl_seconds, resolve_search_count, resolve_search_provider_credential,
    resolve_search_timeout_ms, runtime_web_execution_gate, runtime_web_process_summary,
    runtime_web_replay_bypass, runtime_web_replay_guard_passthrough,
    runtime_web_replay_policy, runtime_web_request_flag, runtime_web_tools_snapshot,
    runtime_web_tools_state_path, runtime_web_truthy_flag, search_cache_key,
    search_default_timeout_ms, search_provider_registration_contract,
    search_provider_request_contract, store_fetch_cache, store_search_cache,
    unsupported_search_filter_response, validate_explicit_fetch_provider_hint,
    validate_explicit_provider_hint, web_provider_public_artifact_contracts,
    web_tool_catalog_snapshot, WebProviderFamily,
};

const POLICY_REL: &str = "client/runtime/config/web_conduit_policy.json";
const RECEIPTS_REL: &str = "client/runtime/local/state/web_conduit/receipts.jsonl";
const APPROVALS_REL: &str = "client/runtime/local/state/ui/infring_dashboard/approvals.json";
const ARTIFACTS_DIR_REL: &str = "client/runtime/local/state/web_conduit/artifacts";
const DEFAULT_ACCEPT_LANGUAGE: &str = "en-US,en;q=0.9";
const DEFAULT_REFERER: &str = "https://www.google.com/";
const DEFAULT_WEB_USER_AGENTS: &[&str] = &[
    "Infring-WebConduit/1.0",
    "Mozilla/5.0 (Macintosh; Intel Mac OS X 14_5) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/124.0 Safari/537.36",
    "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/123.0 Safari/537.36",
    "Mozilla/5.0 (Windows NT 10.0; Win64; x64; rv:125.0) Gecko/20100101 Firefox/125.0",
];
const SERPER_SEARCH_URL: &str = "https://google.serper.dev/search";

fn usage() {
    println!("web-conduit commands:");
    println!("  infring-ops web-conduit status");
    println!("  infring-ops web-conduit receipts [--limit=<n>]");
    println!(
        "  infring-ops web-conduit setup [--provider=<serperdev|duckduckgo|duckduckgo-lite|bing>] [--api-key=<key>] [--api-key-env=<ENV>] [--apply=1] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit migrate-legacy-config [--source-path=<path>] [--apply=1] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit native-codex [--model-provider=<id>] [--model-api=<id>] [--payload-json=<json>] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit fetch --url=<https://...> [--provider=auto|direct-http|curl] [--extract-mode=text|markdown] [--max-chars=<n>] [--cache-ttl-minutes=<n>] [--timeout-ms=<n>] [--max-response-bytes=<n>] [--resolve-citation-redirect=1] [--human-approved=1] [--approval-id=<id>] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit media --url=<https://...|file://...>|--path=<local-path> [--workspace-dir=<path>] [--local-roots=<path,...>|any] [--host-read-capability=1] [--max-bytes=<n>] [--optimize-images=1] [--raw=1] [--resolve-citation-redirect=1] [--human-approved=1] [--approval-id=<id>] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit audio-probe --url=<https://...|file://...>|--path=<local-path> [--workspace-dir=<path>] [--local-roots=<path,...>|any] [--host-read-capability=1] [--max-bytes=<n>]"
    );
    println!(
        "  infring-ops web-conduit pdf-extract --url=<https://...|file://...>|--path=<local-path> [--max-pages=<n>] [--page-numbers=1,2] [--min-text-chars=<n>] [--extract-images=1] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit pdf-native-analyze --provider=<anthropic|google> --model-id=<id> --prompt='<text>' [--path=<pdf>|--url=<pdf>|--sources-json=<json>] [--api-key=<key>|--api-key-env=<ENV>] [--base-url=<url>] [--max-tokens=<n>] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit pdf-tool [--prompt='<text>'] [--model=<provider/model>|--provider=<provider> --model-id=<id>] [--path=<pdf>|--url=<pdf>|--pdf=<pdf>|--pdfs-json=<json>] [--pages=1-3,5] [--max-bytes-mb=<n>] [--max-pages=<n>] [--min-text-chars=<n>] [--api-key=<key>|--api-key-env=<ENV>] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit image-metadata --url=<https://...|file://...>|--path=<local-path> [--workspace-dir=<path>] [--local-roots=<path,...>|any] [--host-read-capability=1] [--max-bytes=<n>] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit image-tool-status [--provider=<id>] [--model=<provider/model|model>] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit image-tool [--prompt='<text>'] [--provider=<id>] [--model=<provider/model|model>] [--image=<path|url>|--images-json=<json>|--path=<path>|--url=<url>] [--max-images=<n>] [--max-bytes=<n>] [--timeout-seconds=<n>] [--max-tokens=<n>] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit attachments [--context-json=<json>] [--attachments-json=<json>] [--media-path=<path>|--media-url=<url>|--media-type=<mime>] [--media-paths-json=<json>] [--media-urls-json=<json>] [--media-types-json=<json>] [--already-transcribed-indices=0,2] [--capability=image|audio|video] [--prefer=first|last|path|url] [--mode=first|all] [--max-attachments=<n>] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit media-host --url=<https://...|file://...>|--path=<local-path> [--workspace-dir=<path>] [--local-roots=<path,...>|any] [--ttl-seconds=<n>] [--base-url=<url>] [--summary-only=1]"
    );
    println!(
        "  infring-ops web-conduit outbound-attachment --url=<https://...|file://...>|--path=<local-path> [--workspace-dir=<path>] [--local-roots=<path,...>|any] [--host-read-capability=1] [--optimize-images=1] [--raw=1] [--summary-only=1]"
    );
    println!("  infring-ops web-conduit parse-media --text='<output with MEDIA:... tokens>'");
    println!(
        "  infring-ops web-conduit qr-image --text='<text>' [--scale=<n>] [--margin-modules=<n>] [--prompt-image-order=inline|offloaded] [--summary-only=1]"
    );
    println!("  infring-ops web-conduit file-context --content='<text>' [--content-base64=<base64>] [--file-name=<name>] [--mime-type=<type>] [--fallback-name=<name>] [--compact=1]");
    println!(
        "  infring-ops web-conduit search --query=<terms> [--provider=auto|serper|duckduckgo|duckduckgo-lite|bing] [--top-k=8|--count=8] [--timeout-ms=<n>] [--cache-ttl-minutes=<n>] [--allowed-domains=docs.rs,github.com] [--exact-domain-only=1] [--country=<code>] [--language=<code>] [--freshness=<token>] [--date-after=<YYYY-MM-DD>] [--date-before=<YYYY-MM-DD>] [--human-approved=1] [--summary-only=1]"
    );
    println!("  infring-ops web-conduit providers");
    println!("  infring-ops browse fetch --url=<https://...>");
}

fn clean_text(raw: &str, max_len: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(max_len.max(1))
        .collect::<String>()
}

fn read_json_or(path: &Path, fallback: Value) -> Value {
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str::<Value>(&raw).unwrap_or(fallback),
        Err(_) => fallback,
    }
}

fn write_json_atomic(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("web_conduit_create_policy_dir_failed:{err}"))?;
    }
    let tmp = path.with_extension(format!(
        "tmp-{}-{}",
        std::process::id(),
        Utc::now().timestamp_millis()
    ));
    let encoded = serde_json::to_vec_pretty(value)
        .map_err(|err| format!("web_conduit_encode_policy_failed:{err}"))?;
    fs::write(&tmp, encoded).map_err(|err| format!("web_conduit_write_policy_tmp_failed:{err}"))?;
    fs::rename(&tmp, path).map_err(|err| format!("web_conduit_rename_policy_failed:{err}"))?;
    Ok(())
}

fn append_jsonl(path: &Path, row: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("web_conduit_create_state_dir_failed:{err}"))?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("web_conduit_open_receipts_failed:{err}"))?;
    let line = serde_json::to_string(row)
        .map_err(|err| format!("web_conduit_encode_receipt_failed:{err}"))?;
    writeln!(file, "{line}").map_err(|err| format!("web_conduit_append_receipt_failed:{err}"))?;
    Ok(())
}

fn parse_bool(value: Option<&String>) -> bool {
    value
        .map(|raw| {
            matches!(
                raw.trim().to_ascii_lowercase().as_str(),
                "1" | "true" | "yes" | "on"
            )
        })
        .unwrap_or(false)
}

fn parse_u64(value: Option<&String>, fallback: u64, min: u64, max: u64) -> u64 {
    value
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(fallback)
        .clamp(min, max)
}

fn policy_path(root: &Path) -> PathBuf {
    if let Ok(raw) = std::env::var("INFRING_WEB_CONDUIT_POLICY_PATH") {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            return PathBuf::from(trimmed);
        }
    }
    root.join(POLICY_REL)
}

fn receipts_path(root: &Path) -> PathBuf {
    root.join(RECEIPTS_REL)
}

fn approvals_path(root: &Path) -> PathBuf {
    root.join(APPROVALS_REL)
}

fn artifacts_dir_path(root: &Path) -> PathBuf {
    root.join(ARTIFACTS_DIR_REL)
}

fn default_policy() -> Value {
    json!({
        "version": "v1",
        "mode": "production",
        "web_conduit": {
            "enabled": true,
            "max_response_bytes": 350000,
            "timeout_ms": 9000,
            "rate_limit_per_minute": 30,
            "allow_domains": [],
            "deny_domains": [
                "127.0.0.1",
                "localhost",
                "metadata.google.internal",
                "169.254.169.254"
            ],
            "sensitive_domains": [
                "accounts.google.com",
                "api.stripe.com",
                "paypal.com",
                "chase.com",
                "bankofamerica.com"
            ],
            "require_human_for_sensitive": true,
            "search_default_count": 8,
            "search_max_count": 12,
            "search_cache_ttl_minutes": 8,
            "search_provider_order": ["serperdev", "duckduckgo", "duckduckgo_lite", "bing_rss"],
            "fetch_provider_order": ["direct_http"],
            "provider_circuit_breaker": {
                "enabled": true,
                "failure_threshold": 3,
                "open_for_secs": 300
            },
            "native_codex_web_search": {
                "enabled": false,
                "mode": "cached",
                "allowed_domains": []
            },
            "image_tool": {
                "enabled": true,
                "provider": "",
                "model": "",
                "default_prompt": "Describe the image.",
                "max_images": 20,
                "max_bytes": 10485760,
                "timeout_seconds": 60,
                "output_max_buffer_bytes": 5242880,
                "media_concurrency": 2
            }
        }
    })
}

fn load_policy(root: &Path) -> (Value, PathBuf) {
    let path = policy_path(root);
    if !path.exists() {
        let _ = write_json_atomic(&path, &default_policy());
    }
    (read_json_or(&path, default_policy()), path)
}

fn load_approvals(root: &Path) -> Vec<Value> {
    let path = approvals_path(root);
    let raw = read_json_or(&path, json!({"approvals": []}));
    raw.get("approvals")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
}

fn save_approvals(root: &Path, approvals: &[Value]) -> Result<(), String> {
    write_json_atomic(
        &approvals_path(root),
        &json!({
            "type": "infring_dashboard_approvals",
            "updated_at": crate::now_iso(),
            "approvals": approvals
        }),
    )
}

fn approval_state_for_request(
    root: &Path,
    approval_id: &str,
    requested_url: &str,
) -> Option<String> {
    let approval_key = clean_text(approval_id, 160);
    if approval_key.is_empty() {
        return None;
    }
    let url_key = clean_text(requested_url, 2200);
    for row in load_approvals(root) {
        let row_id = clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 160);
        if row_id != approval_key {
            continue;
        }
        let row_url = clean_text(
            row.get("requested_url")
                .and_then(Value::as_str)
                .unwrap_or(""),
            2200,
        );
        if !row_url.is_empty() && !url_key.is_empty() && row_url != url_key {
            return Some("mismatched".to_string());
        }
        let state = clean_text(
            row.get("status")
                .and_then(Value::as_str)
                .unwrap_or("pending"),
            40,
        )
        .to_ascii_lowercase();
        return if state.is_empty() {
            Some("pending".to_string())
        } else {
            Some(state)
        };
    }
    None
}

fn ensure_sensitive_web_approval(
    root: &Path,
    requested_url: &str,
    policy_eval: &Value,
) -> Option<Value> {
    let requested = clean_text(requested_url, 2200);
    if requested.is_empty() {
        return None;
    }
    let domain = extract_domain(&requested);
    let approval_id = format!(
        "approval-web-{}",
        &sha256_hex(&format!("{}:{}", domain, requested))[..10]
    );
    let mut approvals = load_approvals(root);
    if let Some(existing) = approvals
        .iter()
        .find(|row| {
            clean_text(row.get("id").and_then(Value::as_str).unwrap_or(""), 160) == approval_id
                && clean_text(
                    row.get("requested_url")
                        .and_then(Value::as_str)
                        .unwrap_or(""),
                    2200,
                ) == requested
                && clean_text(
                    row.get("status")
                        .and_then(Value::as_str)
                        .unwrap_or("pending"),
                    40,
                )
                .to_ascii_lowercase()
                    == "pending"
        })
        .cloned()
    {
        return Some(existing);
    }
    let now = crate::now_iso();
    let row = json!({
        "id": approval_id,
        "action": "Web fetch approval",
        "description": format!("Approve governed web fetch for {}.", requested),
        "agent_name": "web_conduit",
        "status": "pending",
        "domain": domain,
        "requested_url": requested,
        "policy_reason": clean_text(policy_eval.get("reason").and_then(Value::as_str).unwrap_or("human_approval_required_for_sensitive_domain"), 180),
        "created_at": now,
        "updated_at": now
    });
    approvals.push(row.clone());
    let _ = save_approvals(root, &approvals);
    Some(row)
}

fn read_recent_receipts(root: &Path, limit: usize) -> Vec<Value> {
    let raw = fs::read_to_string(receipts_path(root)).unwrap_or_default();
    raw.lines()
        .rev()
        .take(limit.max(1))
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .collect::<Vec<_>>()
}

fn receipt_count(root: &Path) -> usize {
    fs::read_to_string(receipts_path(root))
        .ok()
        .map(|raw| raw.lines().count())
        .unwrap_or(0)
}

fn requests_last_minute(root: &Path) -> u64 {
    let now = Utc::now();
    let mut count = 0u64;
    for row in read_recent_receipts(root, 400) {
        let ts = row
            .get("timestamp")
            .and_then(Value::as_str)
            .unwrap_or_default();
        let Ok(parsed) = DateTime::parse_from_rfc3339(ts) else {
            continue;
        };
        let age = now.signed_duration_since(parsed.with_timezone(&Utc));
        if age.num_seconds() <= 60 {
            count = count.saturating_add(1);
        }
    }
    count
}
