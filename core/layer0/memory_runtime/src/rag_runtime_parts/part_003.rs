pub fn memory_share_payload(args: &HashMap<String, String>) -> Value {
    let root = root_from_args(args);
    let persona = clean_text(args.get("persona").map_or("peer", String::as_str), 120);
    let scope = clean_text(args.get("scope").map_or("task", String::as_str), 40);
    let consent = parse_bool(args.get("consent"), false);
    let reason = clean_text(args.get("reason").map_or("", String::as_str), 240);
    let record = json!({
        "ts": now_iso(),
        "persona": persona,
        "scope": scope,
        "consent": consent,
        "reason": reason
    });
    let consent_scope_digest = sha256_hex(
        serde_json::to_string(&json!({
            "persona": record.get("persona").cloned().unwrap_or(Value::Null),
            "scope": record.get("scope").cloned().unwrap_or(Value::Null),
            "consent": record.get("consent").cloned().unwrap_or(Value::Null),
            "reason": record.get("reason").cloned().unwrap_or(Value::Null)
        }))
        .unwrap_or_default()
        .as_bytes(),
    );
    let path = sharing_ledger_path(&root, args);
    append_history(&path, &record);
    let out = receipt(json!({
        "ok": consent,
        "type": "memory_share",
        "backend": "protheus_memory_core",
        "persona": persona,
        "scope": scope,
        "consent": consent,
        "consent_scope_digest": consent_scope_digest,
        "sharing_ledger_path": normalize_rel_path(&root, &path),
        "error": if consent { Value::Null } else { Value::String("consent_required".to_string()) }
    }));
    append_history(&history_path(&root, args), &out);
    out
}

pub fn memory_evolve_payload(args: &HashMap<String, String>) -> Value {
    let root = root_from_args(args);
    let h_rows = load_history_rows(&history_path(&root, args));
    let meta_path = metacognitive_journal_path(&root, args);
    let meta_rows = load_history_rows(&meta_path);
    let share_rows = load_history_rows(&sharing_ledger_path(&root, args));
    let generation = parse_usize(args.get("generation"), 1, 100_000, 1);
    let stability_score = ((h_rows.len() as f64 * 0.4
        + meta_rows.len() as f64 * 0.3
        + share_rows.len() as f64 * 0.3)
        .sqrt()
        / 10.0)
        .clamp(0.0, 1.0);
    let snapshot = json!({
        "schema_version": "1.0",
        "generated_at": now_iso(),
        "generation": generation,
        "history_events": h_rows.len(),
        "metacognitive_events": meta_rows.len(),
        "sharing_events": share_rows.len(),
        "stability_score": ((stability_score * 1000.0).round() / 1000.0)
    });
    let evolution_digest = sha256_hex(
        serde_json::to_string(&json!({
            "generation": generation,
            "history_events": h_rows.len(),
            "metacognitive_events": meta_rows.len(),
            "sharing_events": share_rows.len(),
            "stability_score": snapshot.get("stability_score").cloned().unwrap_or(Value::Null)
        }))
        .unwrap_or_default()
        .as_bytes(),
    );
    let out_path = evolution_state_path(&root, args);
    if let Some(parent) = out_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(
        &out_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string())
        ),
    );
    let out = receipt(json!({
        "ok": true,
        "type": "memory_evolve",
        "backend": "protheus_memory_core",
        "generation": generation,
        "stability_score": snapshot.get("stability_score").cloned().unwrap_or(Value::Null),
        "evolution_digest": evolution_digest,
        "evolution_state_path": normalize_rel_path(&root, &out_path)
    }));
    append_history(&history_path(&root, args), &out);
    out
}

