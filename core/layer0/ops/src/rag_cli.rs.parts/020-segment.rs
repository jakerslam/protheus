fn build_invocation(argv: &[String]) -> Result<Invocation, String> {
    let section = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    match section.as_str() {
        "status" => Ok(Invocation::MemoryRun {
            memory_command: "rag-status".to_string(),
            memory_args: vec![],
        }),
        "start" => Ok(Invocation::MemoryRun {
            memory_command: "stable-status".to_string(),
            memory_args: vec![],
        }),
        "ingest" => Ok(Invocation::MemoryRun {
            memory_command: "stable-rag-ingest".to_string(),
            memory_args: normalize_ingest_args(&argv.iter().skip(1).cloned().collect::<Vec<_>>()),
        }),
        "search" => Ok(Invocation::MemoryRun {
            memory_command: "stable-rag-search".to_string(),
            memory_args: normalize_search_args(&argv.iter().skip(1).cloned().collect::<Vec<_>>()),
        }),
        "chat" => Ok(Invocation::MemoryRun {
            memory_command: if argv
                .get(1)
                .map(|v| v.trim().eq_ignore_ascii_case("nano"))
                .unwrap_or(false)
            {
                "stable-nano-chat".to_string()
            } else {
                "stable-rag-chat".to_string()
            },
            memory_args: normalize_search_args(
                &argv
                    .iter()
                    .skip(
                        if argv
                            .get(1)
                            .map(|v| v.trim().eq_ignore_ascii_case("nano"))
                            .unwrap_or(false)
                        {
                            2
                        } else {
                            1
                        },
                    )
                    .cloned()
                    .collect::<Vec<_>>(),
            ),
        }),
        "train" => {
            let target = argv
                .get(1)
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_default();
            if target == "nano" {
                Ok(Invocation::MemoryRun {
                    memory_command: "stable-nano-train".to_string(),
                    memory_args: argv.iter().skip(2).cloned().collect(),
                })
            } else {
                Err("train_unknown_target".to_string())
            }
        }
        "nano" => {
            let action = argv
                .get(1)
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "chat".to_string());
            match action.as_str() {
                "fork" => Ok(Invocation::MemoryRun {
                    memory_command: "stable-nano-fork".to_string(),
                    memory_args: argv.iter().skip(2).cloned().collect(),
                }),
                "chat" => Ok(Invocation::MemoryRun {
                    memory_command: "stable-nano-chat".to_string(),
                    memory_args: normalize_search_args(
                        &argv.iter().skip(2).cloned().collect::<Vec<_>>(),
                    ),
                }),
                _ => Err("nano_unknown_action".to_string()),
            }
        }
        "merge" | "merge-vault" => Ok(Invocation::MemoryRun {
            memory_command: "rag-merge-vault".to_string(),
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
                Err("upgrade_unknown_target".to_string())
            }
        }
        "ambient-status" => Ok(Invocation::AmbientStatus),
        "memory" => {
            build_memory_library_invocation(&argv.iter().skip(1).cloned().collect::<Vec<_>>())
        }
        "help" | "--help" | "-h" => Err("help".to_string()),
        _ => Err("unknown_command".to_string()),
    }
}

fn cli_error_receipt(root: &Path, argv: &[String], error: &str, exit_code: i32) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "rag_cli_error",
        "ts": now_iso(),
        "root": root.to_string_lossy().to_string(),
        "argv": argv,
        "error": error,
        "exit_code": exit_code
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let invocation = match build_invocation(argv) {
        Ok(v) => v,
        Err(reason) if reason == "help" => {
            usage();
            return 0;
        }
        Err(reason) => {
            usage();
            print_json(&cli_error_receipt(root, argv, &reason, 2));
            return 2;
        }
    };

    match invocation {
        Invocation::AmbientStatus => memory_ambient::run(root, &["status".to_string()]),
        Invocation::MemoryRun {
            memory_command,
            memory_args,
        } => {
            let mut args = vec![
                "run".to_string(),
                format!("--memory-command={memory_command}"),
            ];
            for row in memory_args {
                let trimmed = clean(&row, 1600);
                if !trimmed.is_empty() {
                    args.push(format!("--memory-arg={trimmed}"));
                }
            }
            memory_ambient::run(root, &args)
        }
    }
}

