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
        "checking your browser before accessing",
        "cf-challenge",
        "cloudflare ray id",
        "just a moment...",
    ]
    .iter()
    .any(|marker| lowered.contains(marker))
}

fn contains_web_junk_marker(text: &str) -> bool {
    let lowered = clean_text(text, 4_000).to_ascii_lowercase();
    if lowered.is_empty() {
        return false;
    }
    if contains_antibot_marker(&lowered) {
        return true;
    }
    [
        "please enable javascript",
        "enable javascript and cookies",
        "access denied",
        "403 forbidden",
        "login required",
        "subscribe to continue",
        "please log in to continue",
        "this content is not available in your region",
        "we use cookies to improve your experience",
        "manage your cookie preferences",
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

fn provider_result_identity(value: &Value) -> String {
    let provider = clean_text(
        value.get("provider").and_then(Value::as_str).unwrap_or(""),
        120,
    )
    .to_ascii_lowercase();
    let status = clean_text(value.get("status").and_then(Value::as_str).unwrap_or(""), 80)
        .to_ascii_lowercase();
    let error = clean_text(value.get("error").and_then(Value::as_str).unwrap_or(""), 160)
        .to_ascii_lowercase();
    if !provider.is_empty() && !error.is_empty() {
        return crate::deterministic_receipt_hash(&json!({
            "kind": "provider_error",
            "provider": provider,
            "status": status,
            "error": error
        }));
    }

    let content = value
        .get("content_preview")
        .or_else(|| value.get("summary"))
        .and_then(Value::as_str)
        .map(|raw| clean_text(raw, 1_200).to_ascii_lowercase())
        .unwrap_or_default();
    if content.split_whitespace().count() >= 8 {
        return crate::deterministic_receipt_hash(&json!({
            "kind": "provider_content",
            "provider": provider,
            "status": status,
            "content": content
        }));
    }

    let stage = clean_text(value.get("stage").and_then(Value::as_str).unwrap_or(""), 120)
        .to_ascii_lowercase();
    let locator = clean_text(
        value.get("locator").and_then(Value::as_str).unwrap_or(""),
        600,
    )
    .to_ascii_lowercase();
    crate::deterministic_receipt_hash(&json!({
        "kind": "provider_locator",
        "provider": provider,
        "stage": stage,
        "status": status,
        "locator": locator
    }))
}

fn dedup_provider_results(provider_results: Vec<Value>) -> (Vec<Value>, usize) {
    let before = provider_results.len();
    let mut seen = HashSet::<String>::new();
    let mut deduped = Vec::<Value>::new();
    for value in provider_results {
        let key = provider_result_identity(&value);
        if seen.insert(key) {
            deduped.push(value);
        }
    }
    let removed = before.saturating_sub(deduped.len());
    (deduped, removed)
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
        Some(cleaned)
    }
}

fn max_explicit_queries_for_budget(primary_query: &str, budget: ApertureBudget) -> usize {
    let _ = primary_query;
    budget.max_candidates.clamp(2, 12)
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
    let normalized_primary = clean_text(primary_query, 600);
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

fn broad_current_research_recovery_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/query_recovery/broad_current_research/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn broad_current_research_recovery_max_queries(policy: &Value, budget: ApertureBudget) -> usize {
    policy
        .pointer("/batch_query/query_recovery/broad_current_research/max_queries")
        .and_then(Value::as_u64)
        .unwrap_or(4)
        .clamp(1, max_explicit_queries_for_budget("", budget) as u64) as usize
}

fn broad_current_research_recovery_templates(policy: &Value) -> Vec<String> {
    let configured = policy
        .pointer("/batch_query/query_recovery/broad_current_research/templates")
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, 600))
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    if configured.is_empty() {
        vec![
            "{query}".to_string(),
            "{query} source-backed overview".to_string(),
            "{query} primary sources".to_string(),
            "{query} official sources".to_string(),
            "{query} recent publications".to_string(),
            "{query} institution announcements".to_string(),
        ]
    } else {
        configured
    }
}

fn query_looks_like_broad_current_research(query: &str) -> bool {
    let cleaned = clean_text(query, 600);
    if cleaned.is_empty() || !current_web_intent(&cleaned) {
        return false;
    }
    let lowered = cleaned.to_ascii_lowercase();
    if lowered.contains("http://")
        || lowered.contains("https://")
        || lowered.contains('"')
        || lowered.contains('`')
    {
        return false;
    }
    let word_count = cleaned.split_whitespace().count();
    let broad_marker = [
        "what are",
        "what were",
        "some ",
        "overview",
        "landscape",
        "trend",
        "trends",
        "changes",
        "developments",
        "breakthrough",
        "breakthroughs",
        "news",
        "current state",
        "state of",
    ]
    .iter()
    .any(|marker| lowered.contains(marker));
    broad_marker || word_count <= 8
}

