#[allow(dead_code)]
pub(crate) fn contains_forbidden_runtime_context_marker(raw: &str) -> bool {
    const FORBIDDEN: [&str; 6] = [
        "You are an expert Python programmer.",
        "[PATCH v2",
        "List Leaves (25",
        "BEGIN_OPENCLAW_INTERNAL_CONTEXT",
        "END_OPENCLAW_INTERNAL_CONTEXT",
        "UNTRUSTED_CHILD_RESULT_DELIMITER",
    ];
    FORBIDDEN.iter().any(|marker| raw.contains(marker))
}

fn query_index_payload(args: &HashMap<String, String>) -> QueryResult {
    let root = PathBuf::from(arg_or_default(args, "root", "."));
    let q = arg_or_default(args, "q", "");
    let requested_top = parse_clamped_usize(
        arg_or_default(args, "top", DEFAULT_RECALL_TOP.to_string().as_str()).as_str(),
        1,
        1_000,
        DEFAULT_RECALL_TOP,
    );
    let tag_filters = parse_tag_filters(&arg_or_default(args, "tags", ""));
    let cache_path = arg_or_default(args, "cache-path", "");
    let cache_max_bytes = parse_cache_max_bytes(&arg_or_default(args, "cache-max-bytes", ""));
    let mut cache = if cache_path.is_empty() {
        None
    } else {
        Some(load_working_set_cache(&cache_path))
    };
    let requested_expand_lines = parse_clamped_usize(
        &arg_any(args, &["expand-lines", "excerpt-lines"]),
        0,
        1_000,
        DEFAULT_EXPAND_LINES,
    );
    let requested_max_files = parse_clamped_usize(
        &arg_any(args, &["max-files", "max_files"]),
        1,
        100,
        DEFAULT_MAX_FILES,
    );
    let budget_mode = FailClosedMode::from_raw(&arg_any(
        args,
        &["budget-mode", "budget_mode", "cap-mode", "cap_mode"],
    ));
    let budget_decision = enforce_recall_budget(&RecallBudgetInput {
        requested_top,
        requested_max_files,
        requested_expand_lines,
        mode: budget_mode,
        max_top: MAX_RECALL_TOP,
        max_files: MAX_MAX_FILES,
        max_expand_lines: MAX_EXPAND_LINES,
    });
    if !budget_decision.ok {
        return query_error(
            budget_decision.reason_code,
            vec![],
            vec![],
            json!({
                "budget": {
                    "mode": match budget_mode { FailClosedMode::Reject => "reject", FailClosedMode::Trim => "trim" },
                    "requested": {
                        "top": requested_top,
                        "max_files": requested_max_files,
                        "expand_lines": requested_expand_lines
                    },
                    "caps": {
                        "top": MAX_RECALL_TOP,
                        "max_files": MAX_MAX_FILES,
                        "expand_lines": MAX_EXPAND_LINES
                    }
                }
            }),
            None,
            None,
        );
    }
    let top = budget_decision.effective_top;
    let expand_lines = budget_decision.effective_expand_lines;
    let max_files = budget_decision.effective_max_files;
    let runtime_index = load_runtime_index(&root, args);
    let newest_index_mtime = newest_runtime_index_mtime_ms(&root, &runtime_index);
    let estimated_hydration_tokens = estimate_hydration_tokens(&runtime_index);
    let index_sources = runtime_index.index_sources;
    let tag_sources = runtime_index.tag_sources;
    let entries = runtime_index.entries;
    let tag_map = runtime_index.tag_map;
    let score_mode_raw = arg_any(args, &["score-mode", "score_mode"]).to_lowercase();
    let score_mode = score_mode_raw
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '_' || *ch == '-')
        .collect::<String>();
    let index_first = enforce_index_first(&index_sources, entries.len());
    if !index_first.ok {
        return query_error(
            index_first.reason_code,
            index_sources,
            tag_sources,
            json!({
                "budget": {
                    "requested": {
                        "top": requested_top,
                        "max_files": requested_max_files,
                        "expand_lines": requested_expand_lines
                    },
                    "effective": {
                        "top": top,
                        "max_files": max_files,
                        "expand_lines": expand_lines
                    },
                    "mode": match budget_mode { FailClosedMode::Reject => "reject", FailClosedMode::Trim => "trim" },
                    "trimmed": budget_decision.trimmed
                },
                "index_first": index_first.reason_code
            }),
            None,
            None,
        );
    }
    let allow_stale = parse_bool_arg(&arg_any(args, &["allow-stale", "allow_stale"]), false);
    let max_index_age_ms = parse_u64_clamped(
        &arg_any(args, &["max-index-age-ms", "max_index_age_ms"]),
        1_000,
        7 * 24 * 60 * 60 * 1000,
        DEFAULT_INDEX_MAX_AGE_MS,
    );
    let freshness_decision = enforce_index_freshness(
        now_epoch_ms(),
        newest_index_mtime,
        max_index_age_ms,
        allow_stale,
    );
    let freshness_payload = json!({
        "ok": freshness_decision.ok,
        "stale": freshness_decision.stale,
        "reason_code": freshness_decision.reason_code,
        "age_ms": freshness_decision.age_ms,
        "threshold_ms": freshness_decision.threshold_ms
    });
    if !freshness_decision.ok {
        return query_error(
            freshness_decision.reason_code,
            index_sources,
            tag_sources,
            json!({
                "budget": {
                    "requested": {
                        "top": requested_top,
                        "max_files": requested_max_files,
                        "expand_lines": requested_expand_lines
                    },
                    "effective": {
                        "top": top,
                        "max_files": max_files,
                        "expand_lines": expand_lines
                    },
                    "mode": match budget_mode { FailClosedMode::Reject => "reject", FailClosedMode::Trim => "trim" },
                    "trimmed": budget_decision.trimmed
                },
                "index_first": index_first.reason_code
            }),
            Some(freshness_payload),
            None,
        );
    }
    let bootstrap = parse_bool_arg(&arg_any(args, &["bootstrap"]), false);
    let hydrate_mode = arg_any(args, &["hydrate-mode", "hydrate_mode", "hydrate"]);
    let lazy_hydration = if hydrate_mode.trim().is_empty() {
        true
    } else {
        matches!(hydrate_mode.trim().to_ascii_lowercase().as_str(), "lazy")
    };
    let hydration_tokens = parse_u32_clamped(
        &arg_any(args, &["hydration-token-estimate", "hydration_tokens"]),
        0,
        4_000,
        estimated_hydration_tokens,
    );
    let hydration_cap = parse_u32_clamped(
        &arg_any(
            args,
            &[
                "bootstrap-hydration-token-cap",
                "bootstrap_hydration_token_cap",
            ],
        ),
        1,
        4_000,
        DEFAULT_BOOTSTRAP_HYDRATION_TOKEN_CAP,
    );
    let hydration_guard = enforce_hydration_guard(&HydrationGuardInput {
        bootstrap,
        lazy_hydration,
        estimated_hydration_tokens: hydration_tokens,
        max_bootstrap_tokens: hydration_cap,
        force: parse_bool_arg(
            &arg_any(args, &["force-hydration", "force_hydration"]),
            false,
        ),
    });
    if !hydration_guard.ok {
        return query_error(
            hydration_guard.reason_code,
            index_sources,
            tag_sources,
            json!({
                "budget": {
                    "requested": {
                        "top": requested_top,
                        "max_files": requested_max_files,
                        "expand_lines": requested_expand_lines
                    },
                    "effective": {
                        "top": top,
                        "max_files": max_files,
                        "expand_lines": expand_lines
                    },
                    "mode": match budget_mode { FailClosedMode::Reject => "reject", FailClosedMode::Trim => "trim" },
                    "trimmed": budget_decision.trimmed
                },
                "index_first": index_first.reason_code,
                "hydration": {
                    "bootstrap": bootstrap,
                    "lazy_hydration": lazy_hydration,
                    "estimated_tokens": hydration_tokens,
                    "token_cap": hydration_cap,
                    "reason_code": hydration_guard.reason_code
                }
            }),
            Some(freshness_payload),
            None,
        );
    }
    let vector_enabled = score_mode != "lexical";
    let mut tag_node_ids: HashSet<String> = HashSet::new();
    for tag in &tag_filters {
        if let Some(ids) = tag_map.get(tag) {
            for id in ids {
                tag_node_ids.insert(id.clone());
            }
        }
    }
    let mut candidates = entries.clone();
    if !tag_filters.is_empty() && !tag_node_ids.is_empty() {
        candidates = candidates
            .into_iter()
            .filter(|entry| tag_node_ids.contains(&entry.node_id))
            .collect::<Vec<IndexEntry>>();
    }
    let query_tokens = tokenize(&q);
    let session_id = arg_any(args, &["session-id", "session_id"]);
    let mut hits = if vector_enabled {
        hybrid_query_hits(
            &root,
            args,
            &RuntimeIndexBundle {
                entries: candidates.clone(),
                ..RuntimeIndexBundle::default()
            },
            &q,
            top,
            if session_id.trim().is_empty() {
                None
            } else {
                Some(session_id.as_str())
            },
        )
    } else {
        let mut scored = candidates
            .iter()
            .map(|entry| {
                let (lexical_score, reasons) =
                    score_entry(entry, &query_tokens, &tag_filters, &tag_node_ids);
                (entry, lexical_score, reasons)
            })
            .collect::<Vec<(&IndexEntry, i64, Vec<String>)>>();
        scored.sort_by(|a, b| {
            if b.1 != a.1 {
                return b.1.cmp(&a.1);
            }
            if a.0.file_rel != b.0.file_rel {
                return a.0.file_rel.cmp(&b.0.file_rel);
            }
            a.0.node_id.cmp(&b.0.node_id)
        });
        scored
            .into_iter()
            .take(top)
            .map(|(entry, score, reasons)| QueryHit {
                node_id: entry.node_id.clone(),
                uid: entry.uid.clone(),
                file: entry.file_rel.clone(),
                summary: entry.summary.clone(),
                tags: dedupe_sorted(entry.tags.clone()),
                score,
                reasons,
                memory_kind: None,
                trust_state: None,
                entity_refs: None,
                recall_explanation: None,
                section_excerpt: None,
                section_hash: None,
                section_source: None,
                expand_blocked: None,
                expand_error: None,
            })
            .collect::<Vec<QueryHit>>()
    };
    if expand_lines > 0 {
        let mut file_order = hits
            .iter()
            .map(|hit| hit.file.clone())
            .collect::<Vec<String>>();
        file_order = dedupe_sorted(file_order);
        let allowed_files = file_order
            .into_iter()
            .take(max_files)
            .collect::<HashSet<String>>();
        let mut file_cache: HashMap<String, String> = HashMap::new();
        for hit in hits.iter_mut() {
            if !allowed_files.contains(&hit.file) {
                hit.expand_blocked = Some("file_budget".to_string());
                continue;
            }
            let section_pair = if cache.is_some() {
                load_section_cached(&root, &hit.file, &hit.node_id, cache.as_mut())
            } else {
                let content = if let Some(cached) = file_cache.get(&hit.file) {
                    cached.clone()
                } else {
                    let file_abs = root.join(&hit.file);
                    match fs::read_to_string(&file_abs) {
                        Ok(text) => {
                            file_cache.insert(hit.file.clone(), text.clone());
                            text
                        }
                        Err(_) => {
                            hit.expand_error = Some("file_read_failed".to_string());
                            continue;
                        }
                    }
                };
                let section = extract_node_section(&content, &hit.node_id);
                if section.is_empty() {
                    Err("node_not_found".to_string())
                } else {
                    Ok((section.clone(), sha256_hex(&section)))
                }
            };
            match section_pair {
                Ok((section, section_hash)) => {
                    hit.section_source = Some("rust".to_string());
                    hit.section_hash = Some(section_hash);
                    hit.section_excerpt = Some(excerpt_lines(&section, expand_lines));
                }
                Err(reason) => {
                    hit.expand_error = Some(reason);
                }
            }
        }
    }
    if let Some(ref mut cache_ref) = cache {
        save_working_set_cache(&cache_path, cache_ref, cache_max_bytes);
    }
    let startup_tokens = parse_u32_clamped(
        &arg_any(args, &["startup-token-estimate", "startup_tokens"]),
        0,
        4_000,
        24,
    );
    let expanded_hits = hits
        .iter()
        .filter(|hit| hit.section_excerpt.is_some())
        .count() as u32;
    let retrieval_estimate = (query_tokens.len() as u32)
        .saturating_mul(6)
        .saturating_add((hits.len() as u32).saturating_mul(12))
        .saturating_add(expanded_hits.saturating_mul(24));
    let response_tokens = parse_u32_clamped(
        &arg_any(args, &["response-token-estimate", "response_tokens"]),
        0,
        4_000,
        (hits.len() as u32).saturating_mul(8),
    );
    let burn_threshold = parse_u32_clamped(
        &arg_any(
            args,
            &["burn-threshold", "burn_threshold", "burn-threshold-tokens"],
        ),
        1,
        10_000,
        DEFAULT_BURN_THRESHOLD_TOKENS,
    );
    let burn_mode = FailClosedMode::from_raw(&arg_any(
        args,
        &["burn-mode", "burn_mode", "burn-cap-mode", "burn_cap_mode"],
    ));
    let telemetry = TokenTelemetryEvent {
        startup_tokens,
        hydration_tokens,
        retrieval_tokens: retrieval_estimate,
        response_tokens,
        mode: if expand_lines > 0 {
            RetrievalMode::NodeRead
        } else {
            RetrievalMode::IndexOnly
        },
    };
    let burn_decision = evaluate_burn_slo(&telemetry, burn_threshold);
    let burn_payload = json!({
        "ok": burn_decision.ok,
        "reason_code": burn_decision.reason,
        "threshold_tokens": burn_decision.threshold_tokens,
        "total_tokens": burn_decision.total_tokens,
        "mode": telemetry.mode.as_str(),
        "components": {
            "startup_tokens": telemetry.startup_tokens,
            "hydration_tokens": telemetry.hydration_tokens,
            "retrieval_tokens": telemetry.retrieval_tokens,
            "response_tokens": telemetry.response_tokens
        }
    });
    if !burn_decision.ok && matches!(burn_mode, FailClosedMode::Reject) {
        return query_error(
            burn_decision.reason,
            index_sources,
            tag_sources,
            json!({
                "budget": {
                    "requested": {
                        "top": requested_top,
                        "max_files": requested_max_files,
                        "expand_lines": requested_expand_lines
                    },
                    "effective": {
                        "top": top,
                        "max_files": max_files,
                        "expand_lines": expand_lines
                    },
                    "mode": match budget_mode { FailClosedMode::Reject => "reject", FailClosedMode::Trim => "trim" },
                    "trimmed": budget_decision.trimmed
                },
                "index_first": index_first.reason_code,
                "hydration": {
                    "bootstrap": bootstrap,
                    "lazy_hydration": lazy_hydration,
                    "estimated_tokens": hydration_tokens,
                    "token_cap": hydration_cap,
                    "reason_code": hydration_guard.reason_code
                }
            }),
            Some(freshness_payload),
            Some(burn_payload),
        );
    }
    QueryResult {
        ok: true,
        backend: "infring_memory_core".to_string(),
        score_mode: if vector_enabled {
            "hybrid".to_string()
        } else {
            "lexical".to_string()
        },
        vector_enabled,
        recall_mode: if vector_enabled {
            "heap_hybrid".to_string()
        } else {
            "lexical_index".to_string()
        },
        entries_total: entries.len(),
        candidates_total: candidates.len(),
        index_sources,
        tag_sources,
        hits,
        session_id: if session_id.trim().is_empty() {
            None
        } else {
            Some(session_id)
        },
        error: None,
        reason_code: None,
        policy: Some(json!({
            "budget": {
                "requested": {
                    "top": requested_top,
                    "max_files": requested_max_files,
                    "expand_lines": requested_expand_lines
                },
                "effective": {
                    "top": top,
                    "max_files": max_files,
                    "expand_lines": expand_lines
                },
                "mode": match budget_mode { FailClosedMode::Reject => "reject", FailClosedMode::Trim => "trim" },
                "trimmed": budget_decision.trimmed
            },
            "index_first": index_first.reason_code,
            "hydration": {
                "bootstrap": bootstrap,
                "lazy_hydration": lazy_hydration,
                "estimated_tokens": hydration_tokens,
                "token_cap": hydration_cap,
                "reason_code": hydration_guard.reason_code
            }
        })),
        burn_slo: Some(burn_payload),
        freshness: Some(freshness_payload),
    }
}
