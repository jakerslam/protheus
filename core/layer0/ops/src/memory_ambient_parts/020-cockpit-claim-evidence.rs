fn cockpit_claim_evidence(
    memory_command: &str,
    memory_args: &[String],
    telemetry: &Value,
) -> Vec<Value> {
    if !is_nano_memory_command(memory_command) {
        return Vec::new();
    }

    let mut claims = Vec::new();
    match memory_command {
        "stable-nano-chat" => claims.push(json!({
            "id": "V6-COCKPIT-026.1",
            "claim": "chat_nano_routes_through_rust_core_memory_runtime_with_deterministic_receipts",
            "evidence": {
                "memory_command": memory_command
            }
        })),
        "stable-nano-train" => claims.push(json!({
            "id": "V6-COCKPIT-026.2",
            "claim": "train_nano_depth_harness_routes_through_stable_rust_memory_path",
            "evidence": {
                "memory_command": memory_command,
                "depth": parse_arg_value(memory_args, "depth").unwrap_or_else(|| "unknown".to_string())
            }
        })),
        "stable-nano-fork" => claims.push(json!({
            "id": "V6-COCKPIT-026.3",
            "claim": "nano_fork_emits_deterministic_fork_artifact_path_contract_receipts",
            "evidence": {
                "memory_command": memory_command,
                "target": parse_arg_value(memory_args, "target").unwrap_or_else(|| ".nanochat/fork".to_string())
            }
        })),
        _ => {}
    }

    claims.push(json!({
        "id": "V6-COCKPIT-026.4",
        "claim": "all_nano_commands_route_through_rust_core_memory_runtime_with_fail_closed_conduit_boundary",
        "evidence": {
            "memory_command": memory_command,
            "memory_args_count": memory_args.len()
        }
    }));

    claims.push(json!({
        "id": "V6-COCKPIT-026.5",
        "claim": "nano_mode_receipts_include_live_telemetry_for_dashboard_observability",
        "evidence": {
            "memory_command": memory_command,
            "tokens_total": telemetry
                .get("tokens")
                .and_then(|v| v.get("total"))
                .and_then(Value::as_i64)
                .unwrap_or(0),
            "retrieval_mode": telemetry
                .get("retrieval_mode")
                .and_then(Value::as_str)
                .unwrap_or("index_only")
        }
    }));

    claims
}

fn classify_retrieval_mode(memory_command: &str, memory_args: &[String]) -> String {
    if memory_command == "get-node" {
        return "node_read".to_string();
    }
    if memory_command != "query-index" {
        return "index_only".to_string();
    }
    let expand_lines = parse_arg_value(memory_args, "expand-lines")
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0);
    if expand_lines >= 120 {
        return "full_file".to_string();
    }
    if expand_lines > 0 {
        return "node_read".to_string();
    }
    "index_only".to_string()
}

fn token_threshold() -> i64 {
    std::env::var("MEMORY_RECALL_TOKEN_BURN_THRESHOLD")
        .ok()
        .and_then(|raw| raw.trim().parse::<i64>().ok())
        .filter(|value| *value >= 50)
        .unwrap_or(200)
}

fn telemetry_reasons(
    retrieval_mode: &str,
    total_tokens: i64,
    threshold_tokens: i64,
    memory_args: &[String],
) -> Vec<String> {
    let mut out = Vec::new();
    match retrieval_mode {
        "full_file" => out.push("full_file_mode".to_string()),
        "node_read" => out.push("node_expansion_path".to_string()),
        _ => out.push("index_first_path".to_string()),
    }

    let expand_lines = parse_arg_value(memory_args, "expand-lines")
        .and_then(|value| value.parse::<i64>().ok())
        .unwrap_or(0);
    if expand_lines > 40 {
        out.push("large_expand_lines".to_string());
    }
    if total_tokens > threshold_tokens {
        out.push("high_burn_threshold_exceeded".to_string());
    }
    out
}

