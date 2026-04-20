
pub fn parse_args(argv: &[String]) -> Options {
    let mut out = Options {
        json: parse_bool_flag(std::env::var("PROTHEUS_GLOBAL_JSON").ok().as_deref(), false),
        prewarm: true,
        allow_local_simulation: parse_bool_flag(
            std::env::var("INFRING_ALLOW_LOCAL_SIMULATION")
                .ok()
                .or_else(|| std::env::var("PROTHEUS_ALLOW_LOCAL_SIMULATION").ok())
                .as_deref(),
            false,
        ),
        plan_only: false,
        ..Options::default()
    };
    for token in argv {
        let trimmed = token.trim();
        if trimmed == "--help" || trimmed == "-h" {
            out.help = true;
            continue;
        }
        if let Some(raw) = trimmed.strip_prefix("--showcase=") {
            out.showcase = parse_bool_flag(Some(raw), false);
            continue;
        }
        if trimmed == "--showcase" {
            out.showcase = true;
            continue;
        }
        if let Some(raw) = trimmed.strip_prefix("--scaffold-payload=") {
            out.scaffold_payload = parse_bool_flag(Some(raw), false);
            continue;
        }
        if trimmed == "--scaffold-payload" {
            out.scaffold_payload = true;
            continue;
        }
        if trimmed == "--no-prewarm" {
            out.prewarm = false;
            continue;
        }
        if let Some(raw) = trimmed.strip_prefix("--prewarm=") {
            out.prewarm = parse_bool_flag(Some(raw), true);
            continue;
        }
        if let Some(raw) = trimmed.strip_prefix("--allow-local-simulation=") {
            out.allow_local_simulation = parse_bool_flag(Some(raw), out.allow_local_simulation);
            continue;
        }
        if trimmed == "--allow-local-simulation" {
            out.allow_local_simulation = true;
            continue;
        }
        if let Some(raw) = trimmed.strip_prefix("--plan-only=") {
            out.plan_only = parse_bool_flag(Some(raw), out.plan_only);
            continue;
        }
        if trimmed == "--plan-only" {
            out.plan_only = true;
            continue;
        }
        if let Some(raw) = trimmed.strip_prefix("--strict=") {
            out.strict = parse_bool_flag(Some(raw), out.strict);
            continue;
        }
        if trimmed == "--strict" {
            out.strict = true;
            continue;
        }
        if let Some(raw) = trimmed.strip_prefix("--hard-selector=") {
            out.hard_selector = raw.trim().to_string();
            continue;
        }
        if let Some(raw) = trimmed.strip_prefix("--selector-bypass=") {
            out.selector_bypass = parse_bool_flag(Some(raw), out.selector_bypass);
            continue;
        }
        if trimmed == "--selector-bypass" {
            out.selector_bypass = true;
            continue;
        }
        if let Some(raw) = trimmed.strip_prefix("--duration-ms=") {
            if let Ok(parsed) = raw.parse::<u64>() {
                out.duration_ms = Some(parsed);
            }
            continue;
        }
        if let Some(raw) = trimmed.strip_prefix("--json=") {
            out.json = parse_bool_flag(Some(raw), out.json);
            continue;
        }
        if let Some(raw) = trimmed.strip_prefix("--core-domain=") {
            out.core_domain = raw.trim().to_string();
            continue;
        }
        if let Some(raw) = trimmed.strip_prefix("--core-args-base64=") {
            out.core_args_base64 = raw.trim().to_string();
            continue;
        }
        if let Some(raw) = trimmed.strip_prefix("--target=") {
            out.target = raw.trim().to_string();
            continue;
        }
        if !trimmed.starts_with("--") && out.target.is_empty() {
            out.target = trimmed.to_string();
        }
    }
    out.target = normalize_target(&out.target);
    out
}

pub fn build_receipt_hash(target: &str, ts_iso: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("{target}|assimilation|{ts_iso}").as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

pub fn decode_injected_route(options: &Options) -> Result<Option<Route>, String> {
    let domain = options.core_domain.trim();
    if domain.is_empty() {
        return Ok(None);
    }
    let raw_b64 = options.core_args_base64.trim();
    if raw_b64.is_empty() {
        return Err("core-args-base64 is required when core-domain is provided".to_string());
    }
    let decoded = BASE64_STANDARD
        .decode(raw_b64.as_bytes())
        .map_err(|_| "invalid core route payload".to_string())?;
    let text = String::from_utf8(decoded).map_err(|_| "invalid core route payload".to_string())?;
    let rows = serde_json::from_str::<Vec<String>>(&text)
        .map_err(|_| "core route args must be a string array".to_string())?;
    Ok(Some(Route {
        domain: domain.to_string(),
        args: rows,
    }))
}

pub fn payload_scaffold_for(target: &str) -> Value {
    let normalized = target.trim().to_ascii_lowercase();
    match normalized.as_str() {
        "haystack" | "workflow://haystack" | "rag://haystack" => json!({
            "name": "example-haystack-pipeline",
            "components": [{
                "id": "retriever",
                "stage_type": "retriever",
                "input_type": "text",
                "output_type": "docs",
                "parallel": false,
                "spawn": false,
                "budget": 128
            }]
        }),
        "workflow_chain" | "workflow://workflow_chain" | "chains://workflow_chain" => {
            json!({"name":"workflow_chain-integration","integration_type":"tool","capabilities":["retrieve"]})
        }
        "dspy" | "workflow://dspy" | "optimizer://dspy" => {
            json!({"name":"dspy-integration","kind":"retriever","capabilities":["retrieve"]})
        }
        "pydantic-ai" | "workflow://pydantic-ai" | "agents://pydantic-ai" => {
            json!({"name":"pydantic-agent","model":"gpt-4o-mini","tools":[]})
        }
        "camel" | "workflow://camel" | "society://camel" => {
            json!({"name":"camel-dataset","dataset":{"rows":[]}})
        }
        "llamaindex" | "rag://llamaindex" => {
            json!({"name":"llamaindex-connector","connector_type":"filesystem","root_path":"./docs"})
        }
        "google-adk" | "workflow://google-adk" => {
            json!({"name":"google-adk-tool-manifest","tools":[]})
        }
        "mastra" | "workflow://mastra" => json!({"name":"mastra-graph","nodes":[],"edges":[]}),
        "shannon" | "workflow://shannon" => json!({"profile":"rich","task":"assimilate"}),
        _ => json!({
            "target": if normalized.is_empty() { "unknown" } else { &normalized },
            "hint": "No specialized scaffold exists for this target. Use --payload-base64 with target-specific JSON."
        }),
    }
}

fn normalized_path_text(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn should_skip_scan_path(path: &Path) -> bool {
    let normalized = normalized_path_text(path).to_ascii_lowercase();
    normalized.contains("/.git/")
        || normalized.ends_with("/.git")
        || normalized.contains("/node_modules/")
        || normalized.ends_with("/node_modules")
        || normalized.contains("/target/")
        || normalized.ends_with("/target")
        || normalized.contains("/dist/")
        || normalized.ends_with("/dist")
        || normalized.contains("/build/")
        || normalized.ends_with("/build")
        || normalized.contains("/.next/")
        || normalized.ends_with("/.next")
        || normalized.contains("/__pycache__/")
        || normalized.ends_with("/__pycache__")
        || normalized.contains("/.venv/")
        || normalized.ends_with("/.venv")
        || normalized.contains("/venv/")
        || normalized.ends_with("/venv")
        || normalized.contains("/local/state/")
}
