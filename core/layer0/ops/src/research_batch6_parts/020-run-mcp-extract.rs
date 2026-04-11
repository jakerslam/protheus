pub fn run_mcp_extract(root: &Path, parsed: &ParsedArgs, strict: bool) -> Value {
    let contract = read_json_or(
        root,
        MCP_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "mcp_extraction_contract",
            "required_artifacts": ["summary","links","entities","provenance"],
            "max_summary_chars": 600
        }),
    );
    let payload = parsed
        .flags
        .get("payload")
        .cloned()
        .or_else(|| parsed.flags.get("html").cloned())
        .or_else(|| {
            parsed.flags.get("payload-path").and_then(|p| {
                let path = if Path::new(p).is_absolute() {
                    PathBuf::from(p)
                } else {
                    root.join(p)
                };
                fs::read_to_string(path).ok()
            })
        })
        .unwrap_or_default();
    let source = clean(
        parsed
            .flags
            .get("source")
            .cloned()
            .or_else(|| parsed.flags.get("url").cloned())
            .unwrap_or_else(|| "unknown".to_string()),
        1200,
    );
    let query = clean(parsed.flags.get("query").cloned().unwrap_or_default(), 280);

    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("mcp_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "mcp_extraction_contract"
    {
        errors.push("mcp_contract_kind_invalid".to_string());
    }
    if payload.trim().is_empty() {
        errors.push("missing_payload".to_string());
    }

    if !errors.is_empty() {
        return finalize_receipt(json!({
            "ok": false,
            "strict": strict,
            "type": "research_plane_mcp_extract",
            "errors": errors
        }));
    }

    let title = parse_title(&payload);
    let text = strip_tags(&payload);
    let max_summary_chars = contract
        .get("max_summary_chars")
        .and_then(Value::as_u64)
        .unwrap_or(600) as usize;
    let summary = clean(&text, max_summary_chars);
    let links = extract_links(&payload);
    let words = text
        .split_whitespace()
        .map(|w| clean(w, 64).to_ascii_lowercase())
        .filter(|w| w.len() >= 4)
        .collect::<Vec<_>>();
    let mut freq = BTreeMap::<String, u64>::new();
    for token in words {
        *freq.entry(token).or_insert(0) += 1;
    }
    let mut entities_ranked = freq.into_iter().collect::<Vec<_>>();
    entities_ranked.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| left.0.cmp(&right.0))
    });
    let entities = entities_ranked
        .into_iter()
        .take(8)
        .map(|(token, count)| json!({"token": token, "count": count}))
        .collect::<Vec<_>>();

    let artifacts = json!({
        "title": title,
        "summary": summary,
        "links": links,
        "entities": entities
    });
    let out = finalize_receipt(json!({
        "ok": true,
        "strict": strict,
        "type": "research_plane_mcp_extract",
        "lane": "core/layer0/ops",
        "source": source,
        "query": query,
        "artifacts": artifacts,
        "provenance": {
            "source_hash": sha256_hex_str(&payload),
            "artifact_hash": sha256_hex_str(&artifacts.to_string()),
            "contract_path": MCP_CONTRACT_PATH
        },
        "claim_evidence": [
            {
                "id": "V6-RESEARCH-001.4",
                "claim": "mcp_extraction_returns_structured_artifacts_and_provenance_before_model_invocation",
                "evidence": {"source_hash": sha256_hex_str(&payload)}
            }
        ]
    }));
    out
}

fn parse_graph(value: Value) -> BTreeMap<String, Vec<String>> {
    let mut out = BTreeMap::<String, Vec<String>>::new();
    let Some(obj) = value.as_object() else {
        return out;
    };
    for (url, node) in obj {
        let links = node
            .get("links")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(Value::as_str)
            .map(|v| clean(v, 1600))
            .filter(|v| !v.is_empty())
            .collect::<Vec<_>>();
        out.insert(clean(url, 1600), links);
    }
    out
}

fn domain_of(url: &str) -> String {
    if url.starts_with("file://") {
        return "file".to_string();
    }
    clean(
        url.split("://")
            .nth(1)
            .unwrap_or(url)
            .split('/')
            .next()
            .unwrap_or("unknown"),
        120,
    )
    .to_ascii_lowercase()
}

