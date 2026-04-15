#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rag_search_promotes_positional_query() {
        let inv = build_invocation(&[
            "search".to_string(),
            "what".to_string(),
            "is".to_string(),
            "conduit".to_string(),
            "--top=3".to_string(),
        ])
        .expect("invocation");
        match inv {
            Invocation::MemoryRun {
                memory_command,
                memory_args,
            } => {
                assert_eq!(memory_command, "stable-rag-search");
                assert!(memory_args.iter().any(|v| v == "--top=3"));
                assert!(memory_args.iter().any(|v| v == "--q=what is conduit"));
            }
            _ => panic!("expected memory run"),
        }
    }

    #[test]
    fn ingest_promotes_first_positional_path() {
        let inv =
            build_invocation(&["ingest".to_string(), "docs/rag".to_string()]).expect("invocation");
        match inv {
            Invocation::MemoryRun { memory_args, .. } => {
                assert!(memory_args.iter().any(|v| v == "--path=docs/rag"));
            }
            _ => panic!("expected memory run"),
        }
    }

    #[test]
    fn memory_library_enable_stable_routes_to_stable_status() {
        let inv = build_invocation(&[
            "memory".to_string(),
            "library".to_string(),
            "enable".to_string(),
            "stable".to_string(),
        ])
        .expect("invocation");
        match inv {
            Invocation::MemoryRun {
                memory_command,
                memory_args,
            } => {
                assert_eq!(memory_command, "stable-status");
                assert!(memory_args.is_empty());
            }
            _ => panic!("expected memory run"),
        }
    }

    #[test]
    fn memory_upgrade_byterover_routes_to_stable_command() {
        let inv = build_invocation(&[
            "memory".to_string(),
            "upgrade".to_string(),
            "byterover".to_string(),
        ])
        .expect("invocation");
        match inv {
            Invocation::MemoryRun { memory_command, .. } => {
                assert_eq!(memory_command, "stable-memory-upgrade-byterover");
            }
            _ => panic!("expected memory run"),
        }
    }

    #[test]
    fn unknown_command_is_rejected() {
        let err = build_invocation(&["explode".to_string()]).expect_err("must fail");
        assert_eq!(err, "unknown_command");
    }

    #[test]
    fn memory_taxonomy_routes_to_stable_taxonomy() {
        let inv =
            build_invocation(&["memory".to_string(), "taxonomy".to_string()]).expect("invocation");
        match inv {
            Invocation::MemoryRun { memory_command, .. } => {
                assert_eq!(memory_command, "stable-memory-taxonomy");
            }
            _ => panic!("expected memory run"),
        }
    }

    #[test]
    fn memory_enable_causality_routes_to_stable_enable_command() {
        let inv = build_invocation(&[
            "memory".to_string(),
            "enable".to_string(),
            "causality".to_string(),
        ])
        .expect("invocation");
        match inv {
            Invocation::MemoryRun { memory_command, .. } => {
                assert_eq!(memory_command, "stable-memory-enable-causality");
            }
            _ => panic!("expected memory run"),
        }
    }

    #[test]
    fn memory_enable_metacognitive_routes_to_stable_enable_command() {
        let inv = build_invocation(&[
            "memory".to_string(),
            "enable".to_string(),
            "metacognitive".to_string(),
            "--note=reflect".to_string(),
        ])
        .expect("invocation");
        match inv {
            Invocation::MemoryRun {
                memory_command,
                memory_args,
            } => {
                assert_eq!(memory_command, "stable-memory-enable-metacognitive");
                assert!(memory_args.iter().any(|v| v == "--note=reflect"));
            }
            _ => panic!("expected memory run"),
        }
    }

    #[test]
    fn memory_share_routes_to_stable_share_command() {
        let inv = build_invocation(&[
            "memory".to_string(),
            "share".to_string(),
            "--persona=peer".to_string(),
            "--scope=task".to_string(),
            "--consent=true".to_string(),
        ])
        .expect("invocation");
        match inv {
            Invocation::MemoryRun { memory_command, .. } => {
                assert_eq!(memory_command, "stable-memory-share");
            }
            _ => panic!("expected memory run"),
        }
    }

    #[test]
    fn memory_evolve_routes_to_stable_evolve_command() {
        let inv = build_invocation(&[
            "memory".to_string(),
            "evolve".to_string(),
            "--generation=5".to_string(),
        ])
        .expect("invocation");
        match inv {
            Invocation::MemoryRun {
                memory_command,
                memory_args,
            } => {
                assert_eq!(memory_command, "stable-memory-evolve");
                assert!(memory_args.iter().any(|v| v == "--generation=5"));
            }
            _ => panic!("expected memory run"),
        }
    }

    #[test]
    fn memory_causal_retrieve_routes_to_stable_command() {
        let inv = build_invocation(&[
            "memory".to_string(),
            "causal-retrieve".to_string(),
            "--q=policy".to_string(),
            "--depth=3".to_string(),
        ])
        .expect("invocation");
        match inv {
            Invocation::MemoryRun {
                memory_command,
                memory_args,
            } => {
                assert_eq!(memory_command, "stable-memory-causal-retrieve");
                assert!(memory_args.iter().any(|v| v == "--q=policy"));
                assert!(memory_args.iter().any(|v| v == "--depth=3"));
            }
            _ => panic!("expected memory run"),
        }
    }

    #[test]
    fn memory_fuse_routes_to_stable_fuse_command() {
        let inv = build_invocation(&["memory".to_string(), "fuse".to_string()]).expect("inv");
        match inv {
            Invocation::MemoryRun { memory_command, .. } => {
                assert_eq!(memory_command, "stable-memory-fuse");
            }
            _ => panic!("expected memory run"),
        }
    }

    #[test]
    fn memory_benchmark_ama_routes_to_stable_benchmark_command() {
        let inv = build_invocation(&[
            "memory".to_string(),
            "benchmark".to_string(),
            "ama".to_string(),
            "--threshold=0.8".to_string(),
        ])
        .expect("invocation");
        match inv {
            Invocation::MemoryRun {
                memory_command,
                memory_args,
            } => {
                assert_eq!(memory_command, "stable-memory-benchmark-ama");
                assert!(memory_args.iter().any(|v| v == "--threshold=0.8"));
            }
            _ => panic!("expected memory run"),
        }
    }

    #[test]
    fn chat_nano_routes_to_stable_nano_chat() {
        let inv = build_invocation(&[
            "chat".to_string(),
            "nano".to_string(),
            "--q=teach me".to_string(),
        ])
        .expect("invocation");
        match inv {
            Invocation::MemoryRun {
                memory_command,
                memory_args,
            } => {
                assert_eq!(memory_command, "stable-nano-chat");
                assert!(memory_args.iter().any(|v| v == "--q=teach me"));
            }
            _ => panic!("expected memory run"),
        }
    }

    #[test]
    fn train_nano_routes_to_stable_nano_train() {
        let inv = build_invocation(&[
            "train".to_string(),
            "nano".to_string(),
            "--depth=12".to_string(),
        ])
        .expect("invocation");
        match inv {
            Invocation::MemoryRun {
                memory_command,
                memory_args,
            } => {
                assert_eq!(memory_command, "stable-nano-train");
                assert!(memory_args.iter().any(|v| v == "--depth=12"));
            }
            _ => panic!("expected memory run"),
        }
    }

    #[test]
    fn nano_fork_routes_to_stable_nano_fork() {
        let inv = build_invocation(&[
            "nano".to_string(),
            "fork".to_string(),
            "--target=.nanochat/fork".to_string(),
        ])
        .expect("invocation");
        match inv {
            Invocation::MemoryRun {
                memory_command,
                memory_args,
            } => {
                assert_eq!(memory_command, "stable-nano-fork");
                assert!(memory_args.iter().any(|v| v == "--target=.nanochat/fork"));
            }
            _ => panic!("expected memory run"),
        }
    }
}
