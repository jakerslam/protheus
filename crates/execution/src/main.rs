use base64::engine::general_purpose::STANDARD as BASE64_STANDARD;
use base64::Engine;
use execution_core::{run_workflow, run_workflow_json};
use std::env;
use std::fs;

fn usage() {
    eprintln!("Usage:");
    eprintln!("  execution_core run --yaml=<payload>");
    eprintln!("  execution_core run --yaml-base64=<base64_payload>");
    eprintln!("  execution_core run --yaml-file=<path>");
    eprintln!("  execution_core demo");
}

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

fn load_yaml(args: &[String]) -> Result<String, String> {
    if let Some(v) = parse_arg(args, "--yaml") {
        return Ok(v);
    }
    if let Some(v) = parse_arg(args, "--yaml-base64") {
        let bytes = BASE64_STANDARD
            .decode(v.as_bytes())
            .map_err(|err| format!("base64_decode_failed:{}", err))?;
        let text = String::from_utf8(bytes).map_err(|err| format!("utf8_decode_failed:{}", err))?;
        return Ok(text);
    }
    if let Some(v) = parse_arg(args, "--yaml-file") {
        let content = fs::read_to_string(v.as_str())
            .map_err(|err| format!("yaml_file_read_failed:{}", err))?;
        return Ok(content);
    }
    Err("missing_yaml_payload".to_string())
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let command = args.first().map(String::as_str).unwrap_or("demo");

    match command {
        "run" => match load_yaml(&args[1..]) {
            Ok(yaml) => {
                println!("{}", run_workflow_json(&yaml));
            }
            Err(err) => {
                let payload = serde_json::json!({ "ok": false, "error": err });
                eprintln!("{}", payload);
                std::process::exit(1);
            }
        },
        "demo" => {
            let demo = serde_json::json!({
                "workflow_id": "execution_demo",
                "deterministic_seed": "demo_seed",
                "pause_after_step": "score",
                "steps": [
                    {
                        "id": "collect",
                        "kind": "task",
                        "action": "collect_data",
                        "command": "collect --source=eyes"
                    },
                    {
                        "id": "score",
                        "kind": "task",
                        "action": "score",
                        "command": "score --strategy=deterministic"
                    },
                    {
                        "id": "ship",
                        "kind": "task",
                        "action": "ship",
                        "command": "ship --mode=canary"
                    }
                ]
            })
            .to_string();
            let receipt = run_workflow(&demo);
            println!(
                "{}",
                serde_json::to_string(&receipt).unwrap_or_else(|_| "{\"ok\":false}".to_string())
            );
        }
        _ => {
            usage();
            std::process::exit(1);
        }
    }
}
