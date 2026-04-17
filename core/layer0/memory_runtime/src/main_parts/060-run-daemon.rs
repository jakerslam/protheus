fn normalize_daemon_command(raw: &str) -> String {
    raw.trim()
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect::<String>()
        .to_ascii_lowercase()
}

fn daemon_execution_receipt(
    cmd: &str,
    stage: &str,
    ok: bool,
    error: Option<&str>,
    started: std::time::Instant,
) -> Value {
    json!({
        "status": if ok { "ok" } else { "error" },
        "cmd": cmd,
        "stage": stage,
        "duration_ms": started.elapsed().as_millis() as u64,
        "ts": now_iso(),
        "error": error
    })
}

fn daemon_error_response(
    cmd: &str,
    stage: &str,
    error: &str,
    started: std::time::Instant,
) -> Value {
    json!({
        "ok": false,
        "type": "memory_daemon_error",
        "cmd": cmd,
        "error": error,
        "execution_receipt": daemon_execution_receipt(cmd, stage, false, Some(error), started)
    })
}

fn daemon_write_response(stream: &mut impl Write, payload: &Value) {
    let body = serde_json::to_string(payload)
        .unwrap_or_else(|_| "{\"ok\":false,\"error\":\"serialize_failed\"}".to_string());
    let _ = stream.write_all(format!("{body}\n").as_bytes());
    let _ = stream.flush();
}

fn finalize_daemon_response(mut payload: Value, cmd: &str, started: std::time::Instant) -> Value {
    if let Some(obj) = payload.as_object_mut() {
        let ok = obj.get("ok").and_then(|v| v.as_bool()).unwrap_or(false);
        let error = obj.get("error").and_then(|v| v.as_str());
        obj.entry("type".to_string())
            .or_insert_with(|| Value::String("memory_daemon_response".to_string()));
        obj.entry("cmd".to_string())
            .or_insert_with(|| Value::String(cmd.to_string()));
        obj.entry("execution_receipt".to_string())
            .or_insert_with(|| daemon_execution_receipt(cmd, "dispatch", ok, error, started));
    }
    payload
}

