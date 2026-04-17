// SPDX-License-Identifier: Apache-2.0
use chrono::Utc;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

fn now_iso() -> String {
    Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

fn strip_invisible_control(value: &str) -> String {
    value
        .chars()
        .filter(|ch| {
            *ch == '\n'
                || *ch == '\t'
                || (!ch.is_control()
                    && !matches!(
                        ch,
                        '\u{200B}'
                            | '\u{200C}'
                            | '\u{200D}'
                            | '\u{2060}'
                            | '\u{FEFF}'
                    ))
        })
        .collect()
}

fn clean_text(v: &str, max_len: usize) -> String {
    let filtered = strip_invisible_control(v);
    let mut out = String::with_capacity(v.len().min(max_len));
    let mut last_space = false;
    for ch in filtered.chars() {
        let mapped = if ch.is_whitespace() { ' ' } else { ch };
        if mapped == ' ' {
            if last_space {
                continue;
            }
            last_space = true;
        } else {
            last_space = false;
        }
        out.push(mapped);
        if out.len() >= max_len {
            break;
        }
    }
    out.trim().to_string()
}

fn parse_flags(args: &[String]) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for token in args {
        if !token.starts_with("--") {
            continue;
        }
        if let Some((k, v)) = token[2..].split_once('=') {
            out.insert(k.to_string(), v.to_string());
        } else {
            out.insert(token[2..].to_string(), "1".to_string());
        }
    }
    out
}

fn parse_json_payload(raw: &str) -> Option<Value> {
    let text = raw.trim();
    if text.is_empty() {
        return None;
    }
    if let Ok(v) = serde_json::from_str::<Value>(text) {
        return Some(v);
    }
    let lines: Vec<&str> = text
        .lines()
        .map(|line| line.trim())
        .filter(|line| !line.is_empty())
        .collect();
    for line in lines.iter().rev() {
        if let Ok(v) = serde_json::from_str::<Value>(line) {
            return Some(v);
        }
    }
    if text.contains("```") {
        let mut fenced = text.split("```");
        while let Some(_prefix) = fenced.next() {
            let Some(block) = fenced.next() else { break };
            let normalized = block
                .trim()
                .strip_prefix("json")
                .map(str::trim)
                .unwrap_or_else(|| block.trim());
            if normalized.is_empty() {
                continue;
            }
            if let Ok(v) = serde_json::from_str::<Value>(normalized) {
                return Some(v);
            }
        }
    }
    None
}

fn ensure_dir(path: &Path) {
    if let Some(parent) = path.parent() {
        let _ = fs::create_dir_all(parent);
    }
}

fn read_json(path: &Path) -> Value {
    match fs::read_to_string(path) {
        Ok(raw) => serde_json::from_str::<Value>(&raw).unwrap_or(Value::Null),
        Err(_) => Value::Null,
    }
}

fn write_json_atomic(path: &Path, value: &Value) {
    ensure_dir(path);
    let tmp = path.with_extension(format!("tmp-{}", std::process::id()));
    let payload = serde_json::to_vec_pretty(value).unwrap_or_else(|_| b"{}\n".to_vec());
    if fs::write(&tmp, payload).is_ok() {
        let _ = fs::rename(tmp, path);
    }
}

fn append_jsonl(path: &Path, value: &Value) {
    ensure_dir(path);
    if let Ok(mut file) = fs::OpenOptions::new().create(true).append(true).open(path) {
        if let Ok(row) = serde_json::to_string(value) {
            let _ = file.write_all(row.as_bytes());
            let _ = file.write_all(b"\n");
        }
    }
}

fn read_jsonl(path: &Path) -> Vec<Value> {
    let raw = match fs::read_to_string(path) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    raw.lines()
        .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
        .collect()
}

fn resolve_path(root: &Path, raw: Option<&str>, fallback_rel: &str) -> PathBuf {
    let fallback = root.join(fallback_rel);
    let Some(raw) = raw else { return fallback };
    let expanded = raw
        .replace("${INFRING_WORKSPACE}", &root.to_string_lossy())
        .replacen("$INFRING_WORKSPACE", &root.to_string_lossy(), 1);
    if expanded.trim().is_empty() {
        return fallback;
    }
    let p = PathBuf::from(expanded);
    if p.is_absolute() {
        p
    } else {
        root.join(p)
    }
}

#[derive(Debug, Clone)]
struct CommandRun {
    ok: bool,
    payload: Value,
    error: String,
}

fn run_command_json(bin: &str, args: &[String], cwd: &Path) -> Option<CommandRun> {
    let started = std::time::Instant::now();
    let output = Command::new(bin)
        .args(args)
        .current_dir(cwd)
        .output()
        .ok()?;
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    let payload = parse_json_payload(&stdout)
        .or_else(|| parse_json_payload(&stderr))
        .unwrap_or(Value::Null);
    let status_text = output
        .status
        .code()
        .map(|v| v.to_string())
        .unwrap_or_else(|| "signal".to_string());
    let error_basis = if stderr.trim().is_empty() {
        stdout.clone()
    } else if stdout.trim().is_empty() {
        stderr.clone()
    } else {
        format!("{stderr}\n{stdout}")
    };
    Some(CommandRun {
        ok: output.status.success() && payload.is_object(),
        payload,
        error: clean_text(
            &format!(
                "status={status_text}; duration_ms={}; {}",
                started.elapsed().as_millis(),
                error_basis
            ),
            320,
        ),
    })
}