fn expand_query_recovery_template(template: &str, query: &str) -> Option<String> {
    let year = current_year();
    if template.contains("{current_year}") && query.to_ascii_lowercase().contains(&year) {
        return None;
    }
    let expanded = template
        .replace("{query}", query)
        .replace("{current_year}", &year);
    let cleaned = clean_text(&expanded, 600);
    if cleaned.is_empty() {
        None
    } else {
        Some(cleaned)
    }
}

fn broad_current_research_recovery_queries(
    policy: &Value,
    query: &str,
    budget: ApertureBudget,
) -> Vec<String> {
    if !broad_current_research_recovery_enabled(policy)
        || !query_looks_like_broad_current_research(query)
    {
        return Vec::new();
    }
    let max_queries = broad_current_research_recovery_max_queries(policy, budget);
    let mut dedup = HashSet::<String>::new();
    let mut queries = Vec::<String>::new();
    for template in broad_current_research_recovery_templates(policy) {
        if queries.len() >= max_queries {
            break;
        }
        if let Some(value) = expand_query_recovery_template(&template, query) {
            let key = value.to_ascii_lowercase();
            if dedup.insert(key) {
                queries.push(value);
            }
        }
    }
    if queries.len() <= 1 {
        Vec::new()
    } else {
        queries
    }
}

fn quality_gate_enabled(policy: &Value) -> bool {
    policy
        .pointer("/batch_query/quality_gate/enabled")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

fn provider_recovery_enabled(policy: &Value) -> bool {
    quality_gate_enabled(policy)
        && policy
            .pointer("/batch_query/quality_gate/provider_recovery/enabled")
            .and_then(Value::as_bool)
            .unwrap_or(false)
}

fn provider_recovery_max_providers(policy: &Value) -> usize {
    policy
        .pointer("/batch_query/quality_gate/provider_recovery/max_providers")
        .and_then(Value::as_u64)
        .unwrap_or(1)
        .clamp(1, 6) as usize
}

fn provider_recovery_list(policy: &Value, pointer: &str) -> Vec<String> {
    policy
        .pointer(pointer)
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .filter_map(Value::as_str)
                .map(|row| clean_text(row, 80).to_ascii_lowercase())
                .filter(|row| !row.is_empty())
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn provider_recovery_providers(policy: &Value, query: &str) -> Vec<String> {
    if !provider_recovery_enabled(policy) {
        return Vec::new();
    }
    let mut dedup = HashSet::<String>::new();
    let mut providers = Vec::<String>::new();
    let mut push_provider = |provider: String| {
        if provider.is_empty() {
            return;
        }
        if dedup.insert(provider.clone()) {
            providers.push(provider);
        }
    };
    if current_web_intent(query) {
        for provider in provider_recovery_list(
            policy,
            "/batch_query/quality_gate/provider_recovery/current_intent_providers",
        ) {
            push_provider(provider);
        }
    }
    for provider in provider_recovery_list(
        policy,
        "/batch_query/quality_gate/provider_recovery/providers",
    ) {
        push_provider(provider);
    }
    providers.truncate(provider_recovery_max_providers(policy));
    providers
}

fn resolve_query_plan(
    policy: &Value,
    request: &Value,
    query: &str,
    budget: ApertureBudget,
) -> QueryPlanSelection {
    let explicit_queries = normalize_requested_queries(request, query, budget);
    let explicit_query_pack_used = !explicit_queries.is_empty()
        && (query.is_empty()
            || explicit_queries.len() > 1
            || explicit_queries
                .first()
                .map(|value| !value.eq_ignore_ascii_case(query))
                .unwrap_or(false));
    if explicit_query_pack_used {
        let rerank_query = clean_text(
            explicit_queries.first().map(String::as_str).unwrap_or(query),
            600,
        );
        let rewrite_set = explicit_queries.iter().skip(1).cloned().collect::<Vec<_>>();
        return QueryPlanSelection {
            rewrite_applied: explicit_queries.len() > 1,
            queries: explicit_queries,
            rewrite_set,
            rerank_query,
            query_plan_source: "explicit_request_pack",
        };
    }
    let recovery_queries = broad_current_research_recovery_queries(policy, query, budget);
    if !recovery_queries.is_empty() {
        let rerank_query = recovery_queries
            .first()
            .cloned()
            .unwrap_or_else(|| clean_text(query, 600));
        let rewrite_set = recovery_queries.iter().skip(1).cloned().collect::<Vec<_>>();
        return QueryPlanSelection {
            rewrite_applied: recovery_queries.len() > 1,
            queries: recovery_queries,
            rewrite_set,
            rerank_query,
            query_plan_source: "policy_broad_current_research_recovery",
        };
    }
    let queries = cache_identity_query_plan(query, &explicit_queries);
    let rerank_query = queries
        .first()
        .cloned()
        .unwrap_or_else(|| clean_text(query, 600));
    QueryPlanSelection {
        queries,
        rewrite_set: Vec::new(),
        rewrite_applied: false,
        rerank_query,
        query_plan_source: "agent_submitted_single_query",
    }
}
