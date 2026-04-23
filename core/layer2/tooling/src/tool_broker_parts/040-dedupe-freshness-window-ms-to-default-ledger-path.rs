
fn dedupe_freshness_window_ms(tool_name: &str, requested: Option<u64>) -> u64 {
    let default = if matches!(tool_name, "web_search" | "web_fetch" | "batch_query") {
        30_000
    } else {
        0
    };
    requested.unwrap_or(default).min(86_400_000)
}

fn sanitize_lineage(lineage: &[String]) -> Vec<String> {
    let mut rows = lineage
        .iter()
        .map(|v| clean_text(v, 200))
        .filter(|v| !v.is_empty())
        .collect::<Vec<_>>();
    if rows.is_empty() {
        rows.push("tool_broker_v1".to_string());
    }
    rows
}

#[cfg(test)]
fn default_ledger_path() -> PathBuf {
    if let Some(path) = std::env::var("INFRING_TOOL_BROKER_LEDGER_PATH")
        .ok()
        .map(|v| PathBuf::from(clean_text(&v, 400)))
        .filter(|v| !v.as_os_str().is_empty())
    {
        return path;
    }
    std::env::temp_dir().join(format!(
        "infring_tool_broker_test_{}_{}.jsonl",
        std::process::id(),
        now_ms()
    ))
}

#[cfg(not(test))]
fn default_ledger_path() -> PathBuf {
    if let Some(path) = std::env::var("INFRING_TOOL_BROKER_LEDGER_PATH")
        .ok()
        .map(|v| PathBuf::from(clean_text(&v, 400)))
        .filter(|v| !v.as_os_str().is_empty())
    {
        return path;
    }
    if let Some(root) = std::env::var("INFRING_ROOT")
        .ok()
        .or_else(|| std::env::var("INFRING_ROOT").ok())
        .map(|v| PathBuf::from(clean_text(&v, 400)))
        .filter(|v| !v.as_os_str().is_empty())
    {
        return root
            .join("core")
            .join("local")
            .join("state")
            .join("tooling")
            .join("tool_broker_events.jsonl");
    }
    std::env::temp_dir()
        .join("infring")
        .join("tool_broker_events.jsonl")
}
