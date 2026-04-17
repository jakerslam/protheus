fn web_tooling_provider_contract_targets() -> [&'static str; 10] {
    [
        "brave",
        "duckduckgo",
        "exa",
        "firecrawl",
        "google",
        "minimax",
        "moonshot",
        "perplexity",
        "tavily",
        "xai",
    ]
}

fn normalize_web_tooling_provider(raw: &str) -> Option<String> {
    let normalized = raw.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return None;
    }
    let canonical = match normalized.as_str() {
        "kimi" | "moonshot" => "moonshot",
        "grok" | "xai" => "xai",
        "duck_duck_go" | "duckduckgo" => "duckduckgo",
        "brave_search" | "brave" => "brave",
        _ => normalized.as_str(),
    };
    Some(canonical.to_string())
}

fn vault_directive_mentions_web_tooling(entry: &Value) -> bool {
    let directive = entry
        .get("directive")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let pattern = entry
        .get("rule_pattern")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let content = format!("{directive} {pattern}");
    ["web", "search", "fetch", "crawl", "citation", "internet"]
        .iter()
        .any(|token| content.contains(token))
}

fn vault_directive_web_tooling_provider_targets(entry: &Value) -> Vec<String> {
    let directive = entry
        .get("directive")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let pattern = entry
        .get("rule_pattern")
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_ascii_lowercase();
    let content = format!("{directive} {pattern}");
    let aliases = [
        "brave",
        "brave_search",
        "duckduckgo",
        "duck_duck_go",
        "exa",
        "firecrawl",
        "google",
        "minimax",
        "moonshot",
        "kimi",
        "perplexity",
        "tavily",
        "xai",
        "grok",
    ];
    let mut targets = aliases
        .iter()
        .filter(|alias| content.contains(*alias))
        .filter_map(|alias| normalize_web_tooling_provider(alias))
        .collect::<Vec<_>>();
    targets.sort();
    targets.dedup();
    targets
}