pub fn memory_causal_retrieve_payload(args: &HashMap<String, String>) -> Value {
    let root = root_from_args(args);
    let graph_path = causality_path(&root, args);
    let Some(raw) = fs::read_to_string(&graph_path).ok() else {
        return receipt(json!({
            "ok": false,
            "type": "memory_causal_retrieve",
            "error": "causality_graph_missing",
            "graph_path": normalize_rel_path(&root, &graph_path)
        }));
    };
    let Ok(graph) = serde_json::from_str::<CausalityGraph>(&raw) else {
        return receipt(json!({
            "ok": false,
            "type": "memory_causal_retrieve",
            "error": "causality_graph_invalid",
            "graph_path": normalize_rel_path(&root, &graph_path)
        }));
    };
    let q = clean_text(args.get("q").map_or("", String::as_str), 200);
    let depth = parse_usize(args.get("depth"), 1, 6, 2);
    let seed = if !q.is_empty() {
        graph
            .nodes
            .iter()
            .find(|n| {
                n.summary
                    .to_ascii_lowercase()
                    .contains(&q.to_ascii_lowercase())
                    || n.event_type
                        .to_ascii_lowercase()
                        .contains(&q.to_ascii_lowercase())
            })
            .map(|n| n.id.clone())
            .or_else(|| graph.nodes.first().map(|n| n.id.clone()))
            .unwrap_or_default()
    } else {
        graph
            .nodes
            .first()
            .map(|n| n.id.clone())
            .unwrap_or_default()
    };
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    for edge in &graph.edges {
        adjacency
            .entry(edge.from.clone())
            .or_default()
            .push(edge.to.clone());
    }
    let mut visited = HashSet::new();
    let mut queue: VecDeque<(String, usize)> = VecDeque::new();
    queue.push_back((seed.clone(), 0));
    let mut trace = Vec::new();
    while let Some((cur, d)) = queue.pop_front() {
        if !visited.insert(cur.clone()) {
            continue;
        }
        if let Some(node) = graph.nodes.iter().find(|n| n.id == cur) {
            trace.push(json!({
                "id": node.id,
                "depth": d,
                "event_type": node.event_type,
                "summary": node.summary
            }));
        }
        if d >= depth {
            continue;
        }
        for nxt in adjacency.get(&cur).cloned().unwrap_or_default() {
            queue.push_back((nxt, d + 1));
        }
    }
    let out = receipt(json!({
        "ok": true,
        "type": "memory_causal_retrieve",
        "backend": "protheus_memory_core",
        "query": q,
        "seed": seed,
        "depth": depth,
        "trace_count": trace.len(),
        "trace": trace,
        "graph_path": normalize_rel_path(&root, &graph_path)
    }));
    append_history(&history_path(&root, args), &out);
    out
}

pub fn memory_fuse_payload(args: &HashMap<String, String>) -> Value {
    let root = root_from_args(args);
    let taxonomy_rows = read_json_file(&taxonomy_path(&root, args))
        .and_then(|v| v.get("rows").and_then(Value::as_array).map(|v| v.len()))
        .unwrap_or(0usize);
    let causality = read_json_file(&causality_path(&root, args))
        .and_then(|v| v.get("node_count").and_then(Value::as_u64))
        .unwrap_or(0) as usize;
    let meta = load_history_rows(&metacognitive_journal_path(&root, args)).len();
    let fusion_score = ((taxonomy_rows as f64 * 0.4 + causality as f64 * 0.4 + meta as f64 * 0.2)
        / 100.0)
        .clamp(0.0, 1.0);
    let snapshot = json!({
        "schema_version": "1.0",
        "generated_at": now_iso(),
        "taxonomy_rows": taxonomy_rows,
        "causality_nodes": causality,
        "metacognitive_events": meta,
        "fusion_score": ((fusion_score * 1000.0).round() / 1000.0)
    });
    let out_path = fusion_state_path(&root, args);
    if let Some(parent) = out_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(
        &out_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string())
        ),
    );
    let out = receipt(json!({
        "ok": true,
        "type": "memory_fuse",
        "backend": "protheus_memory_core",
        "fusion_score": snapshot.get("fusion_score").cloned().unwrap_or(Value::Null),
        "fusion_state_path": normalize_rel_path(&root, &out_path)
    }));
    append_history(&history_path(&root, args), &out);
    out
}

pub fn nano_chat_payload(args: &HashMap<String, String>) -> Value {
    let root = root_from_args(args);
    let query = clean_text(args.get("q").map_or("nano mode", String::as_str), 500);
    let top = parse_usize(args.get("top"), 1, 20, 5);
    let transport = clean_text(args.get("transport").map_or("cli+web", String::as_str), 80);
    let latest_path = nanochat_latest_path(&root, args);

    let out = receipt(json!({
        "ok": true,
        "type": "nano_chat_mode",
        "backend": "protheus_memory_core",
        "query": query,
        "top": top,
        "transport": transport,
        "history_enabled": true,
        "state_path": normalize_rel_path(&root, &latest_path)
    }));
    if let Some(parent) = latest_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(
        &latest_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&out).unwrap_or_else(|_| "{}".to_string())
        ),
    );
    append_history(&history_path(&root, args), &out);
    out
}

