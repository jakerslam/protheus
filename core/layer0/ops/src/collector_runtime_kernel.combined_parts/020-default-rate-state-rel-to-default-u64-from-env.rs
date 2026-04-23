
const DEFAULT_RATE_STATE_REL: &str = "local/state/sensory/eyes/collector_rate_state.json";
const RATE_SCHEMA_ID: &str = "collector_rate_state_v1";
const EYES_STATE_DEFAULT_REL: &str = "local/state/sensory/eyes";

fn usage() {
    println!("collector-runtime-kernel commands:");
    println!("  infring-ops collector-runtime-kernel classify-error --payload-base64=<json>");
    println!("  infring-ops collector-runtime-kernel resolve-controls --payload-base64=<json>");
    println!("  infring-ops collector-runtime-kernel begin-collection --payload-base64=<json>");
    println!("  infring-ops collector-runtime-kernel prepare-run --payload-base64=<json>");
    println!("  infring-ops collector-runtime-kernel finalize-run --payload-base64=<json>");
    println!("  infring-ops collector-runtime-kernel fetch-text --payload-base64=<json>");
    println!("  infring-ops collector-runtime-kernel prepare-attempt --payload-base64=<json>");
    println!("  infring-ops collector-runtime-kernel mark-success --payload-base64=<json>");
    println!("  infring-ops collector-runtime-kernel mark-failure --payload-base64=<json>");
}

fn now_ms_u64() -> u64 {
    let ts = chrono::Utc::now().timestamp_millis();
    if ts <= 0 {
        0
    } else {
        ts as u64
    }
}

fn payload_obj<'a>(value: &'a Value) -> &'a Map<String, Value> {
    lane_utils::payload_obj(value)
}

fn json_u64(payload: &Map<String, Value>, key: &str, fallback: u64, lo: u64, hi: u64) -> u64 {
    payload
        .get(key)
        .and_then(Value::as_u64)
        .unwrap_or(fallback)
        .clamp(lo, hi)
}

fn json_bool(payload: &Map<String, Value>, key: &str, fallback: bool) -> bool {
    payload
        .get(key)
        .and_then(Value::as_bool)
        .unwrap_or(fallback)
}

fn json_f64(payload: &Map<String, Value>, key: &str, fallback: f64, lo: f64, hi: f64) -> f64 {
    payload
        .get(key)
        .and_then(Value::as_f64)
        .unwrap_or(fallback)
        .clamp(lo, hi)
}

fn code_matches_any(code: &str, allowed: &[&str]) -> bool {
    let normalized = code.trim().to_ascii_lowercase();
    allowed.iter().any(|candidate| normalized == *candidate)
}

fn is_retryable_code(code: &str) -> bool {
    const RETRYABLE: [&str; 9] = [
        "env_blocked",
        "dns_unreachable",
        "connection_refused",
        "connection_reset",
        "timeout",
        "http_5xx",
        "rate_limited",
        "http_error",
        "collector_error",
    ];
    code_matches_any(code, &RETRYABLE)
}

fn is_transport_failure_code(code: &str) -> bool {
    const TRANSPORT_FAILURES: [&str; 11] = [
        "env_blocked",
        "dns_unreachable",
        "connection_refused",
        "connection_reset",
        "timeout",
        "tls_error",
        "http_4xx",
        "http_404",
        "http_5xx",
        "rate_limited",
        "http_error",
    ];
    code_matches_any(code, &TRANSPORT_FAILURES)
}

fn http_status_to_code(status: u64) -> &'static str {
    lane_utils::http_status_to_code(status)
}

fn normalize_node_code(raw: &str) -> String {
    let c = raw.trim().to_ascii_lowercase();
    if c.is_empty() {
        return String::new();
    }
    match c.as_str() {
        "auth_missing"
        | "auth_unauthorized"
        | "auth_forbidden"
        | "env_blocked"
        | "dns_unreachable"
        | "connection_refused"
        | "connection_reset"
        | "timeout"
        | "tls_error"
        | "rate_limited"
        | "http_4xx"
        | "http_404"
        | "http_5xx"
        | "http_error"
        | "network_error"
        | "endpoint_unsupported"
        | "collector_error" => c,
        "enotfound" | "eai_again" => "dns_unreachable".to_string(),
        "eperm" => "env_blocked".to_string(),
        "econnrefused" => "connection_refused".to_string(),
        "econnreset" => "connection_reset".to_string(),
        "etimedout" | "esockettimedout" => "timeout".to_string(),
        "unauthorized" => "auth_unauthorized".to_string(),
        "forbidden" => "auth_forbidden".to_string(),
        _ => {
            if c.contains("cert") || c.contains("ssl") || c.contains("tls") {
                "tls_error".to_string()
            } else {
                String::new()
            }
        }
    }
}

fn parse_http_status_from_message(msg: &str) -> Option<u64> {
    let lower = msg.to_ascii_lowercase();
    let idx = lower.find("http ")?;
    let rest = lower.get((idx + 5)..)?;
    let digits = rest
        .chars()
        .take_while(|ch| ch.is_ascii_digit())
        .collect::<String>();
    if digits.len() == 3 {
        digits.parse::<u64>().ok()
    } else {
        None
    }
}

fn classify_message(msg: &str) -> String {
    let s = msg.to_ascii_lowercase();
    if s.is_empty() {
        return String::new();
    }
    if s.contains("missing_moltbook_api_key") || s.contains("missing api key") {
        return "auth_missing".to_string();
    }
    if s.contains("unauthorized") {
        return "auth_unauthorized".to_string();
    }
    if s.contains("forbidden") {
        return "auth_forbidden".to_string();
    }
    if s.contains("enotfound")
        || s.contains("getaddrinfo")
        || s.contains("dns")
        || s.contains("eai_again")
    {
        return "dns_unreachable".to_string();
    }
    if s.contains("operation not permitted") || s.contains("permission denied") {
        return "env_blocked".to_string();
    }
    if s.contains("econnrefused") || s.contains("connection refused") {
        return "connection_refused".to_string();
    }
    if s.contains("econnreset") {
        return "connection_reset".to_string();
    }
    if s.contains("timed out") || s.contains("timeout") || s.contains("etimedout") {
        return "timeout".to_string();
    }
    if s.contains("ssl") || s.contains("tls") || s.contains("certificate") {
        return "tls_error".to_string();
    }
    if let Some(status) = parse_http_status_from_message(&s) {
        return http_status_to_code(status).to_string();
    }
    String::new()
}

fn clean_collector_id(payload: &Map<String, Value>) -> String {
    lane_utils::clean_token(
        payload.get("collector_id").and_then(Value::as_str),
        "collector",
    )
}

fn default_u64_from_env(key: &str, fallback: u64, lo: u64, hi: u64) -> u64 {
    std::env::var(key)
        .ok()
        .and_then(|raw| raw.trim().parse::<u64>().ok())
        .unwrap_or(fallback)
        .clamp(lo, hi)
}
