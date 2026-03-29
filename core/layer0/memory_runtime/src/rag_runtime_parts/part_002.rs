pub fn byterover_upgrade_payload(args: &HashMap<String, String>) -> Value {
    let root = root_from_args(args);
    let brv = byterover_root(&root, args);
    let ctx = brv.join("context-tree");
    let timeline = ctx.join("timeline.md");
    let facts = ctx.join("facts.md");
    let meaning = ctx.join("meaning.md");
    let rules = ctx.join("rules.md");
    let manifest = ctx.join("manifest.json");

    let _ = fs::create_dir_all(&ctx);
    let mut created = Vec::new();
    for (path, title) in [
        (&timeline, "Timeline"),
        (&facts, "Facts"),
        (&meaning, "Meaning"),
        (&rules, "Rules"),
    ] {
        if !path.exists() {
            let body = format!("# {title}\n\nInitialized by `memory-upgrade-byterover`.\n");
            if fs::write(path, body).is_ok() {
                created.push(normalize_rel_path(&root, path));
            }
        }
    }

    let snapshot = json!({
        "schema_version": "1.0",
        "profile": "byterover",
        "generated_at": now_iso(),
        "paths": {
            "timeline": normalize_rel_path(&root, &timeline),
            "facts": normalize_rel_path(&root, &facts),
            "meaning": normalize_rel_path(&root, &meaning),
            "rules": normalize_rel_path(&root, &rules)
        }
    });
    let _ = fs::write(
        &manifest,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&snapshot).unwrap_or_else(|_| "{}".to_string())
        ),
    );

    let out = receipt(json!({
        "ok": true,
        "type": "memory_upgrade_byterover",
        "backend": "protheus_memory_core",
        "schema_version": "1.0",
        "profile": "byterover",
        "root": normalize_rel_path(&root, &brv),
        "context_tree_path": normalize_rel_path(&root, &ctx),
        "manifest_path": normalize_rel_path(&root, &manifest),
        "files_created": created,
        "created_count": created.len()
    }));
    append_history(&history_path(&root, args), &out);
    out
}

pub fn memory_metacognitive_enable_payload(args: &HashMap<String, String>) -> Value {
    let root = root_from_args(args);
    let enabled = parse_bool(args.get("enabled"), true);
    let note = clean_text(args.get("note").map_or("", String::as_str), 300);
    let config_digest = sha256_hex(
        serde_json::to_string(&json!({
            "schema_version": "1.0",
            "enabled": enabled,
            "note": note
        }))
        .unwrap_or_default()
        .as_bytes(),
    );
    let cfg_path = metacognitive_config_path(&root, args);
    let payload = json!({
        "schema_version": "1.0",
        "enabled": enabled,
        "updated_at": now_iso(),
        "note": note
    });
    if let Some(parent) = cfg_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(
        &cfg_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string())
        ),
    );
    let out = receipt(json!({
        "ok": true,
        "type": "memory_metacognitive_enable",
        "backend": "protheus_memory_core",
        "enabled": enabled,
        "config_path": normalize_rel_path(&root, &cfg_path),
        "config_digest": config_digest
    }));
    append_history(&history_path(&root, args), &out);
    append_metacognitive_note(
        &root,
        args,
        json!({
            "type": "metacognitive_toggle",
            "ts": now_iso(),
            "enabled": enabled,
            "note": note
        }),
    );
    out
}

