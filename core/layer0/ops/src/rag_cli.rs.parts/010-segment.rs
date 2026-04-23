// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)
use crate::{deterministic_receipt_hash, memory_ambient, now_iso};
use serde_json::{json, Value};
use std::path::Path;

#[derive(Debug, Clone)]
enum Invocation {
    AmbientStatus,
    MemoryRun {
        memory_command: String,
        memory_args: Vec<String>,
    },
}

fn usage() {
    eprintln!("Usage:");
    eprintln!("  infring-ops rag status");
    eprintln!("  infring-ops rag start");
    eprintln!("  infring-ops rag ingest [--path=<path>] [--chunk-size=<n>] [--chunk-overlap=<n>]");
    eprintln!("  infring-ops rag search --q=<query> [--top=<n>]");
    eprintln!("  infring-ops rag chat --q=<query> [--top=<n>]");
    eprintln!("  infring-ops rag chat nano [--q=<query>] [--top=<n>]");
    eprintln!("  infring-ops rag train nano [--depth=<n>] [--profile=<name>]");
    eprintln!("  infring-ops rag nano fork [--target=<path>]");
    eprintln!("  infring-ops rag merge-vault [--max-merge=<n>]");
    eprintln!("  infring-ops rag memory status");
    eprintln!("  infring-ops rag memory search --q=<query> [--top=<n>]");
    eprintln!("  infring-ops rag memory get-node --node-id=<id> | --uid=<uid>");
    eprintln!("  infring-ops rag memory build-index");
    eprintln!("  infring-ops rag memory upgrade byterover");
    eprintln!("  infring-ops rag memory taxonomy");
    eprintln!("  infring-ops rag memory enable metacognitive");
    eprintln!("  infring-ops rag memory enable causality");
    eprintln!("  infring-ops rag memory benchmark ama");
    eprintln!("  infring-ops rag memory share --persona=<id> --scope=task|step --consent=true");
    eprintln!("  infring-ops rag memory evolve [--generation=<n>]");
    eprintln!("  infring-ops rag memory causal-retrieve --q=<query> [--depth=<n>]");
    eprintln!("  infring-ops rag memory fuse");
    eprintln!("  infring-ops rag memory library enable stable");
}

fn print_json(value: &Value) {
    println!(
        "{}",
        serde_json::to_string(value).unwrap_or_else(|_| "{\"ok\":false}".to_string())
    );
}

fn clean(v: &str, max_len: usize) -> String {
    v.split_whitespace()
        .collect::<Vec<&str>>()
        .join(" ")
        .chars()
        .take(max_len)
        .collect::<String>()
        .trim()
        .to_string()
}

fn has_flag(argv: &[String], key: &str) -> bool {
    let exact = format!("--{key}");
    let pref = format!("--{key}=");
    argv.iter()
        .any(|row| row == &exact || row.starts_with(&pref))
}

fn normalize_search_args(argv: &[String]) -> Vec<String> {
    if has_flag(argv, "q") {
        return argv.to_vec();
    }
    let mut flags = Vec::new();
    let mut positional = Vec::new();
    for row in argv {
        if row.starts_with("--") {
            flags.push(row.clone());
        } else {
            positional.push(row.clone());
        }
    }
    if !positional.is_empty() {
        let query = clean(&positional.join(" "), 1600);
        if !query.is_empty() {
            flags.push(format!("--q={query}"));
        }
    }
    flags
}

fn normalize_ingest_args(argv: &[String]) -> Vec<String> {
    if has_flag(argv, "path") {
        return argv.to_vec();
    }
    let mut out = Vec::new();
    let mut used_path = false;
    for row in argv {
        if !used_path && !row.starts_with("--") {
            out.push(format!("--path={}", clean(row, 600)));
            used_path = true;
        } else {
            out.push(row.clone());
        }
    }
    out
}

fn build_memory_library_invocation(argv: &[String]) -> Result<Invocation, String> {
    if argv.is_empty() {
        return Ok(Invocation::MemoryRun {
            memory_command: "stable-status".to_string(),
            memory_args: vec![],
        });
    }
    let section = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_default();
    match section.as_str() {
        "status" => Ok(Invocation::MemoryRun {
            memory_command: "stable-status".to_string(),
            memory_args: vec![],
        }),
        "search" => Ok(Invocation::MemoryRun {
            memory_command: "stable-search".to_string(),
            memory_args: normalize_search_args(&argv[1..]),
        }),
        "get-node" => Ok(Invocation::MemoryRun {
            memory_command: "stable-get-node".to_string(),
            memory_args: argv.iter().skip(1).cloned().collect(),
        }),
        "build-index" => Ok(Invocation::MemoryRun {
            memory_command: "stable-build-index".to_string(),
            memory_args: argv.iter().skip(1).cloned().collect(),
        }),
        "taxonomy" => Ok(Invocation::MemoryRun {
            memory_command: "stable-memory-taxonomy".to_string(),
            memory_args: argv.iter().skip(1).cloned().collect(),
        }),
        "enable" => {
            let target = argv
                .get(1)
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_default();
            match target.as_str() {
                "metacognitive" => Ok(Invocation::MemoryRun {
                    memory_command: "stable-memory-enable-metacognitive".to_string(),
                    memory_args: argv.iter().skip(2).cloned().collect(),
                }),
                "causality" => Ok(Invocation::MemoryRun {
                    memory_command: "stable-memory-enable-causality".to_string(),
                    memory_args: argv.iter().skip(2).cloned().collect(),
                }),
                _ => Err("memory_enable_unknown_target".to_string()),
            }
        }
        "benchmark" => {
            let target = argv
                .get(1)
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_default();
            if target == "ama" {
                Ok(Invocation::MemoryRun {
                    memory_command: "stable-memory-benchmark-ama".to_string(),
                    memory_args: argv.iter().skip(2).cloned().collect(),
                })
            } else {
                Err("memory_benchmark_unknown_target".to_string())
            }
        }
        "share" => Ok(Invocation::MemoryRun {
            memory_command: "stable-memory-share".to_string(),
            memory_args: argv.iter().skip(1).cloned().collect(),
        }),
        "evolve" => Ok(Invocation::MemoryRun {
            memory_command: "stable-memory-evolve".to_string(),
            memory_args: argv.iter().skip(1).cloned().collect(),
        }),
        "causal-retrieve" => Ok(Invocation::MemoryRun {
            memory_command: "stable-memory-causal-retrieve".to_string(),
            memory_args: normalize_search_args(&argv.iter().skip(1).cloned().collect::<Vec<_>>()),
        }),
        "fuse" => Ok(Invocation::MemoryRun {
            memory_command: "stable-memory-fuse".to_string(),
            memory_args: argv.iter().skip(1).cloned().collect(),
        }),
        "upgrade" => {
            let target = argv
                .get(1)
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_default();
            if target == "byterover" {
                Ok(Invocation::MemoryRun {
                    memory_command: "stable-memory-upgrade-byterover".to_string(),
                    memory_args: argv.iter().skip(2).cloned().collect(),
                })
            } else {
                Err("memory_upgrade_unknown_target".to_string())
            }
        }
        "library" => {
            let action = argv
                .get(1)
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_default();
            let value = argv
                .get(2)
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_default();
            if action == "enable" && value == "stable" {
                Ok(Invocation::MemoryRun {
                    memory_command: "stable-status".to_string(),
                    memory_args: vec![],
                })
            } else {
                Err("memory_library_unknown_action".to_string())
            }
        }
        _ => Err("memory_unknown_command".to_string()),
    }
}