pub fn run_spider(root: &Path, parsed: &ParsedArgs, strict: bool) -> Value {
    let contract = read_json_or(
        root,
        SPIDER_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "rule_spider_contract",
            "default_max_depth": 3,
            "default_max_links": 512
        }),
    );
    let graph_json =
        match parse_json_flag_or_path(root, parsed, "graph-json", "graph-path", json!({})) {
            Ok(v) => v,
            Err(err) => {
                return finalize_receipt(json!({
                    "ok": false,
                    "strict": strict,
                    "type": "research_plane_rule_spider",
                    "errors": [err]
                }));
            }
        };
    let graph = parse_graph(graph_json);
    let seeds = parse_list_flag(parsed, "seed-urls", 1800);
    let allow_rules = parse_list_flag(parsed, "allow-rules", 220);
    let deny_rules = parse_list_flag(parsed, "deny-rules", 220);
    let allowed_domains = parse_list_flag(parsed, "allowed-domains", 220)
        .into_iter()
        .map(|v| v.to_ascii_lowercase())
        .collect::<Vec<_>>();
    let max_depth = parse_u64(
        parsed.flags.get("max-depth"),
        contract
            .get("default_max_depth")
            .and_then(Value::as_u64)
            .unwrap_or(3),
    )
    .clamp(1, 20);
    let max_links = parse_u64(
        parsed.flags.get("max-links"),
        contract
            .get("default_max_links")
            .and_then(Value::as_u64)
            .unwrap_or(512),
    )
    .clamp(1, 50_000);

    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("rule_spider_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "rule_spider_contract"
    {
        errors.push("rule_spider_contract_kind_invalid".to_string());
    }
    if seeds.is_empty() {
        errors.push("missing_seed_urls".to_string());
    }
    if graph.is_empty() {
        errors.push("missing_graph_fixture".to_string());
    }
    if !errors.is_empty() {
        return finalize_receipt(json!({
            "ok": false,
            "strict": strict,
            "type": "research_plane_rule_spider",
            "errors": errors
        }));
    }

    let mut queue = VecDeque::<(String, u64)>::new();
    for seed in seeds {
        queue.push_back((seed, 0));
    }
    let mut visited = BTreeSet::<String>::new();
    let mut per_link = Vec::<Value>::new();

    while let Some((url, depth)) = queue.pop_front() {
        if visited.len() as u64 >= max_links {
            break;
        }
        if depth > max_depth || visited.contains(&url) {
            continue;
        }
        visited.insert(url.clone());
        let links = graph.get(&url).cloned().unwrap_or_default();
        for next in links {
            let next_domain = domain_of(&next);
            let denied = deny_rules.iter().any(|rule| pattern_match(&next, rule));
            let allow_match = if allow_rules.is_empty() {
                true
            } else {
                allow_rules.iter().any(|rule| pattern_match(&next, rule))
            };
            let domain_allowed = if allowed_domains.is_empty() {
                true
            } else {
                allowed_domains.iter().any(|d| d == &next_domain)
            };
            let decision = !denied && allow_match && domain_allowed && depth < max_depth;
            let reason = if denied {
                "deny_rule"
            } else if !allow_match {
                "allow_rule_miss"
            } else if !domain_allowed {
                "domain_not_allowed"
            } else if depth >= max_depth {
                "max_depth_reached"
            } else {
                "accepted"
            };
            per_link.push(json!({
                "from": url,
                "to": next,
                "depth": depth.saturating_add(1),
                "decision": if decision { "enqueue" } else { "drop" },
                "reason": reason
            }));
            if decision {
                queue.push_back((next, depth.saturating_add(1)));
            }
        }
    }

    let out = finalize_receipt(json!({
        "ok": true,
        "strict": strict,
        "type": "research_plane_rule_spider",
        "lane": "core/layer0/ops",
        "visited_count": visited.len(),
        "visited": visited,
        "per_link_receipts": per_link,
        "limits": {"max_depth": max_depth, "max_links": max_links},
        "claim_evidence": [
            {
                "id": "V6-RESEARCH-002.1",
                "claim": "rule_based_spider_enforces_allow_deny_depth_domain_with_per_link_receipts",
                "evidence": {"visited_count": visited.len()}
            }
        ]
    }));
    out
}

