fn validate_swarm_profile(
    profile: &RuntimeCapabilityProfile,
    argv: &[String],
) -> Result<(), String> {
    let sub = argv
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    match sub.as_str() {
        "spawn" => {
            if !profile.allow_swarm_spawn {
                return Err("hardware_profile_blocks_swarm_spawn".to_string());
            }
            let requested_depth = parse_u8_flag(argv, "max-depth", profile.max_swarm_depth);
            if requested_depth > profile.max_swarm_depth {
                return Err(format!(
                    "hardware_profile_max_swarm_depth_exceeded:{}>{}",
                    requested_depth, profile.max_swarm_depth
                ));
            }
            let execution_mode = parse_flag(argv, "execution-mode")
                .unwrap_or_else(|| "task".to_string())
                .trim()
                .to_ascii_lowercase();
            if matches!(execution_mode.as_str(), "persistent" | "background")
                && !profile.allow_persistent_swarm
            {
                return Err("hardware_profile_blocks_persistent_swarm".to_string());
            }
            Ok(())
        }
        "background" => {
            if !profile.allow_persistent_swarm {
                Err("hardware_profile_blocks_background_swarm".to_string())
            } else {
                Ok(())
            }
        }
        "test" => {
            let suite = argv
                .get(1)
                .map(|v| v.trim().to_ascii_lowercase())
                .unwrap_or_else(|| "recursive".to_string());
            if suite == "persistent" && !profile.allow_persistent_swarm {
                Err("hardware_profile_blocks_persistent_swarm_test".to_string())
            } else {
                Ok(())
            }
        }
        _ => Ok(()),
    }
}

fn memory_store_path(root: &Path) -> PathBuf {
    client_state_root(root)
        .join("memory")
        .join("pure_workspace_memory_v1.jsonl")
}

fn read_memory_entries(path: &Path) -> Vec<Value> {
    if !path.exists() {
        return Vec::new();
    }
    let file = match std::fs::File::open(path) {
        Ok(v) => v,
        Err(_) => return Vec::new(),
    };
    let mut out = Vec::new();
    for line in BufReader::new(file).lines().map_while(Result::ok) {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
            out.push(value);
        }
    }
    out
}

fn append_memory_entry(path: &Path, row: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|err| format!("create_memory_parent_failed:{err}"))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|err| format!("open_memory_store_failed:{err}"))?;
    let line =
        serde_json::to_string(row).map_err(|err| format!("encode_memory_row_failed:{err}"))?;
    file.write_all(line.as_bytes())
        .map_err(|err| format!("write_memory_row_failed:{err}"))?;
    file.write_all(b"\n")
        .map_err(|err| format!("write_memory_newline_failed:{err}"))?;
    Ok(())
}

