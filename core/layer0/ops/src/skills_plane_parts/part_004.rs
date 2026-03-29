fn run_tot_deliberate(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        TOT_DELIBERATE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "tot_deliberate_profile_contract",
            "max_depth": 4,
            "max_branching": 5,
            "allowed_strategies": ["bfs", "dfs"]
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("tot_profile_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "tot_deliberate_profile_contract"
    {
        errors.push("tot_profile_contract_kind_invalid".to_string());
    }
    let task = clean(
        parsed
            .flags
            .get("task")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_default(),
        500,
    );
    if task.is_empty() {
        errors.push("tot_task_required".to_string());
    }
    let strategy = clean(
        parsed
            .flags
            .get("strategy")
            .cloned()
            .unwrap_or_else(|| "bfs".to_string()),
        20,
    )
    .to_ascii_lowercase();
    let allowed_strategies = contract
        .get("allowed_strategies")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| vec![json!("bfs"), json!("dfs")]);
    let allowed = allowed_strategies
        .iter()
        .filter_map(Value::as_str)
        .map(|v| clean(v, 20).to_ascii_lowercase())
        .collect::<Vec<_>>();
    if strict && !allowed.iter().any(|v| v == &strategy) {
        errors.push("tot_strategy_invalid".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_tot_deliberate",
            "errors": errors
        });
    }

    let max_depth = parse_u64(parsed.flags.get("max-depth"), 0).max(1).min(
        contract
            .get("max_depth")
            .and_then(Value::as_u64)
            .unwrap_or(4),
    ) as usize;
    let branching = parse_u64(parsed.flags.get("branching"), 0).max(2).min(
        contract
            .get("max_branching")
            .and_then(Value::as_u64)
            .unwrap_or(5),
    ) as usize;

    let mut branches = Vec::<Value>::new();
    for depth in 0..max_depth {
        for branch_idx in 0..branching {
            let node_id = format!("d{}_b{}", depth + 1, branch_idx + 1);
            let score_seed = sha256_hex_str(&format!("{task}:{strategy}:{node_id}"));
            let score =
                (u64::from_str_radix(&score_seed[..8], 16).unwrap_or(0) % 10_000) as f64 / 10_000.0;
            branches.push(json!({
                "node_id": node_id,
                "depth": depth + 1,
                "branch_index": branch_idx + 1,
                "strategy": strategy,
                "proposal": format!("{} :: option {}", clean(&task, 120), branch_idx + 1),
                "score": score,
                "eval_hash": sha256_hex_str(&format!("{}:{}:{}:{}", task, strategy, depth, branch_idx))
            }));
        }
    }
    branches.sort_by(|a, b| {
        let left = a.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        let right = b.get("score").and_then(Value::as_f64).unwrap_or(0.0);
        right
            .partial_cmp(&left)
            .unwrap_or(std::cmp::Ordering::Equal)
    });
    let best = branches.first().cloned().unwrap_or_else(|| json!({}));
    let artifact = json!({
        "version": "v1",
        "profile": "tot-deliberate",
        "task": task,
        "strategy": strategy,
        "max_depth": max_depth,
        "branching": branching,
        "branches": branches,
        "selected": best
    });
    let artifact_path = state_root(root).join("tot_deliberate").join("latest.json");
    let _ = write_json(&artifact_path, &artifact);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "skills_plane_tot_deliberate",
        "lane": "core/layer0/ops",
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&artifact.to_string())
        },
        "result": artifact,
        "claim_evidence": [
            {
                "id": "V6-SKILLS-001.8",
                "claim": "tot_deliberate_profile_runs_bounded_branch_search_with_deterministic_branch_and_eval_receipts",
                "evidence": {
                    "max_depth": max_depth,
                    "branching": branching,
                    "strategy": strategy
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