pub fn run_middleware(root: &Path, parsed: &ParsedArgs, strict: bool) -> Value {
    let contract = read_json_or(
        root,
        MIDDLEWARE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "middleware_stack_contract",
            "ordered_hooks": ["before_request","after_response"]
        }),
    );
    let request_json = parse_json_flag_or_path(
        root,
        parsed,
        "request-json",
        "request-path",
        json!({"url":"https://example.com","headers":{}}),
    );
    let response_json = parse_json_flag_or_path(
        root,
        parsed,
        "response-json",
        "response-path",
        json!({"status":200,"body":"<html></html>"}),
    );
    let stack_json = parse_json_flag_or_path(
        root,
        parsed,
        "stack-json",
        "stack-path",
        json!([
            {"id":"ua_injector","hook":"before_request","set_header":{"User-Agent":"InfRing/1.0"}},
            {"id":"html_compact","hook":"after_response","compact_body":true}
        ]),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("middleware_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "middleware_stack_contract"
    {
        errors.push("middleware_contract_kind_invalid".to_string());
    }
    let mut request = request_json.unwrap_or_else(|err| {
        errors.push(err);
        json!({})
    });
    let mut response = response_json.unwrap_or_else(|err| {
        errors.push(err);
        json!({})
    });
    let stack = stack_json.unwrap_or_else(|err| {
        errors.push(err);
        json!([])
    });
    if !errors.is_empty() {
        return finalize_receipt(json!({
            "ok": false,
            "strict": strict,
            "type": "research_plane_middleware",
            "errors": errors
        }));
    }
    let mut lifecycle = Vec::<Value>::new();
    for row in stack.as_array().cloned().unwrap_or_default() {
        let id = row
            .get("id")
            .and_then(Value::as_str)
            .map(|v| clean(v, 120))
            .unwrap_or_else(|| "unnamed".to_string());
        let hook = row
            .get("hook")
            .and_then(Value::as_str)
            .map(|v| clean(v, 64).to_ascii_lowercase())
            .unwrap_or_else(|| "before_request".to_string());
        if hook == "before_request" {
            if let Some(set_header) = row.get("set_header").and_then(Value::as_object) {
                if !request
                    .get("headers")
                    .map(Value::is_object)
                    .unwrap_or(false)
                {
                    request["headers"] = Value::Object(Map::new());
                }
                for (k, v) in set_header {
                    request["headers"][clean(k, 120)] = Value::String(clean(v.to_string(), 240));
                }
            }
        } else if hook == "after_response" {
            let compact_from_str = row
                .get("compact_body")
                .and_then(Value::as_str)
                .map(|v| {
                    matches!(
                        v.trim().to_ascii_lowercase().as_str(),
                        "1" | "true" | "yes" | "on"
                    )
                })
                .unwrap_or(false);
            let compact_from_bool = row
                .get("compact_body")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            if compact_from_str || compact_from_bool {
                let compact = response
                    .get("body")
                    .and_then(Value::as_str)
                    .map(strip_tags)
                    .unwrap_or_default();
                response["body_compact"] = Value::String(clean(compact, 4000));
            }
        }
        lifecycle.push(json!({
            "middleware_id": id,
            "hook": hook,
            "ts": now_iso()
        }));
    }

    let out = finalize_receipt(json!({
        "ok": true,
        "strict": strict,
        "type": "research_plane_middleware",
        "lane": "core/layer0/ops",
        "request": request,
        "response": response,
        "lifecycle_receipts": lifecycle,
        "claim_evidence": [
            {
                "id": "V6-RESEARCH-002.2",
                "claim": "ordered_downloader_and_spider_middleware_hooks_emit_deterministic_lifecycle_receipts",
                "evidence": {"hooks": lifecycle.len()}
            }
        ]
    }));
    out
}