fn memory_status_payload(root: &Path) -> Value {
    let path = memory_store_path(root);
    let entries = read_memory_entries(&path);
    let bytes = std::fs::metadata(&path).map(|m| m.len()).unwrap_or(0);
    let last_ts = entries
        .last()
        .and_then(|v| v.get("ts"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let mut out = json!({
        "ok": true,
        "type": "pure_memory_status",
        "ts": now_iso(),
        "path": path.to_string_lossy(),
        "entry_count": entries.len(),
        "bytes": bytes,
        "last_ts": last_ts
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn memory_write_payload(root: &Path, argv: &[String]) -> Result<Value, String> {
    let text = clean_text(parse_flag(argv, "text").as_deref(), 4000);
    if text.is_empty() {
        return Err("missing_text".to_string());
    }
    let session_id = clean_token(parse_flag(argv, "session-id").as_deref(), "default");
    let tags = parse_flag(argv, "tags")
        .map(|raw| {
            raw.split(',')
                .map(|v| clean_token(Some(v), ""))
                .filter(|v| !v.is_empty())
                .take(16)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let ts = now_iso();
    let id_seed = json!({
        "text": text,
        "session_id": session_id,
        "ts": ts
    });
    let derived_id = crate::deterministic_receipt_hash(&id_seed)
        .chars()
        .take(16)
        .collect::<String>();
    let item_id = clean_token(parse_flag(argv, "id").as_deref(), derived_id.as_str());
    let row = json!({
        "id": item_id,
        "ts": ts,
        "session_id": session_id,
        "text": text,
        "tags": tags
    });
    let path = memory_store_path(root);
    append_memory_entry(&path, &row)?;
    let mut out = json!({
        "ok": true,
        "type": "pure_memory_write",
        "ts": now_iso(),
        "path": path.to_string_lossy(),
        "item": row
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    Ok(out)
}

fn memory_query_payload(root: &Path, argv: &[String]) -> Value {
    let q = clean_text(
        parse_flag(argv, "q")
            .or_else(|| parse_flag(argv, "text"))
            .as_deref(),
        240,
    )
    .to_ascii_lowercase();
    let session = clean_token(parse_flag(argv, "session-id").as_deref(), "");
    let tag = clean_token(parse_flag(argv, "tag").as_deref(), "").to_ascii_lowercase();
    let limit = parse_usize(parse_flag(argv, "limit").as_deref(), 20, 1, 200);
    let mut entries = read_memory_entries(&memory_store_path(root))
        .into_iter()
        .filter(|row| {
            let text = row
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_ascii_lowercase();
            let row_session = row
                .get("session_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let tag_match = if tag.is_empty() {
                true
            } else {
                row.get("tags")
                    .and_then(Value::as_array)
                    .map(|tags| {
                        tags.iter().any(|v| {
                            v.as_str()
                                .map(|s| s.to_ascii_lowercase() == tag)
                                .unwrap_or(false)
                        })
                    })
                    .unwrap_or(false)
            };
            let session_match = session.is_empty() || row_session == session;
            let text_match = q.is_empty() || text.contains(&q);
            session_match && tag_match && text_match
        })
        .collect::<Vec<_>>();
    entries.sort_by(|a, b| {
        b.get("ts")
            .and_then(Value::as_str)
            .unwrap_or("")
            .cmp(a.get("ts").and_then(Value::as_str).unwrap_or(""))
    });
    entries.truncate(limit);
    let mut out = json!({
        "ok": true,
        "type": "pure_memory_query",
        "ts": now_iso(),
        "q": q,
        "session_id": if session.is_empty() { Value::Null } else { Value::String(session.clone()) },
        "tag": if tag.is_empty() { Value::Null } else { Value::String(tag.clone()) },
        "limit": limit,
        "matches": entries
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn contains_any_token(haystack: &str, tokens: &[String]) -> usize {
    let hay = haystack.to_ascii_lowercase();
    tokens
        .iter()
        .filter(|token| hay.contains(token.as_str()))
        .count()
}

fn think_payload(root: &Path, argv: &[String]) -> Result<Value, String> {
    let prompt = clean_text(parse_flag(argv, "prompt").as_deref(), 1200);
    if prompt.is_empty() {
        return Err("missing_prompt".to_string());
    }
    let session_id = clean_token(parse_flag(argv, "session-id").as_deref(), "default");
    let profile = runtime_capability_profile(argv);
    let requested_memory_limit = parse_usize(parse_flag(argv, "memory-limit").as_deref(), 5, 1, 20);
    let memory_limit = requested_memory_limit.min(profile.max_memory_hits);
    let lower_prompt = prompt.to_ascii_lowercase();
    let tokens = lower_prompt
        .split(|ch: char| !ch.is_ascii_alphanumeric())
        .filter(|token| token.len() >= 3)
        .take(12)
        .map(|token| token.to_string())
        .collect::<Vec<_>>();
    let mut scored = read_memory_entries(&memory_store_path(root))
        .into_iter()
        .filter_map(|entry| {
            let row_session = entry
                .get("session_id")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            if row_session != session_id {
                return None;
            }
            let text = entry
                .get("text")
                .and_then(Value::as_str)
                .unwrap_or("")
                .to_string();
            let score = contains_any_token(&text, &tokens);
            if score == 0 {
                return None;
            }
            Some((score, entry))
        })
        .collect::<Vec<_>>();
    scored.sort_by(|a, b| b.0.cmp(&a.0));
    let memory_hits = scored
        .into_iter()
        .take(memory_limit)
        .map(|(_, row)| row)
        .collect::<Vec<_>>();

    let hint = if lower_prompt.contains("http://") || lower_prompt.contains("https://") {
        "Detected URL intent: run `infring research fetch --url=<url>` for source capture."
    } else if lower_prompt.contains("research") {
        "Research intent detected: run `infring research status` then `infring research fetch --url=<url>`."
    } else {
        "Action intent detected: break the task into one immediate execution step and one verification step."
    };
    let response = format!(
        "Prompt focus: {}. {}",
        prompt.chars().take(180).collect::<String>(),
        hint
    );
    let mut out = json!({
        "ok": true,
        "type": "pure_think",
        "ts": now_iso(),
        "session_id": session_id,
        "prompt": prompt,
        "requested_memory_limit": requested_memory_limit,
        "effective_memory_limit": memory_limit,
        "capability_profile": profile.as_json(),
        "memory_hits": memory_hits,
        "response": response,
        "next_actions": [
            "define_success_criteria",
            "execute_smallest_safe_step",
            "record_outcome_in_memory"
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    Ok(out)
}

fn run_research(root: &Path, argv: &[String]) -> i32 {
    let args = profiled_run_args(argv, "status");
    let command = args
        .rest
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if !matches!(command.as_str(), "status" | "fetch" | "diagnostics") {
        print_json(&cli_error(
            "research_command_not_allowed_in_pure_v1",
            "research",
        ));
        return 1;
    }
    if command == "fetch" && !args.profile.allow_research_fetch {
        print_json(&cli_error(
            "hardware_profile_blocks_research_fetch",
            "research",
        ));
        return 1;
    }
    infring_ops_core::research_plane::run(root, &args.rest)
}

fn run_memory(root: &Path, argv: &[String]) -> i32 {
    let command = argv
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    match command.as_str() {
        "status" => {
            print_json(&memory_status_payload(root));
            0
        }
        "write" => match memory_write_payload(root, &argv[1..]) {
            Ok(payload) => {
                print_json(&payload);
                0
            }
            Err(err) => {
                print_json(&cli_error(err.as_str(), "memory"));
                1
            }
        },
        "query" => {
            print_json(&memory_query_payload(root, &argv[1..]));
            0
        }
        _ => {
            print_json(&cli_error("unknown_memory_command", "memory"));
            1
        }
    }
}

fn run_think(root: &Path, argv: &[String]) -> i32 {
    match think_payload(root, argv) {
        Ok(payload) => {
            print_json(&payload);
            0
        }
        Err(err) => {
            print_json(&cli_error(err.as_str(), "think"));
            1
        }
    }
}

fn run_orchestration(root: &Path, argv: &[String]) -> i32 {
    let args = profiled_run_args(argv, "help");
    if enforce_profile(
        &args.profile,
        &args.rest,
        "orchestration",
        validate_orchestration_profile,
    )
    .is_err()
    {
        return 1;
    }
    infring_ops_core::orchestration::run(root, &args.rest)
}

fn run_swarm(root: &Path, argv: &[String]) -> i32 {
    let args = profiled_run_args(argv, "status");
    if enforce_profile(
        &args.profile,
        &args.rest,
        "swarm-runtime",
        validate_swarm_profile,
    )
    .is_err()
    {
        return 1;
    }
    infring_ops_core::swarm_runtime::run(root, &args.rest)
}

#[cfg(feature = "embedded-minimal-core")]
fn embedded_minimal_core_planes() -> [(&'static str, &'static str, PlaneRunner); 5] {
    [
        (
            "layer0-directives",
            "directive_kernel",
            infring_ops_core::directive_kernel::run,
        ),
        (
            "layer0-attention",
            "attention_queue",
            infring_ops_core::attention_queue::run,
        ),
        (
            "layer0-receipts",
            "metakernel",
            infring_ops_core::metakernel::run,
        ),
        (
            "layer0-min-memory",
            "memory_plane",
            infring_ops_core::memory_plane::run,
        ),
        (
            "layer-1-substrate-detector",
            "substrate_plane",
            infring_ops_core::substrate_plane::run,
        ),
    ]
}