fn build_token_telemetry(memory_command: &str, memory_args: &[String], payload: &Value) -> Value {
    let command = command_label(memory_command);
    let retrieval_mode = classify_retrieval_mode(memory_command, memory_args);
    let threshold_tokens = token_threshold();
    let retrieval_input = json!({
        "cmd": command,
        "q": parse_arg_value(memory_args, "q").unwrap_or_default(),
        "tags": parse_arg_value(memory_args, "tags").unwrap_or_default(),
        "top": parse_arg_value(memory_args, "top").unwrap_or_default(),
        "expand_lines": parse_arg_value(memory_args, "expand-lines").unwrap_or_default(),
        "node_id": parse_arg_value(memory_args, "node-id").unwrap_or_default(),
        "uid": parse_arg_value(memory_args, "uid").unwrap_or_default(),
        "args_count": memory_args.len()
    });
    let hydration_tokens = 0_i64;
    let retrieval_tokens = estimate_tokens(&retrieval_input);
    let response_tokens = estimate_tokens(payload);
    let total_tokens = hydration_tokens + retrieval_tokens + response_tokens;
    let reason_codes =
        telemetry_reasons(&retrieval_mode, total_tokens, threshold_tokens, memory_args);

    json!({
        "version": "1.0",
        "command": command,
        "retrieval_mode": retrieval_mode,
        "threshold_tokens": threshold_tokens,
        "tokens": {
            "hydration": hydration_tokens,
            "retrieval": retrieval_tokens,
            "response": response_tokens,
            "total": total_tokens
        },
        "reason_codes": reason_codes
    })
}

fn token_telemetry_path(root: &Path) -> PathBuf {
    if let Ok(custom) = std::env::var("MEMORY_RECALL_TOKEN_TELEMETRY_PATH") {
        let trimmed = custom.trim();
        if !trimmed.is_empty() {
            let candidate = PathBuf::from(trimmed);
            if candidate.is_absolute() {
                return candidate;
            }
            return root.join(candidate);
        }
    }
    root.join("client")
        .join("runtime")
        .join("local")
        .join("state")
        .join("memory")
        .join("query_token_metrics.jsonl")
}

fn normalize_path(root: &Path, value: Option<&Value>, fallback: &str) -> PathBuf {
    let raw = value
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or(fallback);
    let candidate = PathBuf::from(raw);
    if candidate.is_absolute() {
        candidate
    } else {
        root.join(candidate)
    }
}

fn parse_json_payload(stdout: &str) -> Option<Value> {
    let raw = stdout.trim();
    if raw.is_empty() {
        return None;
    }
    if let Ok(payload) = serde_json::from_str::<Value>(raw) {
        return Some(payload);
    }
    for line in raw.lines().rev() {
        let trimmed = line.trim();
        if !trimmed.starts_with('{') {
            continue;
        }
        if let Ok(payload) = serde_json::from_str::<Value>(trimmed) {
            return Some(payload);
        }
    }
    None
}

fn load_policy(root: &Path) -> MemoryAmbientPolicy {
    let default_policy = root.join("config").join("mech_suit_mode_policy.json");
    let policy_path = std::env::var("MECH_SUIT_MODE_POLICY_PATH")
        .ok()
        .map(PathBuf::from)
        .map(|p| if p.is_absolute() { p } else { root.join(p) })
        .unwrap_or(default_policy);
    let policy = read_json(&policy_path).unwrap_or_else(|| json!({}));
    let enabled = bool_from_env("MECH_SUIT_MODE_FORCE").unwrap_or_else(|| {
        policy
            .get("enabled")
            .and_then(Value::as_bool)
            .unwrap_or(true)
    });
    let eyes = policy.get("eyes");
    let receipts = policy.get("receipts");
    let memory = policy.get("memory");
    let state = policy.get("state");

    let surface_levels = parse_string_array(memory.and_then(|v| v.get("surface_levels")), 6, 24)
        .into_iter()
        .map(|row| row.to_ascii_lowercase())
        .filter(|row| matches!(row.as_str(), "critical" | "warn" | "info"))
        .collect::<Vec<_>>();

    MemoryAmbientPolicy {
        enabled,
        rust_authoritative: memory
            .and_then(|v| v.get("rust_authoritative"))
            .and_then(Value::as_bool)
            .unwrap_or(true),
        push_attention_queue: memory
            .and_then(|v| v.get("push_attention_queue"))
            .and_then(Value::as_bool)
            .or_else(|| {
                eyes.and_then(|v| v.get("push_attention_queue"))
                    .and_then(Value::as_bool)
            })
            .unwrap_or(true),
        quiet_non_critical: memory
            .and_then(|v| v.get("quiet_non_critical"))
            .and_then(Value::as_bool)
            .or_else(|| {
                receipts
                    .and_then(|v| v.get("silent_unless_critical"))
                    .and_then(Value::as_bool)
            })
            .unwrap_or(true),
        surface_levels: if surface_levels.is_empty() {
            vec!["warn".to_string(), "critical".to_string()]
        } else {
            surface_levels
        },
        latest_path: normalize_path(
            root,
            memory.and_then(|v| v.get("latest_path")),
            "local/state/client/memory/ambient/latest.json",
        ),
        receipts_path: normalize_path(
            root,
            memory.and_then(|v| v.get("receipts_path")),
            "local/state/client/memory/ambient/receipts.jsonl",
        ),
        status_path: normalize_path(
            root,
            state.and_then(|v| v.get("status_path")),
            "local/state/ops/mech_suit_mode/latest.json",
        ),
        history_path: normalize_path(
            root,
            state.and_then(|v| v.get("history_path")),
            "local/state/ops/mech_suit_mode/history.jsonl",
        ),
        policy_path,
    }
}

