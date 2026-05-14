#[path = "../coding_memory_runtime_bridge.rs"]
mod coding_memory_runtime_bridge;
#[path = "../eval_local_context_pack_builder.rs"]
mod eval_local_context_pack_builder;

use eval_local_context_pack_builder::{judge_context_pack_batch, seed_context_pack_batch};
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let args = std::env::args().skip(1).collect::<Vec<_>>();
    let command = args.first().map(String::as_str).unwrap_or("seed");
    match command {
        "seed" => {
            let attempts = args
                .iter()
                .find_map(|arg| arg.strip_prefix("--attempts="))
                .and_then(|raw| raw.parse::<usize>().ok())
                .unwrap_or(5);
            let report = seed_context_pack_batch(attempts);
            println!(
                "{}",
                serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
            );
            ExitCode::from((!report.ok) as u8)
        }
        "judge" => {
            let batch_root = args
                .iter()
                .find_map(|arg| arg.strip_prefix("--batch-root="))
                .map(PathBuf::from)
                .unwrap_or_default();
            let report = judge_context_pack_batch(&batch_root);
            println!(
                "{}",
                serde_json::to_string_pretty(&report).unwrap_or_else(|_| "{}".to_string())
            );
            ExitCode::from((!report.ok) as u8)
        }
        _ => {
            eprintln!(
                "usage: local_context_pack_builder_eval_execute seed --attempts=N | judge --batch-root=PATH"
            );
            ExitCode::from(2)
        }
    }
}