pub fn memory_taxonomy_payload(args: &HashMap<String, String>) -> Value {
    let root = root_from_args(args);
    let idx_path = index_path(&root, args);
    let Some(index) = load_index(&idx_path) else {
        return receipt(json!({
            "ok": false,
            "type": "memory_taxonomy_4w",
            "error": "index_missing",
            "index_path": normalize_rel_path(&root, &idx_path)
        }));
    };
    let which = effective_which(args);
    let mut rows = Vec::new();
    let mut what_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut how_counts: BTreeMap<String, usize> = BTreeMap::new();
    let mut when_missing = 0usize;
    for chunk in index.chunks {
        let when_value = parse_yyyy_mm_dd(&chunk.source_path);
        if when_value.is_empty() {
            when_missing += 1;
        }
        let what_value = classify_what(&chunk.source_path, &chunk.mime, &chunk.text);
        let how_value = classify_how(&chunk.source_path, &chunk.mime);
        *what_counts.entry(what_value.clone()).or_insert(0) += 1;
        *how_counts.entry(how_value.clone()).or_insert(0) += 1;
        let tokenized = tokenize(&chunk.text);
        let keywords = tokenized.into_iter().take(8).collect::<Vec<String>>();
        let mut confidence = 0.6_f64;
        if !when_value.is_empty() {
            confidence += 0.2;
        }
        if !keywords.is_empty() {
            confidence += 0.2;
        }
        rows.push(TaxonomyRow {
            chunk_id: chunk.chunk_id,
            source: chunk.source_path,
            when_value,
            what_value,
            how_value,
            which_value: which.clone(),
            confidence: (confidence * 1000.0).round() / 1000.0,
            keywords,
        });
    }
    rows.sort_by(|a, b| a.chunk_id.cmp(&b.chunk_id));
    let snapshot = TaxonomySnapshot {
        schema_version: "1.0".to_string(),
        generated_at: now_iso(),
        row_count: rows.len(),
        rows,
    };
    let taxonomy_digest = sha256_hex(
        serde_json::to_string(&snapshot.rows)
            .unwrap_or_default()
            .as_bytes(),
    );
    let out_path = taxonomy_path(&root, args);
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
        "type": "memory_taxonomy_4w",
        "backend": "protheus_memory_core",
        "index_path": normalize_rel_path(&root, &idx_path),
        "taxonomy_path": normalize_rel_path(&root, &out_path),
        "taxonomy_digest": taxonomy_digest,
        "row_count": snapshot.row_count,
        "which": which,
        "when_missing": when_missing,
        "what_counts": what_counts,
        "how_counts": how_counts
    }));
    append_history(&history_path(&root, args), &out);
    if metacognitive_enabled(&root, args) {
        append_metacognitive_note(
            &root,
            args,
            json!({
                "type": "taxonomy_reflection",
                "ts": now_iso(),
                "row_count": snapshot.row_count,
                "when_missing": when_missing,
                "dominant_what": out.get("what_counts").and_then(Value::as_object).and_then(|m| m.iter().max_by_key(|(_,v)| v.as_u64().unwrap_or(0)).map(|(k,_)| k.clone())).unwrap_or_else(|| "unknown".to_string())
            }),
        );
    }
    out
}

pub fn memory_causality_enable_payload(args: &HashMap<String, String>) -> Value {
    let root = root_from_args(args);
    let hist = history_path(&root, args);
    let rows = load_history_rows(&hist);
    if rows.is_empty() {
        return receipt(json!({
            "ok": false,
            "type": "memory_causality_enable",
            "error": "history_missing",
            "history_path": normalize_rel_path(&root, &hist)
        }));
    }
    let mut nodes = Vec::new();
    let mut edges = Vec::new();
    for (idx, row) in rows.iter().enumerate() {
        let seed = format!(
            "{}|{}|{}",
            row.get("receipt_hash")
                .and_then(Value::as_str)
                .unwrap_or(""),
            row.get("type").and_then(Value::as_str).unwrap_or(""),
            idx
        );
        let id = format!("evt.{}", &sha256_hex(seed.as_bytes())[..16]);
        let event_type = clean_text(
            row.get("type").and_then(Value::as_str).unwrap_or("event"),
            120,
        );
        let ts = clean_text(
            row.get("ts")
                .or_else(|| row.get("generated_at"))
                .and_then(Value::as_str)
                .unwrap_or(""),
            80,
        );
        let summary = clean_text(
            &format!(
                "{} {}",
                event_type,
                row.get("query")
                    .or_else(|| row.get("answer"))
                    .and_then(Value::as_str)
                    .unwrap_or("")
            ),
            220,
        );
        nodes.push(CausalNode {
            id: id.clone(),
            ts,
            event_type,
            summary,
        });
        if idx > 0 {
            let prev = nodes[idx - 1].id.clone();
            edges.push(CausalEdge {
                from: prev,
                to: id,
                relation: "temporal_precedes".to_string(),
            });
        }
    }
    let graph = CausalityGraph {
        schema_version: "1.0".to_string(),
        generated_at: now_iso(),
        node_count: nodes.len(),
        edge_count: edges.len(),
        nodes,
        edges,
    };
    let out_path = causality_path(&root, args);
    if let Some(parent) = out_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(
        &out_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&graph).unwrap_or_else(|_| "{}".to_string())
        ),
    );
    let out = receipt(json!({
        "ok": true,
        "type": "memory_causality_enable",
        "backend": "protheus_memory_core",
        "graph_path": normalize_rel_path(&root, &out_path),
        "history_path": normalize_rel_path(&root, &hist),
        "node_count": graph.node_count,
        "edge_count": graph.edge_count
    }));
    append_history(&history_path(&root, args), &out);
    if metacognitive_enabled(&root, args) {
        append_metacognitive_note(
            &root,
            args,
            json!({
                "type": "causality_reflection",
                "ts": now_iso(),
                "node_count": graph.node_count,
                "edge_count": graph.edge_count
            }),
        );
    }
    out
}

