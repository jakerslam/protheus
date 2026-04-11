// SPDX-License-Identifier: Apache-2.0
use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use protheus_swarm_core_v1::{orchestrate_swarm, orchestrate_swarm_json, SwarmRequest};
use std::env;
use std::fs;
use std::io::{self, Read};

fn parse_arg(args: &[String], key: &str) -> Option<String> {
    for arg in args {
        if let Some((k, v)) = arg.split_once('=') {
            if k == key {
                return Some(v.to_string());
            }
        }
    }
    None
}

fn read_stdin_request() -> Result<String, String> {
    let mut payload = String::new();
    io::stdin()
        .read_to_string(&mut payload)
        .map_err(|e| format!("stdin_read_failed:{e}"))?;
    if payload.trim().is_empty() {
        Err("stdin_request_empty".to_string())
    } else {
        Ok(payload)
    }
}

fn request_source_count(
    request_json: Option<&str>,
    request_base64: Option<&str>,
    request_file: Option<&str>,
    request_stdin: bool,
) -> usize {
    let file_source = request_file.map(|v| v != "-").unwrap_or(false);
    let stdin_source = request_stdin || request_file == Some("-");
    [
        request_json.is_some(),
        request_base64.is_some(),
        file_source,
        stdin_source,
    ]
    .into_iter()
    .filter(|v| *v)
    .count()
}

fn load_request(args: &[String]) -> Result<String, String> {
    let request_json = parse_arg(args, "--request-json");
    let request_base64 = parse_arg(args, "--request-base64");
    let request_file = parse_arg(args, "--request-file");
    let request_stdin =
        args.iter().any(|arg| arg == "--request-stdin") || request_file.as_deref() == Some("-");

    if request_source_count(
        request_json.as_deref(),
        request_base64.as_deref(),
        request_file.as_deref(),
        request_stdin,
    ) != 1
    {
        return Err("expected_exactly_one_request_source".to_string());
    }

    if let Some(v) = request_json {
        return Ok(v);
    }
    if let Some(v) = request_base64 {
        let bytes = BASE64_STANDARD
            .decode(v.as_bytes())
            .map_err(|e| format!("base64_decode_failed:{e}"))?;
        let text = String::from_utf8(bytes).map_err(|e| format!("utf8_decode_failed:{e}"))?;
        return Ok(text);
    }
    if let Some(v) = request_file {
        if v == "-" {
            return read_stdin_request();
        }
        return fs::read_to_string(v.as_str()).map_err(|e| format!("request_file_read_failed:{e}"));
    }
    read_stdin_request()
}

fn demo_request() -> SwarmRequest {
    SwarmRequest {
        swarm_id: "swarm_demo".to_string(),
        mode: "deterministic".to_string(),
        agents: vec![
            protheus_swarm_core_v1::SwarmAgent {
                id: "a1".to_string(),
                skills: vec!["research".to_string(), "coding".to_string()],
                capacity: 3,
                reliability_pct: 91.0,
            },
            protheus_swarm_core_v1::SwarmAgent {
                id: "a2".to_string(),
                skills: vec!["coding".to_string()],
                capacity: 2,
                reliability_pct: 84.0,
            },
        ],
        tasks: vec![
            protheus_swarm_core_v1::SwarmTask {
                id: "t1".to_string(),
                required_skill: "coding".to_string(),
                weight: 2,
                priority: 8,
            },
            protheus_swarm_core_v1::SwarmTask {
                id: "t2".to_string(),
                required_skill: "research".to_string(),
                weight: 1,
                priority: 6,
            },
        ],
    }
}

fn usage() {
    eprintln!("Usage:");
    eprintln!("  swarm_core run --request-json=<payload>");
    eprintln!("  swarm_core run --request-base64=<payload>");
    eprintln!("  swarm_core run --request-file=<path>");
    eprintln!("  swarm_core run --request-stdin");
    eprintln!("  swarm_core demo");
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("demo");

    match command {
        "run" => match load_request(&args[1..]) {
            Ok(payload) => match orchestrate_swarm_json(&payload) {
                Ok(v) => println!("{}", v),
                Err(err) => {
                    eprintln!("{}", serde_json::json!({"ok": false, "error": err}));
                    std::process::exit(1);
                }
            },
            Err(err) => {
                eprintln!("{}", serde_json::json!({"ok": false, "error": err}));
                std::process::exit(1);
            }
        },
        "demo" => {
            let receipt = orchestrate_swarm(&demo_request()).expect("demo");
            println!(
                "{}",
                serde_json::to_string(&receipt).unwrap_or_else(|_| "{}".to_string())
            );
        }
        _ => {
            usage();
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::request_source_count;

    #[test]
    fn request_sources_fail_when_ambiguous() {
        assert_eq!(
            request_source_count(Some("{}"), None, Some("payload.json"), false),
            2
        );
    }

    #[test]
    fn stdin_file_sentinel_counts_as_single_source() {
        assert_eq!(request_source_count(None, None, Some("-"), false), 1);
        assert_eq!(request_source_count(None, None, None, true), 1);
    }
}