fn run_memory_core(root: &Path, args: &[String]) -> CommandRun {
    let explicit = env::var("PROTHEUS_MEMORY_CORE_BIN").unwrap_or_default();
    let candidates = vec![
        explicit,
        root.join("target/release/memory-cli")
            .to_string_lossy()
            .to_string(),
        root.join("target/debug/memory-cli")
            .to_string_lossy()
            .to_string(),
        root.join("core/layer0/memory_runtime/target/release/memory-cli")
            .to_string_lossy()
            .to_string(),
        root.join("core/layer0/memory_runtime/target/debug/memory-cli")
            .to_string_lossy()
            .to_string(),
    ];
    for candidate in candidates {
        if candidate.trim().is_empty() || !Path::new(&candidate).exists() {
            continue;
        }
        if let Some(run) = run_command_json(&candidate, args, root) {
            if run.ok {
                return run;
            }
        }
    }

    let mut cargo_args = vec![
        "run".to_string(),
        "--quiet".to_string(),
        "--manifest-path".to_string(),
        root.join("core/layer0/memory_runtime/Cargo.toml")
            .to_string_lossy()
            .to_string(),
        "--bin".to_string(),
        "memory-cli".to_string(),
        "--".to_string(),
    ];
    cargo_args.extend(args.iter().cloned());
    if let Some(run) = run_command_json("cargo", &cargo_args, root) {
        return run;
    }
    CommandRun {
        ok: false,
        payload: Value::Null,
        error: "memory_core_unavailable".to_string(),
    }
}

fn run_security_check(root: &Path, request: &Value) -> CommandRun {
    let request_json = serde_json::to_string(request).unwrap_or_else(|_| "{}".to_string());
    let arg = format!("--request-json={request_json}");
    let explicit = env::var("PROTHEUS_SECURITY_CORE_BIN").unwrap_or_default();
    let candidates = vec![
        explicit,
        root.join("target/release/security_core")
            .to_string_lossy()
            .to_string(),
        root.join("target/debug/security_core")
            .to_string_lossy()
            .to_string(),
        root.join("core/layer0/security/target/release/security_core")
            .to_string_lossy()
            .to_string(),
        root.join("core/layer0/security/target/debug/security_core")
            .to_string_lossy()
            .to_string(),
    ];
    for candidate in candidates {
        if candidate.trim().is_empty() || !Path::new(&candidate).exists() {
            continue;
        }
        if let Some(run) = run_command_json(&candidate, &["check".to_string(), arg.clone()], root) {
            if run.ok {
                return run;
            }
        }
    }

    let cargo_args = vec![
        "run".to_string(),
        "--quiet".to_string(),
        "--manifest-path".to_string(),
        root.join("core/layer0/security/Cargo.toml")
            .to_string_lossy()
            .to_string(),
        "--bin".to_string(),
        "security_core".to_string(),
        "--".to_string(),
        "check".to_string(),
        arg,
    ];
    if let Some(run) = run_command_json("cargo", &cargo_args, root) {
        return run;
    }
    CommandRun {
        ok: false,
        payload: Value::Null,
        error: "security_core_unavailable".to_string(),
    }
}

fn memory_view_policy(root: &Path) -> Value {
    let policy_path = env::var("MEMORY_ABSTRACTION_VIEW_POLICY_PATH")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| root.join("client/runtime/config/memory_abstraction_view_policy.json"));
    let raw = read_json(&policy_path);
    let default_limit = raw
        .get("default_limit")
        .and_then(|v| v.as_u64())
        .unwrap_or(5)
        .max(1);
    let paths = raw.get("paths").cloned().unwrap_or(Value::Null);
    let latest_path = resolve_path(
        root,
        paths.get("latest_path").and_then(|v| v.as_str()),
        "local/state/client/memory/abstraction/memory_view_latest.json",
    );
    let receipts_path = resolve_path(
        root,
        paths.get("receipts_path").and_then(|v| v.as_str()),
        "local/state/client/memory/abstraction/memory_view_receipts.jsonl",
    );
    json!({
      "enabled": raw.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true),
      "default_limit": default_limit,
      "latest_path": latest_path,
      "receipts_path": receipts_path
    })
}