fn resolve_memory_command(root: &PathBuf) -> (String, Vec<String>) {
    let explicit = std::env::var("PROTHEUS_MEMORY_CORE_BIN").ok();
    if let Some(bin) = explicit {
        let trimmed = bin.trim();
        if !trimmed.is_empty() {
            return (trimmed.to_string(), Vec::new());
        }
    }

    // Prefer the authoritative layer0/memory_runtime binary. Older paths may still
    // leave a `memory-cli` executable from legacy crates, so check both names.
    let release_primary = root
        .join("target")
        .join("release")
        .join("protheus-memory-core");
    if release_primary.exists() {
        return (release_primary.to_string_lossy().to_string(), Vec::new());
    }
    let debug_primary = root
        .join("target")
        .join("debug")
        .join("protheus-memory-core");
    if debug_primary.exists() {
        return (debug_primary.to_string_lossy().to_string(), Vec::new());
    }

    // If runtime roots point at tenant/temp paths, still resolve the compiled
    // authoritative binary from the workspace target directory.
    let workspace_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(3)
        .map(Path::to_path_buf)
        .unwrap_or_else(|| PathBuf::from("."));
    let ws_release_primary = workspace_root
        .join("target")
        .join("release")
        .join("protheus-memory-core");
    if ws_release_primary.exists() {
        return (ws_release_primary.to_string_lossy().to_string(), Vec::new());
    }
    let ws_debug_primary = workspace_root
        .join("target")
        .join("debug")
        .join("protheus-memory-core");
    if ws_debug_primary.exists() {
        return (ws_debug_primary.to_string_lossy().to_string(), Vec::new());
    }

    let release_compat = root.join("target").join("release").join("memory-cli");
    if release_compat.exists() {
        return (release_compat.to_string_lossy().to_string(), Vec::new());
    }
    let debug_compat = root.join("target").join("debug").join("memory-cli");
    if debug_compat.exists() {
        return (debug_compat.to_string_lossy().to_string(), Vec::new());
    }

    let manifest_path = workspace_root.join("core/layer0/memory_runtime/Cargo.toml");
    (
        "cargo".to_string(),
        vec![
            "run".to_string(),
            "--manifest-path".to_string(),
            manifest_path.to_string_lossy().to_string(),
            "--bin".to_string(),
            "protheus-memory-core".to_string(),
            "--".to_string(),
        ],
    )
}

fn resolve_protheus_ops_command(root: &PathBuf, domain: &str) -> (String, Vec<String>) {
    crate::contract_lane_utils::resolve_protheus_ops_command(root.as_path(), domain)
}

fn is_allowed_memory_command(command: &str) -> bool {
    matches!(
        command,
        "recall"
            | "query-index"
            | "probe"
            | "build-index"
            | "verify-envelope"
            | "compress"
            | "set-hot-state"
            | "ingest"
            | "get"
            | "clear-cache"
            | "ebbinghaus-score"
            | "crdt-exchange"
            | "load-embedded-heartbeat"
            | "load-embedded-execution-replay"
            | "load-embedded-vault-policy"
            | "load-embedded-observability-profile"
            | "pack-memory-blobs"
            | "pack-heartbeat-blob"
            | "cryonics-tier"
            | "memory-matrix"
            | "memory-auto-recall"
            | "dream-sequencer"
            | "rag-ingest"
            | "rag-search"
            | "rag-chat"
            | "rag-status"
            | "rag-merge-vault"
            | "memory-upgrade-byterover"
            | "memory-taxonomy"
            | "memory-enable-metacognitive"
            | "memory-enable-causality"
            | "memory-benchmark-ama"
            | "memory-share"
            | "memory-evolve"
            | "memory-causal-retrieve"
            | "memory-fuse"
            | "stable-status"
            | "stable-search"
            | "stable-get-node"
            | "stable-build-index"
            | "stable-rag-ingest"
            | "stable-rag-search"
            | "stable-rag-chat"
            | "stable-nano-chat"
            | "stable-nano-train"
            | "stable-nano-fork"
            | "stable-memory-upgrade-byterover"
            | "stable-memory-taxonomy"
            | "stable-memory-enable-metacognitive"
            | "stable-memory-enable-causality"
            | "stable-memory-benchmark-ama"
            | "stable-memory-share"
            | "stable-memory-evolve"
            | "stable-memory-causal-retrieve"
            | "stable-memory-fuse"
            | "help"
    )
}