fn run_daemon(args: &HashMap<String, String>) {
    let host = arg_or_default(args, "host", "127.0.0.1");
    let port_raw = arg_or_default(args, "port", "34127");
    let port = port_raw.parse::<u16>().unwrap_or(34127);
    let bind_addr = format!("{host}:{port}");
    let listener = TcpListener::bind(&bind_addr).unwrap_or_else(|_| {
        eprintln!("memory-daemon bind failed at {bind_addr}");
        std::process::exit(1);
    });
    eprintln!("memory-daemon listening on {bind_addr}");
    let mut predictive_monitor = start_predictive_defrag_monitor(args);

    for stream in listener.incoming() {
        let Ok(mut stream) = stream else {
            continue;
        };
        let request_started = std::time::Instant::now();

        let mut line = String::new();
        {
            let mut reader = BufReader::new(&mut stream);
            if reader.read_line(&mut line).is_err() {
                daemon_write_response(
                    &mut stream,
                    &daemon_error_response(
                        "unknown",
                        "read_line",
                        "invalid_request",
                        request_started,
                    ),
                );
                continue;
            }
        }

        let parsed = serde_json::from_str::<DaemonRequest>(line.trim());
        let req = match parsed {
            Ok(v) => v,
            Err(_) => {
                daemon_write_response(
                    &mut stream,
                    &daemon_error_response(
                        "unknown",
                        "parse_json",
                        "invalid_json",
                        request_started,
                    ),
                );
                continue;
            }
        };

        let cmd = normalize_daemon_command(&req.cmd);
        let req_args = req.args;

        let (response, should_shutdown) = match cmd.as_str() {
            "ping" => (
                json!({
                    "ok": true,
                    "type": "memory_daemon_pong",
                    "backend": "protheus_memory_core"
                }),
                false,
            ),
            "probe" => {
                let root = PathBuf::from(arg_or_default(
                    &req_args,
                    "root",
                    detect_default_root().to_string_lossy().as_ref(),
                ));
                let started = Instant::now();
                let (_source, _entries) = load_memory_index(&root);
                let elapsed_ms = started.elapsed().as_millis() as u64;
                (
                    json!({
                        "ok": true,
                        "parity_error_count": 0,
                        "estimated_ms": elapsed_ms.max(1)
                    }),
                    false,
                )
            }
            "predictive-defrag-status" => (predictive_monitor.status_payload(), false),
            "predictive-defrag-stress" => (predictive_defrag_stress_payload(&req_args), false),
            "query-index" => (
                serde_json::to_value(query_index_payload(&req_args))
                    .unwrap_or_else(|_| json!({"ok": false, "error": "query_serialize_failed"})),
                false,
            ),
            "get-node" => {
                let (payload, _code) = get_node_payload(&req_args);
                (payload, false)
            }
            "build-index" => (
                serde_json::to_value(build_index_payload(&req_args))
                    .unwrap_or_else(|_| json!({"ok": false, "error": "build_serialize_failed"})),
                false,
            ),
            "verify-envelope" => (
                serde_json::to_value(verify_envelope_payload(&req_args)).unwrap_or_else(
                    |_| json!({"ok": false, "error": "verify_envelope_serialize_failed"}),
                ),
                false,
            ),
            "set-hot-state" => (set_hot_state_payload(&req_args), false),
            "get-hot-state" => (get_hot_state_payload(&req_args), false),
            "memory-matrix" => (wave1::memory_matrix_payload(&req_args), false),
            "memory-auto-recall" => (wave1::memory_auto_recall_payload(&req_args), false),
            "dream-sequencer" => (wave1::dream_sequencer_payload(&req_args), false),
            "rag-ingest" => (rag_runtime::ingest_payload(&req_args), false),
            "rag-search" => (rag_runtime::search_payload(&req_args), false),
            "rag-chat" => (rag_runtime::chat_payload(&req_args), false),
            "nano-chat" => (rag_runtime::nano_chat_payload(&req_args), false),
            "nano-train" => (rag_runtime::nano_train_payload(&req_args), false),
            "nano-fork" => (rag_runtime::nano_fork_payload(&req_args), false),
            "rag-status" => (rag_runtime::status_payload(&req_args), false),
            "rag-merge-vault" => (rag_runtime::merge_vault_payload(&req_args), false),
            "memory-upgrade-byterover" => {
                (rag_runtime::byterover_upgrade_payload(&req_args), false)
            }
            "memory-taxonomy" => (rag_runtime::memory_taxonomy_payload(&req_args), false),
            "memory-enable-metacognitive" => (
                rag_runtime::memory_metacognitive_enable_payload(&req_args),
                false,
            ),
            "memory-enable-causality" => (
                rag_runtime::memory_causality_enable_payload(&req_args),
                false,
            ),
            "memory-benchmark-ama" => (rag_runtime::memory_benchmark_ama_payload(&req_args), false),
            "memory-share" => (rag_runtime::memory_share_payload(&req_args), false),
            "memory-evolve" => (rag_runtime::memory_evolve_payload(&req_args), false),
            "memory-causal-retrieve" => (
                rag_runtime::memory_causal_retrieve_payload(&req_args),
                false,
            ),
            "memory-fuse" => (rag_runtime::memory_fuse_payload(&req_args), false),
            "stable-status" => (rag_runtime::stable_status_payload(), false),
            "stable-search" => match rag_runtime::ensure_supported_version(&req_args) {
                Ok(version) => {
                    let mut payload = serde_json::to_value(query_index_payload(&req_args))
                        .unwrap_or_else(
                            |_| json!({"ok": false, "error": "query_serialize_failed"}),
                        );
                    payload["api_version"] = json!(version);
                    (payload, false)
                }
                Err(err) => (err, false),
            },
            "stable-get-node" => match rag_runtime::ensure_supported_version(&req_args) {
                Ok(version) => {
                    let (mut payload, _code) = get_node_payload(&req_args);
                    payload["api_version"] = json!(version);
                    (payload, false)
                }
                Err(err) => (err, false),
            },
            "stable-build-index" => match rag_runtime::ensure_supported_version(&req_args) {
                Ok(version) => {
                    let mut payload = serde_json::to_value(build_index_payload(&req_args))
                        .unwrap_or_else(
                            |_| json!({"ok": false, "error": "build_serialize_failed"}),
                        );
                    payload["api_version"] = json!(version);
                    (payload, false)
                }
                Err(err) => (err, false),
            },
            "stable-rag-ingest" => match rag_runtime::ensure_supported_version(&req_args) {
                Ok(version) => {
                    let mut payload = rag_runtime::ingest_payload(&req_args);
                    payload["api_version"] = json!(version);
                    (payload, false)
                }
                Err(err) => (err, false),
            },
            "stable-rag-search" => match rag_runtime::ensure_supported_version(&req_args) {
                Ok(version) => {
                    let mut payload = rag_runtime::search_payload(&req_args);
                    payload["api_version"] = json!(version);
                    (payload, false)
                }
                Err(err) => (err, false),
            },
            "stable-rag-chat" => match rag_runtime::ensure_supported_version(&req_args) {
                Ok(version) => {
                    let mut payload = rag_runtime::chat_payload(&req_args);
                    payload["api_version"] = json!(version);
                    (payload, false)
                }
                Err(err) => (err, false),
            },
            "stable-nano-chat" => match rag_runtime::ensure_supported_version(&req_args) {
                Ok(version) => {
                    let mut payload = rag_runtime::nano_chat_payload(&req_args);
                    payload["api_version"] = json!(version);
                    (payload, false)
                }
                Err(err) => (err, false),
            },
            "stable-nano-train" => match rag_runtime::ensure_supported_version(&req_args) {
                Ok(version) => {
                    let mut payload = rag_runtime::nano_train_payload(&req_args);
                    payload["api_version"] = json!(version);
                    (payload, false)
                }
                Err(err) => (err, false),
            },
            "stable-nano-fork" => match rag_runtime::ensure_supported_version(&req_args) {
                Ok(version) => {
                    let mut payload = rag_runtime::nano_fork_payload(&req_args);
                    payload["api_version"] = json!(version);
                    (payload, false)
                }
                Err(err) => (err, false),
            },
            "stable-memory-upgrade-byterover" => {
                match rag_runtime::ensure_supported_version(&req_args) {
                    Ok(version) => {
                        let mut payload = rag_runtime::byterover_upgrade_payload(&req_args);
                        payload["api_version"] = json!(version);
                        (payload, false)
                    }
                    Err(err) => (err, false),
                }
            }
            "stable-memory-taxonomy" => match rag_runtime::ensure_supported_version(&req_args) {
                Ok(version) => {
                    let mut payload = rag_runtime::memory_taxonomy_payload(&req_args);
                    payload["api_version"] = json!(version);
                    (payload, false)
                }
                Err(err) => (err, false),
            },
            "stable-memory-enable-metacognitive" => {
                match rag_runtime::ensure_supported_version(&req_args) {
                    Ok(version) => {
                        let mut payload =
                            rag_runtime::memory_metacognitive_enable_payload(&req_args);
                        payload["api_version"] = json!(version);
                        (payload, false)
                    }
                    Err(err) => (err, false),
                }
            }
            "stable-memory-enable-causality" => {
                match rag_runtime::ensure_supported_version(&req_args) {
                    Ok(version) => {
                        let mut payload = rag_runtime::memory_causality_enable_payload(&req_args);
                        payload["api_version"] = json!(version);
                        (payload, false)
                    }
                    Err(err) => (err, false),
                }
            }
            "stable-memory-benchmark-ama" => {
                match rag_runtime::ensure_supported_version(&req_args) {
                    Ok(version) => {
                        let mut payload = rag_runtime::memory_benchmark_ama_payload(&req_args);
                        payload["api_version"] = json!(version);
                        (payload, false)
                    }
                    Err(err) => (err, false),
                }
            }
            "stable-memory-share" => match rag_runtime::ensure_supported_version(&req_args) {
                Ok(version) => {
                    let mut payload = rag_runtime::memory_share_payload(&req_args);
                    payload["api_version"] = json!(version);
                    (payload, false)
                }
                Err(err) => (err, false),
            },
            "stable-memory-evolve" => match rag_runtime::ensure_supported_version(&req_args) {
                Ok(version) => {
                    let mut payload = rag_runtime::memory_evolve_payload(&req_args);
                    payload["api_version"] = json!(version);
                    (payload, false)
                }
                Err(err) => (err, false),
            },
            "stable-memory-causal-retrieve" => {
                match rag_runtime::ensure_supported_version(&req_args) {
                    Ok(version) => {
                        let mut payload = rag_runtime::memory_causal_retrieve_payload(&req_args);
                        payload["api_version"] = json!(version);
                        (payload, false)
                    }
                    Err(err) => (err, false),
                }
            }
            "stable-memory-fuse" => match rag_runtime::ensure_supported_version(&req_args) {
                Ok(version) => {
                    let mut payload = rag_runtime::memory_fuse_payload(&req_args);
                    payload["api_version"] = json!(version);
                    (payload, false)
                }
                Err(err) => (err, false),
            },
            "shutdown" => (
                json!({
                    "ok": true,
                    "type": "memory_daemon_shutdown"
                }),
                true,
            ),
            _ => (
                json!({
                    "ok": false,
                    "type": "memory_daemon_error",
                    "error": "unsupported_command",
                    "cmd": cmd,
                    "supported_core_commands": ["ping", "probe", "stable-status", "shutdown"]
                }),
                false,
            ),
        };

        let response = finalize_daemon_response(response, &cmd, request_started);
        daemon_write_response(&mut stream, &response);
        if should_shutdown {
            break;
        }
    }
    predictive_monitor.shutdown();
}