fn cmd_memory_view(root: &Path, subcmd: &str, flags: &HashMap<String, String>) -> Value {
    let p = memory_view_policy(root);
    if p.get("enabled").and_then(|v| v.as_bool()) != Some(true) {
        return json!({"ok": false, "error": "memory_abstraction_view_disabled"});
    }
    let latest_path = PathBuf::from(p.get("latest_path").and_then(|v| v.as_str()).unwrap_or(""));
    let receipts_path = PathBuf::from(
        p.get("receipts_path")
            .and_then(|v| v.as_str())
            .unwrap_or(""),
    );
    let default_limit = p.get("default_limit").and_then(|v| v.as_u64()).unwrap_or(5) as u32;

    let receipt = match subcmd {
        "query" => {
            let query = clean_text(
                flags
                    .get("query")
                    .or_else(|| flags.get("q"))
                    .map(String::as_str)
                    .unwrap_or(""),
                400,
            );
            let limit = flags
                .get("limit")
                .or_else(|| flags.get("top"))
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(default_limit)
                .max(1);
            let run = run_memory_core(
                root,
                &[
                    format!("recall"),
                    format!("--query={query}"),
                    format!("--limit={limit}"),
                ],
            );
            let payload = run.payload.clone();
            let hits = payload
                .get("hits")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            json!({
              "ts": now_iso(),
              "type": "memory_view_query",
              "ok": run.ok,
              "backend": "rust_core_v6",
              "engine": if run.ok { Value::String("rust_core".to_string()) } else { Value::Null },
              "query": query,
              "limit": limit,
              "hit_count": payload.get("hit_count").and_then(|v| v.as_u64()).unwrap_or(hits.len() as u64),
              "hits": hits,
              "error": if run.ok { Value::Null } else { Value::String(clean_text(&run.error, 280)) }
            })
        }
        "get" => {
            let id = clean_text(flags.get("id").map(String::as_str).unwrap_or(""), 200);
            let run = run_memory_core(root, &[format!("get"), format!("--id={id}")]);
            let payload = run.payload.clone();
            json!({
              "ts": now_iso(),
              "type": "memory_view_get",
              "ok": run.ok,
              "backend": "rust_core_v6",
              "engine": if run.ok { Value::String("rust_core".to_string()) } else { Value::Null },
              "id": id,
              "row": payload.get("row").cloned().unwrap_or(Value::Null),
              "error": if run.ok { Value::Null } else { Value::String(clean_text(&run.error, 280)) }
            })
        }
        "snapshot" => {
            let query = clean_text(
                flags
                    .get("query")
                    .or_else(|| flags.get("q"))
                    .map(String::as_str)
                    .unwrap_or("memory"),
                200,
            );
            let limit = flags
                .get("limit")
                .or_else(|| flags.get("top"))
                .and_then(|v| v.parse::<u32>().ok())
                .unwrap_or(default_limit)
                .max(1);
            let recall_run = run_memory_core(
                root,
                &[
                    format!("recall"),
                    format!("--query={query}"),
                    format!("--limit={limit}"),
                ],
            );
            let obs_run =
                run_memory_core(root, &[String::from("load-embedded-observability-profile")]);
            let vault_run = run_memory_core(root, &[String::from("load-embedded-vault-policy")]);

            let recall_payload = recall_run.payload.clone();
            let hits = recall_payload
                .get("hits")
                .and_then(|v| v.as_array())
                .cloned()
                .unwrap_or_default();
            let mut ratios = Vec::new();
            for hit in &hits {
                if let Some(v) = hit.get("compression_ratio").and_then(|n| n.as_f64()) {
                    if v.is_finite() && v >= 0.0 {
                        ratios.push(v);
                    }
                }
            }
            let avg = if ratios.is_empty() {
                0.0
            } else {
                (ratios.iter().sum::<f64>() / ratios.len() as f64 * 1_000_000.0).round()
                    / 1_000_000.0
            };
            let ok = recall_run.ok && obs_run.ok && vault_run.ok;
            let mut errs = Vec::new();
            if !recall_run.ok {
                errs.push("recall_failed".to_string());
            }
            if !obs_run.ok {
                errs.push("observability_blob_failed".to_string());
            }
            if !vault_run.ok {
                errs.push("vault_blob_failed".to_string());
            }
            json!({
              "ts": now_iso(),
              "type": "memory_view_snapshot",
              "ok": ok,
              "backend": "rust_core_v6",
              "query": query,
              "limit": limit,
              "hit_count": recall_payload.get("hit_count").and_then(|v| v.as_u64()).unwrap_or(hits.len() as u64),
              "avg_compression_ratio": avg,
              "observability_profile": obs_run.payload.get("embedded_observability_profile").cloned().unwrap_or(Value::Null),
              "vault_policy": vault_run.payload.get("embedded_vault_policy").cloned().unwrap_or(Value::Null),
              "error": if ok { Value::Null } else { Value::String(clean_text(&errs.join("; "), 320)) }
            })
        }
        "status" => json!({
          "ok": true,
          "type": "memory_view_status",
          "latest": read_json(&latest_path)
        }),
        _ => json!({"ok": false, "error": "unsupported_command", "cmd": subcmd}),
    };

    if subcmd != "status" && receipt.get("type").is_some() {
        write_json_atomic(&latest_path, &receipt);
        append_jsonl(&receipts_path, &receipt);
    }
    receipt
}
