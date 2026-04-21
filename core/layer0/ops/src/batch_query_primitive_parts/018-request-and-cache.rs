const CACHE_REL: &str = "client/runtime/local/state/batch_query/cache.json";
const CACHE_MAX_ENTRIES: usize = 240;
const CACHE_TTL_SUCCESS_SECS: i64 = 30 * 60;
const CACHE_TTL_NO_RESULTS_SECS: i64 = 2 * 60;

#[derive(Clone, Debug)]
struct QueryPlanSelection {
    queries: Vec<String>,
    rewrite_set: Vec<String>,
    rewrite_applied: bool,
    rerank_query: String,
    query_plan_source: &'static str,
}

fn contains_antibot_marker(text: &str) -> bool {
    let lowered = clean_text(text, 4_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    [
        "unfortunately, bots use duckduckgo too",
        "please complete the following challenge",
        "select all squares containing",
        "error-lite@duckduckgo.com",
        "anomaly-modal",
        "captcha",
        "verify you are human",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn looks_like_internal_route_query(query: &str) -> bool {
    let lowered = clean_text(query, 600).to_ascii_lowercase();
    lowered.contains("tool::")
        || lowered.contains("map `tool::")
        || lowered.contains("supported route")
        || lowered.contains("command-to-route")
}

fn looks_like_domain_list_noise(text: &str) -> bool {
    let cleaned = clean_text(text, 1_600);
    if cleaned.is_empty() {
        return false;
    }
    let domains = extract_domains_from_text(&cleaned, 16);
    if domains.len() < 3 {
        return false;
    }
    let words = cleaned.split_whitespace().count();
    words <= (domains.len() * 3 + 10)
}

fn cache_path(root: &Path) -> PathBuf {
    root.join(CACHE_REL)
}

fn cache_key(source: &str, query: &str, aperture: &str, policy: &Value) -> String {
    crate::deterministic_receipt_hash(&json!({
        "version": 1,
        "source": source,
        "query": query,
        "aperture": aperture,
        "policy": policy.get("batch_query").cloned().unwrap_or(Value::Null),
    }))
}

fn cache_key_with_query_plan(
    source: &str,
    query: &str,
    aperture: &str,
    policy: &Value,
    query_plan: &[String],
) -> String {
    let normalized_plan = cache_identity_query_plan(query, query_plan);
    if normalized_plan.len() <= 1 {
        return cache_key(source, query, aperture, policy);
    }
    crate::deterministic_receipt_hash(&json!({
        "version": 3,
        "source": source,
        "query": query,
        "aperture": aperture,
        "query_plan": normalized_plan,
        "policy": policy.get("batch_query").cloned().unwrap_or(Value::Null),
    }))
}

fn cache_identity_query_plan(query: &str, query_plan: &[String]) -> Vec<String> {
    let mut dedup = HashSet::<String>::new();
    let mut normalized = Vec::<String>::new();
    for value in query_plan {
        let cleaned = clean_text(value, 600);
        if cleaned.is_empty() {
            continue;
        }
        let key = cleaned.to_ascii_lowercase();
        if dedup.insert(key) {
            normalized.push(cleaned);
        }
    }
    if normalized.is_empty() {
        let cleaned_query = clean_text(query, 600);
        if !cleaned_query.is_empty() {
            normalized.push(cleaned_query);
        }
    }
    normalized
}

fn cache_ttl_for_status(status: &str) -> i64 {
    if status == "ok" || status == "partial" {
        CACHE_TTL_SUCCESS_SECS
    } else {
        CACHE_TTL_NO_RESULTS_SECS
    }
}

fn load_cached_response(root: &Path, key: &str) -> Option<Value> {
    let path = cache_path(root);
    let mut cache = read_json_or(&path, json!({"version": 1, "entries": {}}));
    let now_ts = chrono::Utc::now().timestamp();
    let mut mutated = false;
    let mut hit = None::<Value>;
    if let Some(entries) = cache.get_mut("entries").and_then(Value::as_object_mut) {
        let stale_keys = entries
            .iter()
            .filter_map(|(entry_key, entry)| {
                let expires_at = entry.get("expires_at").and_then(Value::as_i64).unwrap_or(0);
                if expires_at <= now_ts {
                    Some(entry_key.clone())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>();
        for stale_key in stale_keys {
            entries.remove(&stale_key);
            mutated = true;
        }
        if let Some(entry) = entries.get(key) {
            if let Some(response) = entry.get("response") {
                hit = Some(response.clone());
            }
        }
    }
    if mutated {
        let _ = write_json_atomic(&path, &cache);
    }
    hit
}

fn store_cached_response(root: &Path, key: &str, response: &Value, status: &str) {
    let path = cache_path(root);
    let mut cache = read_json_or(&path, json!({"version": 1, "entries": {}}));
    let now_ts = chrono::Utc::now().timestamp();
    let mut entries = cache
        .get("entries")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    entries
        .retain(|_, entry| entry.get("expires_at").and_then(Value::as_i64).unwrap_or(0) > now_ts);
    let ttl = cache_ttl_for_status(status).max(30);
    entries.insert(
        key.to_string(),
        json!({
            "stored_at": now_ts,
            "expires_at": now_ts + ttl,
            "status": status,
            "response": response
        }),
    );
    if entries.len() > CACHE_MAX_ENTRIES {
        let mut order = entries
            .iter()
            .map(|(entry_key, entry)| {
                (
                    entry_key.clone(),
                    entry.get("stored_at").and_then(Value::as_i64).unwrap_or(0),
                )
            })
            .collect::<Vec<_>>();
        order.sort_by_key(|(_, stored_at)| *stored_at);
        let drop_count = entries.len().saturating_sub(CACHE_MAX_ENTRIES);
        for (entry_key, _) in order.into_iter().take(drop_count) {
            entries.remove(&entry_key);
        }
    }
    cache["version"] = json!(1);
    cache["entries"] = Value::Object(entries);
    let _ = write_json_atomic(&path, &cache);
}

fn request_query_text(request: &Value, max_len: usize) -> String {
    let direct = clean_text(
        request
            .get("query")
            .or_else(|| request.get("q"))
            .and_then(Value::as_str)
            .unwrap_or(""),
        max_len,
    );
    if !direct.is_empty() {
        return direct;
    }
    request
        .get("queries")
        .and_then(Value::as_array)
        .and_then(|rows| rows.iter().find_map(|row| extract_request_query_row(row, max_len)))
        .unwrap_or_default()
}

fn extract_request_query_row(row: &Value, max_len: usize) -> Option<String> {
    let raw = if let Some(value) = row.as_str() {
        Some(value)
    } else {
        row.get("query")
            .or_else(|| row.get("q"))
            .and_then(Value::as_str)
    }?;
    let cleaned = clean_text(raw, max_len);
    if cleaned.is_empty() {
        None
    } else {
        Some(resolve_deictic_framework_reference(&cleaned))
    }
}

fn max_explicit_queries_for_budget(primary_query: &str, budget: ApertureBudget) -> usize {
    if is_framework_catalog_intent(primary_query) && budget.max_query_rewrites > 0 {
        return 8;
    }
    budget.max_evidence.clamp(2, 6)
}

fn derived_framework_catalog_queries(query: &str, budget: ApertureBudget) -> Option<Vec<String>> {
    if !is_framework_catalog_intent(query) || budget.max_query_rewrites == 0 {
        return None;
    }
    let mut dedup = HashSet::<String>::new();
    let mut queries = Vec::<String>::new();
    let max_queries = max_explicit_queries_for_budget(query, budget);
    let push_query =
        |value: &str, dedup: &mut HashSet<String>, queries: &mut Vec<String>| {
            let cleaned = resolve_deictic_framework_reference(&clean_text(value, 600));
            if cleaned.is_empty() {
                return;
            }
            let key = cleaned.to_ascii_lowercase();
            if dedup.insert(key) {
                queries.push(cleaned);
            }
        };
    push_query(query, &mut dedup, &mut queries);
    for value in [
        "AI agent frameworks landscape LangGraph OpenAI Agents SDK AutoGen CrewAI smolagents",
        "site:langchain.com LangGraph agent framework overview",
        "site:openai.github.io/openai-agents-python OpenAI Agents SDK overview",
        "site:microsoft.github.io AutoGen framework overview",
        "site:crewai.com CrewAI agent framework overview",
        "site:github.com huggingface/smolagents smolagents framework overview",
        "OpenAI Agents SDK official docs overview",
    ] {
        if queries.len() >= max_queries {
            break;
        }
        push_query(value, &mut dedup, &mut queries);
    }
    (queries.len() > 1).then_some(queries)
}

fn normalize_requested_queries(
    request: &Value,
    primary_query: &str,
    budget: ApertureBudget,
) -> Vec<String> {
    let mut dedup = HashSet::<String>::new();
    let mut queries = Vec::<String>::new();
    let push_query =
        |value: String, dedup: &mut HashSet<String>, queries: &mut Vec<String>| {
            if value.is_empty() {
                return;
            }
            let key = value.to_ascii_lowercase();
            if dedup.insert(key) {
                queries.push(value);
            }
        };
    let normalized_primary = resolve_deictic_framework_reference(&clean_text(primary_query, 600));
    if !normalized_primary.is_empty() {
        push_query(normalized_primary, &mut dedup, &mut queries);
    }
    let max_queries = max_explicit_queries_for_budget(primary_query, budget);
    if let Some(rows) = request.get("queries").and_then(Value::as_array) {
        for row in rows {
            if queries.len() >= max_queries {
                break;
            }
            if let Some(value) = extract_request_query_row(row, 600) {
                push_query(value, &mut dedup, &mut queries);
            }
        }
    }
    queries
}

fn resolve_query_plan(request: &Value, query: &str, budget: ApertureBudget) -> QueryPlanSelection {
    let benchmark_instructional_rerank = if is_benchmark_or_comparison_intent(query) {
        normalize_instructional_query(query).unwrap_or_default()
    } else {
        String::new()
    };
    let explicit_queries = normalize_requested_queries(request, query, budget);
    let explicit_query_pack_used = !explicit_queries.is_empty()
        && (query.is_empty()
            || explicit_queries.len() > 1
            || explicit_queries
                .first()
                .map(|value| !value.eq_ignore_ascii_case(query))
                .unwrap_or(false));
    if explicit_query_pack_used {
        let rerank_query = if benchmark_instructional_rerank.is_empty() {
            clean_text(
                explicit_queries.first().map(String::as_str).unwrap_or(query),
                600,
            )
        } else {
            clean_text(&benchmark_instructional_rerank, 600)
        };
        let rewrite_set = explicit_queries.iter().skip(1).cloned().collect::<Vec<_>>();
        return QueryPlanSelection {
            rewrite_applied: explicit_queries.len() > 1,
            queries: explicit_queries,
            rewrite_set,
            rerank_query,
            query_plan_source: if benchmark_instructional_rerank.is_empty() {
                "explicit_request_pack"
            } else {
                "explicit_request_pack_instructional_rerank"
            },
        };
    }
    if let Some(queries) = derived_framework_catalog_queries(query, budget) {
        let rewrite_set = queries.iter().skip(1).cloned().collect::<Vec<_>>();
        return QueryPlanSelection {
            rerank_query: if benchmark_instructional_rerank.is_empty() {
                clean_text(query, 600)
            } else {
                clean_text(&benchmark_instructional_rerank, 600)
            },
            rewrite_applied: true,
            queries,
            rewrite_set,
            query_plan_source: "derived_rewrite",
        };
    }
    let (queries, rewrite_set, rewrite_applied) = build_query_plan(query, budget);
    let rerank_query = if !benchmark_instructional_rerank.is_empty() {
        clean_text(&benchmark_instructional_rerank, 600)
    } else if rewrite_applied {
        queries
            .last()
            .cloned()
            .unwrap_or_else(|| clean_text(query, 600))
    } else {
        clean_text(query, 600)
    };
    QueryPlanSelection {
        queries,
        rewrite_set,
        rewrite_applied,
        rerank_query,
        query_plan_source: if !benchmark_instructional_rerank.is_empty() {
            "instructional_rerank_focus"
        } else if rewrite_applied {
            "derived_rewrite"
        } else {
            "single_query"
        },
    }
}