pub fn nano_train_payload(args: &HashMap<String, String>) -> Value {
    let root = root_from_args(args);
    let depth = parse_usize(args.get("depth"), 1, 64, 12);
    let profile = clean_text(args.get("profile").map_or("nanochat", String::as_str), 80);
    let state_dir = nanochat_state_dir(&root, args);
    let checkpoints = state_dir.join("checkpoints");
    let ckpt = checkpoints.join(format!("depth_{depth}.json"));
    let _ = fs::create_dir_all(&checkpoints);
    let pipeline = json!({
        "stages": ["tokenizer", "pretrain", "sft", "rl"],
        "depth": depth,
        "profile": profile,
        "generated_at": now_iso()
    });
    let _ = fs::write(
        &ckpt,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&pipeline).unwrap_or_else(|_| "{}".to_string())
        ),
    );

    let out = receipt(json!({
        "ok": true,
        "type": "nano_train_mode",
        "backend": "protheus_memory_core",
        "depth": depth,
        "profile": profile,
        "pipeline_stages": ["tokenizer", "pretrain", "sft", "rl"],
        "checkpoint_path": normalize_rel_path(&root, &ckpt)
    }));
    append_history(&history_path(&root, args), &out);
    out
}

pub fn nano_fork_payload(args: &HashMap<String, String>) -> Value {
    let root = root_from_args(args);
    let target = clean_text(
        args.get("target").map_or(".nanochat/fork", String::as_str),
        400,
    );
    let target_path = {
        let p = PathBuf::from(target.clone());
        if p.is_absolute() {
            p
        } else {
            root.join(p)
        }
    };
    let _ = fs::create_dir_all(&target_path);
    let readme = target_path.join("README.md");
    if !readme.exists() {
        let _ = fs::write(
            &readme,
            "# NanoChat Fork Mode\n\nGenerated by `protheus nano fork`.\n",
        );
    }

    let out = receipt(json!({
        "ok": true,
        "type": "nano_fork_mode",
        "backend": "protheus_memory_core",
        "target": target,
        "target_path": normalize_rel_path(&root, &target_path),
        "readme_path": normalize_rel_path(&root, &readme)
    }));
    append_history(&history_path(&root, args), &out);
    out
}

pub fn stable_status_payload() -> Value {
    receipt(json!({
        "ok": true,
        "type": "memory_stable_api_status",
        "backend": "protheus_memory_core",
        "stable_api_version": "v1",
        "supported_versions": ["stable", "v1", "1"],
        "commands": [
            "stable-status",
            "stable-search",
            "stable-get-node",
            "stable-build-index",
            "memory-upgrade-byterover",
            "stable-memory-upgrade-byterover",
            "stable-rag-ingest",
            "stable-rag-search",
            "stable-rag-chat",
            "stable-nano-chat",
            "stable-nano-train",
            "stable-nano-fork",
            "stable-memory-taxonomy",
            "stable-memory-enable-metacognitive",
            "stable-memory-enable-causality",
            "stable-memory-benchmark-ama",
            "stable-memory-share",
            "stable-memory-evolve",
            "stable-memory-causal-retrieve",
            "stable-memory-fuse"
        ]
    }))
}

pub fn ensure_supported_version(args: &HashMap<String, String>) -> Result<String, Value> {
    let version = clean_text(args.get("api-version").map_or("stable", String::as_str), 20)
        .to_ascii_lowercase();
    let normalized = if version == "1" {
        "v1".to_string()
    } else {
        version
    };
    if normalized == "stable" || normalized == "v1" {
        Ok(normalized)
    } else {
        Err(receipt(json!({
            "ok": false,
            "type": "memory_stable_api_error",
            "error": "unsupported_api_version",
            "requested_version": normalized,
            "supported_versions": ["stable", "v1", "1"]
        })))
    }
}

