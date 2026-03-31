fn main() {
    let args: Vec<String> = env::args().collect();
    let cmd = args.get(1).map(|s| s.as_str()).unwrap_or("probe");
    let kv = parse_kv_args(&args[2..]);

    match cmd {
        "probe" => run_probe(&kv),
        "query-index" => run_query_index(&kv),
        "get-node" => run_get_node(&kv),
        "build-index" => run_build_index(&kv),
        "verify-envelope" => run_verify_envelope(&kv),
        "set-hot-state" => run_set_hot_state(&kv),
        "get-hot-state" => run_get_hot_state(&kv),
        "memory-matrix" => std::process::exit(wave1::print_payload_and_exit_code(
            wave1::memory_matrix_payload(&kv),
        )),
        "memory-auto-recall" => std::process::exit(wave1::print_payload_and_exit_code(
            wave1::memory_auto_recall_payload(&kv),
        )),
        "dream-sequencer" => std::process::exit(wave1::print_payload_and_exit_code(
            wave1::dream_sequencer_payload(&kv),
        )),
        "rag-ingest" => run_value_payload(rag_runtime::ingest_payload(&kv)),
        "rag-search" => run_value_payload(rag_runtime::search_payload(&kv)),
        "rag-chat" => run_value_payload(rag_runtime::chat_payload(&kv)),
        "nano-chat" => run_value_payload(rag_runtime::nano_chat_payload(&kv)),
        "nano-train" => run_value_payload(rag_runtime::nano_train_payload(&kv)),
        "nano-fork" => run_value_payload(rag_runtime::nano_fork_payload(&kv)),
        "rag-status" => run_value_payload(rag_runtime::status_payload(&kv)),
        "rag-merge-vault" => run_value_payload(rag_runtime::merge_vault_payload(&kv)),
        "memory-upgrade-byterover" => {
            run_value_payload(rag_runtime::byterover_upgrade_payload(&kv))
        }
        "memory-taxonomy" => run_value_payload(rag_runtime::memory_taxonomy_payload(&kv)),
        "memory-enable-metacognitive" => {
            run_value_payload(rag_runtime::memory_metacognitive_enable_payload(&kv))
        }
        "memory-enable-causality" => {
            run_value_payload(rag_runtime::memory_causality_enable_payload(&kv))
        }
        "memory-benchmark-ama" => run_value_payload(rag_runtime::memory_benchmark_ama_payload(&kv)),
        "memory-share" => run_value_payload(rag_runtime::memory_share_payload(&kv)),
        "memory-evolve" => run_value_payload(rag_runtime::memory_evolve_payload(&kv)),
        "memory-causal-retrieve" => {
            run_value_payload(rag_runtime::memory_causal_retrieve_payload(&kv))
        }
        "memory-fuse" => run_value_payload(rag_runtime::memory_fuse_payload(&kv)),
        "predictive-defrag-status" => run_value_payload(predictive_defrag_status_payload(&kv)),
        "predictive-defrag-stress" => run_value_payload(predictive_defrag_stress_payload(&kv)),
        "stable-status" => run_value_payload(rag_runtime::stable_status_payload()),
        "stable-search" => match rag_runtime::ensure_supported_version(&kv) {
            Ok(version) => {
                let mut out = serde_json::to_value(query_index_payload(&kv))
                    .unwrap_or_else(|_| json!({"ok": false, "error": "query_serialize_failed"}));
                out["api_version"] = json!(version);
                run_value_payload(out);
            }
            Err(err) => run_value_payload(err),
        },
        "stable-get-node" => match rag_runtime::ensure_supported_version(&kv) {
            Ok(version) => {
                let (mut out, _code) = get_node_payload(&kv);
                out["api_version"] = json!(version);
                run_value_payload(out);
            }
            Err(err) => run_value_payload(err),
        },
        "stable-build-index" => match rag_runtime::ensure_supported_version(&kv) {
            Ok(version) => {
                let mut out = serde_json::to_value(build_index_payload(&kv))
                    .unwrap_or_else(|_| json!({"ok": false, "error": "build_serialize_failed"}));
                out["api_version"] = json!(version);
                run_value_payload(out);
            }
            Err(err) => run_value_payload(err),
        },
        "stable-rag-ingest" => match rag_runtime::ensure_supported_version(&kv) {
            Ok(version) => {
                let mut out = rag_runtime::ingest_payload(&kv);
                out["api_version"] = json!(version);
                run_value_payload(out);
            }
            Err(err) => run_value_payload(err),
        },
        "stable-rag-search" => match rag_runtime::ensure_supported_version(&kv) {
            Ok(version) => {
                let mut out = rag_runtime::search_payload(&kv);
                out["api_version"] = json!(version);
                run_value_payload(out);
            }
            Err(err) => run_value_payload(err),
        },
        "stable-rag-chat" => match rag_runtime::ensure_supported_version(&kv) {
            Ok(version) => {
                let mut out = rag_runtime::chat_payload(&kv);
                out["api_version"] = json!(version);
                run_value_payload(out);
            }
            Err(err) => run_value_payload(err),
        },
        "stable-nano-chat" => match rag_runtime::ensure_supported_version(&kv) {
            Ok(version) => {
                let mut out = rag_runtime::nano_chat_payload(&kv);
                out["api_version"] = json!(version);
                run_value_payload(out);
            }
            Err(err) => run_value_payload(err),
        },
        "stable-nano-train" => match rag_runtime::ensure_supported_version(&kv) {
            Ok(version) => {
                let mut out = rag_runtime::nano_train_payload(&kv);
                out["api_version"] = json!(version);
                run_value_payload(out);
            }
            Err(err) => run_value_payload(err),
        },
        "stable-nano-fork" => match rag_runtime::ensure_supported_version(&kv) {
            Ok(version) => {
                let mut out = rag_runtime::nano_fork_payload(&kv);
                out["api_version"] = json!(version);
                run_value_payload(out);
            }
            Err(err) => run_value_payload(err),
        },
        "stable-memory-upgrade-byterover" => match rag_runtime::ensure_supported_version(&kv) {
            Ok(version) => {
                let mut out = rag_runtime::byterover_upgrade_payload(&kv);
                out["api_version"] = json!(version);
                run_value_payload(out);
            }
            Err(err) => run_value_payload(err),
        },
        "stable-memory-taxonomy" => match rag_runtime::ensure_supported_version(&kv) {
            Ok(version) => {
                let mut out = rag_runtime::memory_taxonomy_payload(&kv);
                out["api_version"] = json!(version);
                run_value_payload(out);
            }
            Err(err) => run_value_payload(err),
        },
        "stable-memory-enable-metacognitive" => match rag_runtime::ensure_supported_version(&kv) {
            Ok(version) => {
                let mut out = rag_runtime::memory_metacognitive_enable_payload(&kv);
                out["api_version"] = json!(version);
                run_value_payload(out);
            }
            Err(err) => run_value_payload(err),
        },
        "stable-memory-enable-causality" => match rag_runtime::ensure_supported_version(&kv) {
            Ok(version) => {
                let mut out = rag_runtime::memory_causality_enable_payload(&kv);
                out["api_version"] = json!(version);
                run_value_payload(out);
            }
            Err(err) => run_value_payload(err),
        },
        "stable-memory-benchmark-ama" => match rag_runtime::ensure_supported_version(&kv) {
            Ok(version) => {
                let mut out = rag_runtime::memory_benchmark_ama_payload(&kv);
                out["api_version"] = json!(version);
                run_value_payload(out);
            }
            Err(err) => run_value_payload(err),
        },
        "stable-memory-share" => match rag_runtime::ensure_supported_version(&kv) {
            Ok(version) => {
                let mut out = rag_runtime::memory_share_payload(&kv);
                out["api_version"] = json!(version);
                run_value_payload(out);
            }
            Err(err) => run_value_payload(err),
        },
        "stable-memory-evolve" => match rag_runtime::ensure_supported_version(&kv) {
            Ok(version) => {
                let mut out = rag_runtime::memory_evolve_payload(&kv);
                out["api_version"] = json!(version);
                run_value_payload(out);
            }
            Err(err) => run_value_payload(err),
        },
        "stable-memory-causal-retrieve" => match rag_runtime::ensure_supported_version(&kv) {
            Ok(version) => {
                let mut out = rag_runtime::memory_causal_retrieve_payload(&kv);
                out["api_version"] = json!(version);
                run_value_payload(out);
            }
            Err(err) => run_value_payload(err),
        },
        "stable-memory-fuse" => match rag_runtime::ensure_supported_version(&kv) {
            Ok(version) => {
                let mut out = rag_runtime::memory_fuse_payload(&kv);
                out["api_version"] = json!(version);
                run_value_payload(out);
            }
            Err(err) => run_value_payload(err),
        },
        "daemon" => run_daemon(&kv),
        _ => {
            eprintln!("unsupported command: {}", cmd);
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod memory_policy_tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    fn write_daily_node(root: &Path, day: &str, node_id: &str, tags: &str) {
        let memory_dir = root.join("memory");
        fs::create_dir_all(&memory_dir).expect("create memory dir");
        let body = format!(
            r#"<!-- NODE -->
node_id: {node_id}
uid: UID{node_id}
tags: [{tags}]
# {node_id} summary
body
"#
        );
        fs::write(memory_dir.join(format!("{day}.md")), body).expect("write daily node");
    }

    fn as_map(pairs: &[(&str, &str)]) -> HashMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect::<HashMap<String, String>>()
    }

    #[test]
    fn query_rejects_when_budget_exceeds_in_reject_mode() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_daily_node(tmp.path(), "2026-03-01", "node.alpha", "memory,policy");
        let root = tmp.path().to_string_lossy().to_string();
        let args = as_map(&[
            ("root", root.as_str()),
            ("q", "memory"),
            ("top", "999"),
            ("budget-mode", "reject"),
        ]);
        let out = query_index_payload(&args);
        assert!(!out.ok);
        assert_eq!(out.reason_code.as_deref(), Some("recall_budget_exceeded"));
    }

    #[test]
    fn query_trims_budget_when_trim_mode_enabled() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_daily_node(tmp.path(), "2026-03-01", "node.alpha", "memory,policy");
        let root = tmp.path().to_string_lossy().to_string();
        let args = as_map(&[
            ("root", root.as_str()),
            ("q", "memory"),
            ("top", "999"),
            ("max-files", "99"),
            ("expand-lines", "999"),
            ("budget-mode", "trim"),
        ]);
        let out = query_index_payload(&args);
        assert!(out.ok);
        let policy = out.policy.expect("policy");
        assert_eq!(policy["budget"]["trimmed"], true);
        assert_eq!(policy["budget"]["effective"]["top"], 50);
        assert_eq!(policy["budget"]["effective"]["max_files"], 20);
        assert_eq!(policy["budget"]["effective"]["expand_lines"], 300);
    }

    #[test]
    fn query_fail_closed_when_index_is_stale() {
        let tmp = tempfile::tempdir().expect("tempdir");
        let index_dir = tmp.path().join("client/memory");
        fs::create_dir_all(&index_dir).expect("index dir");
        fs::write(
            index_dir.join("MEMORY_INDEX.md"),
            r#"# MEMORY_INDEX.md
| node_id | uid | tags | file | summary |
|---|---|---|---|---|
| `node.alpha` | `UIDALPHA` | #memory | `client/memory/2026-03-01.md` | alpha |
"#,
        )
        .expect("write memory index");
        thread::sleep(Duration::from_millis(1200));
        let root = tmp.path().to_string_lossy().to_string();
        let args = as_map(&[
            ("root", root.as_str()),
            ("q", "memory"),
            ("max-index-age-ms", "1000"),
            ("budget-mode", "trim"),
            ("disable-sqlite", "1"),
        ]);
        let out = query_index_payload(&args);
        assert!(!out.ok);
        assert_eq!(out.reason_code.as_deref(), Some("index_stale_blocked"));
    }

    #[test]
    fn query_fail_closed_when_burn_slo_exceeded() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_daily_node(tmp.path(), "2026-03-01", "node.alpha", "memory,policy");
        let root = tmp.path().to_string_lossy().to_string();
        let args = as_map(&[
            ("root", root.as_str()),
            ("q", "memory"),
            ("burn-threshold", "200"),
            ("burn-mode", "reject"),
            ("startup-token-estimate", "80"),
            ("hydration-token-estimate", "80"),
            ("response-token-estimate", "120"),
            ("budget-mode", "trim"),
        ]);
        let out = query_index_payload(&args);
        assert!(!out.ok);
        assert_eq!(out.reason_code.as_deref(), Some("burn_threshold_exceeded"));
        assert_eq!(out.burn_slo.expect("burn")["ok"], false);
    }

    #[test]
    fn bootstrap_guard_blocks_eager_hydration() {
        let tmp = tempfile::tempdir().expect("tempdir");
        write_daily_node(tmp.path(), "2026-03-01", "node.alpha", "memory,policy");
        let root = tmp.path().to_string_lossy().to_string();
        let args = as_map(&[
            ("root", root.as_str()),
            ("q", "memory"),
            ("bootstrap", "1"),
            ("hydrate-mode", "eager"),
            ("budget-mode", "trim"),
        ]);
        let out = query_index_payload(&args);
        assert!(!out.ok);
        assert_eq!(
            out.reason_code.as_deref(),
            Some("bootstrap_requires_lazy_hydration")
        );
    }

    #[test]
    fn get_node_is_fail_closed_without_node_or_uid() {
        let args = HashMap::new();
        let (out, code) = get_node_payload(&args);
        assert_eq!(code, 2);
        assert_eq!(out["ok"], false);
        assert_eq!(out["error"], "missing_node_or_uid");
    }
}
