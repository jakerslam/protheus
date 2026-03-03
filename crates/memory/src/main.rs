use protheus_memory_core_v6::{
    clear_cache, compress_store, crdt_exchange_json, ebbinghaus_curve, get_json, ingest_memory,
    recall_json,
};
use serde_json::json;
use std::collections::HashMap;
use std::env;

fn parse_args(raw: &[String]) -> HashMap<String, String> {
    let mut out = HashMap::new();
    for token in raw {
        if !token.starts_with("--") {
            continue;
        }
        if let Some(eq) = token.find('=') {
            let key = token[2..eq].to_string();
            let value = token[eq + 1..].to_string();
            out.insert(key, value);
        } else {
            out.insert(token[2..].to_string(), "1".to_string());
        }
    }
    out
}

fn parse_bool(v: Option<&String>, fallback: bool) -> bool {
    match v.map(|s| s.trim().to_lowercase()) {
        Some(raw) if matches!(raw.as_str(), "1" | "true" | "yes" | "on") => true,
        Some(raw) if matches!(raw.as_str(), "0" | "false" | "no" | "off") => false,
        _ => fallback,
    }
}

fn parse_u32(v: Option<&String>, fallback: u32) -> u32 {
    v.and_then(|s| s.parse::<u32>().ok()).unwrap_or(fallback)
}

fn parse_f64(v: Option<&String>, fallback: f64) -> f64 {
    v.and_then(|s| s.parse::<f64>().ok()).unwrap_or(fallback)
}

fn print_json(value: serde_json::Value) {
    println!(
        "{}",
        serde_json::to_string_pretty(&value).unwrap_or_else(|_| "{\"ok\":false}".to_string())
    );
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("help");
    let flags = parse_args(&args);

    match command {
        "help" | "--help" | "-h" => {
            print_json(json!({
              "ok": true,
              "commands": [
                "recall --query=<text> --limit=<n>",
                "compress --aggressive=0|1",
                "ingest --id=<id> --content=<text> [--tags=t1,t2] [--repetitions=1] [--lambda=0.02]",
                "get --id=<id>",
                "clear-cache",
                "ebbinghaus-score --age-days=<n> [--repetitions=1] [--lambda=0.02]",
                "crdt-exchange --payload=<json>"
              ]
            }));
        }
        "recall" => {
            let q = flags.get("query").cloned().unwrap_or_else(|| "".to_string());
            let limit = parse_u32(flags.get("limit"), 5);
            let payload = recall_json(&q, limit);
            let parsed = serde_json::from_str::<serde_json::Value>(&payload)
                .unwrap_or_else(|_| json!({"ok": false, "error": "invalid_recall_payload"}));
            print_json(parsed);
        }
        "compress" => {
            let aggressive = parse_bool(flags.get("aggressive"), false);
            match compress_store(aggressive) {
                Ok(compacted) => print_json(json!({
                  "ok": true,
                  "aggressive": aggressive,
                  "compacted_rows": compacted
                })),
                Err(err) => print_json(json!({
                  "ok": false,
                  "error": err
                })),
            }
        }
        "ingest" => {
            let id = flags
                .get("id")
                .cloned()
                .unwrap_or_else(|| format!("memory://{}", uuid_like_seed()));
            let content = flags.get("content").cloned().unwrap_or_default();
            let tags = flags
                .get("tags")
                .map(|s| {
                    s.split(',')
                        .map(|v| v.trim().to_string())
                        .filter(|v| !v.is_empty())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            let repetitions = parse_u32(flags.get("repetitions"), 1);
            let lambda = parse_f64(flags.get("lambda"), 0.02);
            match ingest_memory(&id, &content, tags, repetitions, lambda) {
                Ok(row) => print_json(json!({
                  "ok": true,
                  "row": row
                })),
                Err(err) => print_json(json!({
                  "ok": false,
                  "error": err
                })),
            }
        }
        "get" => {
            let id = flags.get("id").cloned().unwrap_or_default();
            let payload = get_json(&id);
            let parsed = serde_json::from_str::<serde_json::Value>(&payload)
                .unwrap_or_else(|_| json!({"ok": false, "error": "invalid_get_payload"}));
            print_json(parsed);
        }
        "clear-cache" => match clear_cache() {
            Ok(cleared) => print_json(json!({
              "ok": true,
              "cleared": cleared
            })),
            Err(err) => print_json(json!({
              "ok": false,
              "error": err
            })),
        },
        "ebbinghaus-score" => {
            let age_days = parse_f64(flags.get("age-days"), 0.0);
            let repetitions = parse_u32(flags.get("repetitions"), 1);
            let lambda = parse_f64(flags.get("lambda"), 0.02);
            print_json(ebbinghaus_curve(age_days, repetitions, lambda));
        }
        "crdt-exchange" => {
            let payload = flags
                .get("payload")
                .cloned()
                .unwrap_or_else(|| "{\"left\":{},\"right\":{}}".to_string());
            match crdt_exchange_json(&payload) {
                Ok(encoded) => {
                    let parsed = serde_json::from_str::<serde_json::Value>(&encoded)
                        .unwrap_or_else(|_| json!({"ok": false, "error": "invalid_crdt_payload"}));
                    print_json(parsed);
                }
                Err(err) => print_json(json!({
                  "ok": false,
                  "error": err
                })),
            }
        }
        _ => {
            print_json(json!({
              "ok": false,
              "error": "unsupported_command",
              "command": command
            }));
            std::process::exit(1);
        }
    }
}

fn uuid_like_seed() -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    hasher.update(format!("{:?}", std::time::SystemTime::now()));
    let digest = hasher.finalize();
    hex::encode(&digest[..8])
}
