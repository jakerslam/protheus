fn run_crawl(root: &Path, parsed: &ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "research_plane_contract",
            "crawler": {
                "max_concurrency": 100,
                "per_domain_qps": 2,
                "max_retries": 3,
                "checkpoint_every": 10
            }
        }),
    );
    let policy = load_json_or(
        root,
        POLICY_PATH,
        json!({
            "version": "v1",
            "kind": "research_plane_policy",
            "crawler": {
                "max_concurrency": 100,
                "per_domain_qps": 2,
                "max_retries": 3,
                "checkpoint_every": 10
            },
            "timeouts": {"crawl_fetch_ms": 8000}
        }),
    );
    let max_pages = parse_u64(parsed.flags.get("max-pages"), 100).clamp(1, 10_000) as usize;
    let max_concurrency = parse_u64(
        parsed.flags.get("max-concurrency"),
        policy
            .get("crawler")
            .and_then(|v| v.get("max_concurrency"))
            .and_then(Value::as_u64)
            .unwrap_or(100),
    )
    .clamp(1, 1000) as usize;
    let per_domain_qps = parse_u64(
        parsed.flags.get("per-domain-qps"),
        policy
            .get("crawler")
            .and_then(|v| v.get("per_domain_qps"))
            .and_then(Value::as_u64)
            .unwrap_or(2),
    )
    .clamp(1, 100);
    let max_retries = parse_u64(
        parsed.flags.get("max-retries"),
        policy
            .get("crawler")
            .and_then(|v| v.get("max_retries"))
            .and_then(Value::as_u64)
            .unwrap_or(3),
    )
    .clamp(0, 20);
    let checkpoint_every = parse_u64(
        parsed.flags.get("checkpoint-every"),
        policy
            .get("crawler")
            .and_then(|v| v.get("checkpoint_every"))
            .and_then(Value::as_u64)
            .unwrap_or(10),
    )
    .clamp(1, 1_000) as usize;
    let fetch_timeout = parse_u64(
        parsed.flags.get("timeout-ms"),
        policy
            .get("timeouts")
            .and_then(|v| v.get("crawl_fetch_ms"))
            .and_then(Value::as_u64)
            .unwrap_or(8_000),
    )
    .clamp(1_000, 120_000);
    let checkpoint_rel = parsed
        .flags
        .get("checkpoint-path")
        .cloned()
        .unwrap_or_else(|| {
            state_root(root)
                .join("crawl")
                .join("checkpoint.json")
                .display()
                .to_string()
        });
    let checkpoint_path = if Path::new(&checkpoint_rel).is_absolute() {
        PathBuf::from(&checkpoint_rel)
    } else {
        root.join(&checkpoint_rel)
    };
    let resume = parse_bool(parsed.flags.get("resume"), false);
    let seeds = parse_seed_urls(parsed);
    let mut errors = Vec::<String>::new();

    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("research_contract_version_must_be_v1".to_string());
    }
    if policy
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("research_policy_version_must_be_v1".to_string());
    }

    let mut queue = VecDeque::<Value>::new();
    let mut visited = Vec::<Value>::new();
    let mut failures = Vec::<Value>::new();
    let mut retries = 0_u64;
    let mut clock_ms = 0_u64;

    if resume && checkpoint_path.exists() {
        let checkpoint = read_json(&checkpoint_path).unwrap_or(Value::Null);
        if let Some(rows) = checkpoint.get("queue").and_then(Value::as_array) {
            for row in rows {
                queue.push_back(row.clone());
            }
        }
        if let Some(rows) = checkpoint.get("visited").and_then(Value::as_array) {
            visited = rows.clone();
        }
        if let Some(rows) = checkpoint.get("failures").and_then(Value::as_array) {
            failures = rows.clone();
        }
        retries = checkpoint
            .get("retries")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        clock_ms = checkpoint
            .get("clock_ms")
            .and_then(Value::as_u64)
            .unwrap_or(0);
    } else {
        for url in &seeds {
            queue.push_back(json!({
                "url": url,
                "attempt": 0_u64,
                "ready_at_ms": 0_u64
            }));
        }
    }

    if queue.is_empty() {
        errors.push("crawl_seed_urls_required".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "research_plane_crawl",
            "errors": errors
        });
    }

    let mut domain_next_allowed = BTreeMap::<String, u64>::new();
    let interval_ms = (1000_u64 / per_domain_qps).max(1);
    let started = Instant::now();
    let mut loops_without_progress = 0_u32;

    while visited.len() < max_pages && !queue.is_empty() {
        let mut launched = 0_usize;
        let mut deferred = VecDeque::<Value>::new();
        while let Some(job) = queue.pop_front() {
            if launched >= max_concurrency {
                deferred.push_back(job);
                continue;
            }
            let ready_at = job.get("ready_at_ms").and_then(Value::as_u64).unwrap_or(0);
            if ready_at > clock_ms {
                deferred.push_back(job);
                continue;
            }
            let url = job
                .get("url")
                .and_then(Value::as_str)
                .unwrap_or_default()
                .to_string();
            let attempt = job.get("attempt").and_then(Value::as_u64).unwrap_or(0);
            let domain = url_domain(&url);
            let next_allowed = domain_next_allowed.get(&domain).copied().unwrap_or(0);
            if next_allowed > clock_ms {
                let mut shifted = job.clone();
                shifted["ready_at_ms"] = Value::Number(next_allowed.into());
                deferred.push_back(shifted);
                continue;
            }

            launched += 1;
            let fetched = fetch_auto(
                root,
                &url,
                "auto",
                fetch_timeout,
                300_000,
                &policy,
                &contract,
                strict,
            );
            let ok = fetched.get("ok").and_then(Value::as_bool).unwrap_or(false);
            let status = fetched.get("status").and_then(Value::as_i64).unwrap_or(0);
            domain_next_allowed.insert(domain.clone(), clock_ms.saturating_add(interval_ms));

            if ok {
                visited.push(json!({
                    "url": url,
                    "status": status,
                    "domain": domain,
                    "body_sha256": fetched.get("body_sha256").cloned().unwrap_or(Value::Null)
                }));
                loops_without_progress = 0;
            } else if attempt < max_retries {
                retries = retries.saturating_add(1);
                let backoff = 500_u64.saturating_mul(2_u64.saturating_pow(attempt as u32));
                deferred.push_back(json!({
                    "url": url,
                    "attempt": attempt.saturating_add(1),
                    "ready_at_ms": clock_ms.saturating_add(backoff)
                }));
            } else {
                failures.push(json!({
                    "url": url,
                    "status": status,
                    "attempt": attempt,
                    "error": fetched.get("error").cloned().unwrap_or(Value::Null)
                }));
            }
        }
        queue = deferred;
        if launched == 0 {
            loops_without_progress = loops_without_progress.saturating_add(1);
            clock_ms = clock_ms.saturating_add(interval_ms);
            if loops_without_progress > 2000 {
                errors.push("crawl_scheduler_stalled".to_string());
                break;
            }
        } else {
            clock_ms = clock_ms.saturating_add(5);
        }

        let checkpoint_due = (!visited.is_empty() && visited.len() % checkpoint_every == 0)
            || queue.is_empty();
        if checkpoint_due {
            if let Some(parent) = checkpoint_path.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let checkpoint = json!({
                "ts": chrono::Utc::now().to_rfc3339(),
                "queue": queue,
                "visited": visited,
                "failures": failures,
                "retries": retries,
                "clock_ms": clock_ms
            });
            let _ = fs::write(
                &checkpoint_path,
                serde_json::to_string_pretty(&checkpoint).unwrap_or_else(|_| "{}".to_string())
                    + "\n",
            );
        }
    }

    let elapsed_ms = started.elapsed().as_millis();
    let ok = errors.is_empty();
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "type": "research_plane_crawl",
        "lane": "core/layer0/ops",
        "seed_count": seeds.len(),
        "visited_count": visited.len(),
        "failure_count": failures.len(),
        "retries": retries,
        "max_pages": max_pages,
        "max_concurrency": max_concurrency,
        "per_domain_qps": per_domain_qps,
        "checkpoint_path": checkpoint_path.display().to_string(),
        "elapsed_ms": elapsed_ms,
        "visited": visited,
        "failures": failures,
        "errors": errors,
        "claim_evidence": [
            {
                "id": "V6-RESEARCH-001.3",
                "claim": "crawler_enforces_domain_throttle_checkpoint_and_backoff_resilience",
                "evidence": {
                    "visited_count": visited.len(),
                    "retries": retries
                }
            },
            {
                "id": "V6-RESEARCH-001.5",
                "claim": "crawl_fetch_paths_including_stealth_browser_fallback_are_safety_plane_routed",
                "evidence": {
                    "visited_count": visited.len()
                }
            }
        ]
    })
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let raw_command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    if matches!(raw_command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let command = canonical_research_command(&raw_command).to_string();
    let strict = parse_bool(parsed.flags.get("strict"), true);
    let conduit = if command_uses_top_level_conduit(&command) {
        Some(research_batch6::conduit_enforcement(
            root, &parsed, strict, &command,
        ))
    } else {
        None
    };
    if strict
        && conduit
            .as_ref()
            .and_then(|v| v.get("ok"))
            .and_then(Value::as_bool)
            == Some(false)
    {
        return emit(
            root,
            json!({
                "ok": false,
                "strict": strict,
                "type": top_level_conduit_receipt_type(&command),
                "errors": ["conduit_bypass_rejected"],
                "conduit_enforcement": conduit
            }),
        );
    }
    let payload = match command.as_str() {
        "status" => status(root),
        "diagnostics" => diagnostics(root),
        "fetch" => run_fetch(root, &parsed, strict),
        "recover-selectors" => run_recover_selectors(root, &parsed, strict),
        "crawl" => run_crawl(root, &parsed, strict),
        "mcp-extract" => research_batch6::run_mcp_extract(root, &parsed, strict),
        "spider" => crawl_spider::run(root, &parsed, strict),
        "middleware" => crawl_middleware::run(root, &parsed, strict),
        "pipeline" => crawl_pipeline::run(root, &parsed, strict),
        "signals" => crawl_signals::run(root, &parsed, strict),
        "console" => crawl_console::run(root, &parsed, strict),
        "template-governance" => {
            research_batch6::run_template_governance(root, &parsed, strict)
        }
        "goal-crawl" => research_batch7::run_goal_crawl(root, &parsed, strict),
        "map-site" => research_batch7::run_map_site(root, &parsed, strict),
        "extract-structured" => research_batch7::run_extract_structured(root, &parsed, strict),
        "monitor" => research_batch7::run_monitor(root, &parsed, strict),
        "firecrawl-template-governance" => {
            research_batch7::run_firecrawl_template_governance(root, &parsed, strict)
        }
        "js-scrape" => research_batch7::run_js_scrape(root, &parsed, strict),
        "auth-session" => research_batch7::run_auth_session(root, &parsed, strict),
        "proxy-rotate" => research_batch7::run_proxy_rotate(root, &parsed, strict),
        "parallel-scrape-workers" => {
            research_batch8::run_parallel_scrape_workers(root, &parsed, strict)
        }
        "book-patterns-template-governance" => {
            research_batch8::run_book_patterns_template_governance(root, &parsed, strict)
        }
        "decode-news-url" => research_batch8::run_decode_news_url(root, &parsed, strict),
        "decode-news-urls" => research_batch8::run_decode_news_urls(root, &parsed, strict),
        "decoder-template-governance" => {
            research_batch8::run_decoder_template_governance(root, &parsed, strict)
        }
        _ => json!({
            "ok": false,
            "type": "research_plane_error",
            "error": "unknown_command",
            "command": command,
            "requested_command": raw_command
        }),
    };
    if command == "status" {
        print_payload(&payload);
        return 0;
    }
    emit(root, attach_conduit_if_missing(payload, conduit.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selector_recovery_uses_text_fallback() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = parse_args(&[
            "recover-selectors".to_string(),
            "--html=<div>hello world</div>".to_string(),
            "--selectors=#missing,.missing".to_string(),
            "--target-text=hello world".to_string(),
        ]);
        let out = run_recover_selectors(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("recovered_strategy").and_then(Value::as_str),
            Some("text")
        );
    }

    #[test]
    fn selector_recovery_accepts_xpath_prefixed_selector() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = parse_args(&[
            "recover-selectors".to_string(),
            "--html=<main><p>hello world</p></main>".to_string(),
            "--selectors=xpath=//main".to_string(),
        ]);
        let out = run_recover_selectors(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("recovered_selector").and_then(Value::as_str),
            Some("xpath=//main")
        );
        assert_eq!(
            out.get("recovered_strategy").and_then(Value::as_str),
            Some("css_or_xpath")
        );
    }

    #[test]
    fn selector_recovery_matches_class_token_boundaries() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = parse_args(&[
            "recover-selectors".to_string(),
            "--html=<section class=\"foo bar\"><p>hello</p></section>".to_string(),
            "--selectors=.foo".to_string(),
        ]);
        let out = run_recover_selectors(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            out.get("recovered_selector").and_then(Value::as_str),
            Some(".foo")
        );
    }

    #[test]
    fn crawl_requires_seed_urls() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = parse_args(&["crawl".to_string()]);
        let out = run_crawl(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }
}