pub fn memory_benchmark_ama_payload(args: &HashMap<String, String>) -> Value {
    let root = root_from_args(args);
    let graph_path = causality_path(&root, args);
    let Some(raw) = fs::read_to_string(&graph_path).ok() else {
        return receipt(json!({
            "ok": false,
            "type": "memory_benchmark_ama",
            "error": "causality_graph_missing",
            "graph_path": normalize_rel_path(&root, &graph_path)
        }));
    };
    let Ok(graph) = serde_json::from_str::<CausalityGraph>(&raw) else {
        return receipt(json!({
            "ok": false,
            "type": "memory_benchmark_ama",
            "error": "causality_graph_invalid",
            "graph_path": normalize_rel_path(&root, &graph_path)
        }));
    };
    let threshold = args
        .get("threshold")
        .and_then(|v| v.parse::<f64>().ok())
        .unwrap_or(0.72)
        .clamp(0.1, 1.0);
    let node_ids = graph
        .nodes
        .iter()
        .map(|n| n.id.clone())
        .collect::<HashSet<String>>();
    let valid_edges = graph
        .edges
        .iter()
        .filter(|e| node_ids.contains(&e.from) && node_ids.contains(&e.to))
        .count();
    let edge_validity = if graph.edge_count == 0 {
        0.0
    } else {
        valid_edges as f64 / graph.edge_count as f64
    };
    let covered_nodes = graph
        .nodes
        .iter()
        .filter(|n| !n.summary.trim().is_empty())
        .count();
    let node_coverage = if graph.node_count == 0 {
        0.0
    } else {
        covered_nodes as f64 / graph.node_count as f64
    };
    let mut adjacency: HashMap<String, Vec<String>> = HashMap::new();
    for e in &graph.edges {
        adjacency
            .entry(e.from.clone())
            .or_default()
            .push(e.to.clone());
    }
    let mut two_hop = 0usize;
    for node in &graph.nodes {
        let mut seen: HashSet<String> = HashSet::new();
        let mut q: VecDeque<(String, usize)> = VecDeque::new();
        q.push_back((node.id.clone(), 0));
        while let Some((cur, depth)) = q.pop_front() {
            if depth >= 2 {
                continue;
            }
            for nxt in adjacency.get(&cur).cloned().unwrap_or_default() {
                if seen.insert(nxt.clone()) {
                    q.push_back((nxt, depth + 1));
                }
            }
        }
        if seen.len() >= 2 {
            two_hop += 1;
        }
    }
    let multi_hop_ratio = if graph.node_count == 0 {
        0.0
    } else {
        two_hop as f64 / graph.node_count as f64
    };
    let ama_score =
        (edge_validity * 0.5 + node_coverage * 0.3 + multi_hop_ratio * 0.2).clamp(0.0, 1.0);
    let pass = ama_score >= threshold;
    let benchmark = json!({
        "schema_version": "1.0",
        "generated_at": now_iso(),
        "graph_path": normalize_rel_path(&root, &graph_path),
        "metrics": {
            "edge_validity": ((edge_validity * 1000.0).round() / 1000.0),
            "node_coverage": ((node_coverage * 1000.0).round() / 1000.0),
            "multi_hop_ratio": ((multi_hop_ratio * 1000.0).round() / 1000.0),
            "ama_score": ((ama_score * 1000.0).round() / 1000.0),
            "threshold": ((threshold * 1000.0).round() / 1000.0),
            "pass": pass
        }
    });
    let out_path = ama_benchmark_path(&root, args);
    if let Some(parent) = out_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    let _ = fs::write(
        &out_path,
        format!(
            "{}\n",
            serde_json::to_string_pretty(&benchmark).unwrap_or_else(|_| "{}".to_string())
        ),
    );
    let out = receipt(json!({
        "ok": true,
        "type": "memory_benchmark_ama",
        "backend": "protheus_memory_core",
        "benchmark_path": normalize_rel_path(&root, &out_path),
        "graph_path": normalize_rel_path(&root, &graph_path),
        "ama_score": benchmark.get("metrics").and_then(|m| m.get("ama_score")).cloned().unwrap_or(Value::Null),
        "threshold": benchmark.get("metrics").and_then(|m| m.get("threshold")).cloned().unwrap_or(Value::Null),
        "pass": pass
    }));
    append_history(&history_path(&root, args), &out);
    out
}

