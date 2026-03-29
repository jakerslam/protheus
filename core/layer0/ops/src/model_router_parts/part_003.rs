pub fn normalize_capability_key(value: &str) -> String {
    let src = normalize_key(value);
    if src.is_empty() {
        return String::new();
    }

    let mut sanitized = String::with_capacity(src.len());
    for ch in src.chars() {
        let out = if ch.is_ascii_lowercase()
            || ch.is_ascii_digit()
            || ch == ':'
            || ch == '_'
            || ch == '-'
        {
            ch
        } else {
            '_'
        };
        sanitized.push(out);
    }

    let mut collapsed = String::with_capacity(sanitized.len());
    let mut prev_underscore = false;
    for ch in sanitized.chars() {
        if ch == '_' {
            if prev_underscore {
                continue;
            }
            prev_underscore = true;
        } else {
            prev_underscore = false;
        }
        collapsed.push(ch);
    }

    collapsed
        .trim_matches('_')
        .chars()
        .take(72)
        .collect::<String>()
}

pub fn infer_capability(intent: &str, task: &str, role: &str) -> String {
    let combined = format!("{} {}", intent, task);
    let tokens = tokenize(&combined);

    if has_any_exact(
        &tokens,
        &["edit", "patch", "refactor", "rewrite", "modify", "fix"],
    ) {
        return "file_edit".to_string();
    }
    if has_any_exact(&tokens, &["read", "list", "show", "inspect", "cat"]) {
        return "file_read".to_string();
    }
    if has_any_exact(
        &tokens,
        &[
            "tool",
            "api",
            "curl",
            "exec",
            "command",
            "shell",
            "cli",
            "automation",
        ],
    ) {
        return "tool_use".to_string();
    }
    if has_any_exact(&tokens, &["plan", "roadmap", "strategy", "backlog", "roi"])
        || has_prefix(&tokens, "priorit")
    {
        return "planning".to_string();
    }
    if has_any_exact(
        &tokens,
        &["reply", "respond", "chat", "comment", "summar", "explain"],
    ) {
        return "chat".to_string();
    }

    let role_key = normalize_key(role);
    if role_key.is_empty() {
        "general".to_string()
    } else {
        format!("role:{role_key}")
    }
}

pub fn capability_family_key(capability: &str) -> String {
    let cap = normalize_capability_key(capability);
    if cap.is_empty() {
        return String::new();
    }

    let parts = cap
        .split(':')
        .filter(|row| !row.is_empty())
        .collect::<Vec<_>>();
    if parts.is_empty() {
        return String::new();
    }
    if parts[0] == "proposal" {
        return if parts.len() >= 2 {
            format!("proposal_{}", parts[1])
        } else {
            "proposal".to_string()
        };
    }
    if parts.len() >= 2 {
        return format!("{}_{}", parts[0], parts[1]);
    }
    parts[0].to_string()
}

pub fn task_type_key_from_route(route_class: &str, capability: &str, role: &str) -> String {
    let route_class_key = normalize_key(route_class);
    if !route_class_key.is_empty() && route_class_key != "default" {
        return format!("class:{route_class_key}");
    }

    let capability_family = capability_family_key(capability);
    if !capability_family.is_empty() {
        return format!("cap:{capability_family}");
    }

    let role_key = normalize_key(role);
    if !role_key.is_empty() {
        return format!("role:{role_key}");
    }
    "general".to_string()
}

pub fn normalize_risk_level(value: &str) -> String {
    let risk = normalize_key(value);
    match risk.as_str() {
        "low" | "medium" | "high" => risk,
        _ => "medium".to_string(),
    }
}

pub fn normalize_complexity_level(value: &str) -> String {
    let complexity = normalize_key(value);
    match complexity.as_str() {
        "low" | "medium" | "high" => complexity,
        _ => "medium".to_string(),
    }
}

pub fn pressure_order(value: &str) -> u8 {
    match normalize_key(value).as_str() {
        "critical" => 4,
        "hard" | "high" => 3,
        "soft" | "medium" => 2,
        "low" => 1,
        _ => 0,
    }
}

pub fn normalize_router_pressure(value: &str) -> String {
    match normalize_key(value).as_str() {
        "critical" | "hard" | "high" => "hard".to_string(),
        "soft" | "medium" => "soft".to_string(),
        _ => "none".to_string(),
    }
}

pub fn is_env_probe_blocked_text(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    if lower.is_empty() {
        return false;
    }
    (lower.contains("operation not permitted") && lower.contains("11434"))
        || (lower.contains("permission denied") && lower.contains("11434"))
        || (lower.contains("sandbox") && lower.contains("11434"))
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProbeBlockedNormalization {
    pub rec: Option<Value>,
    pub changed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeHealthStabilizerPolicy {
    pub suppression_enabled: bool,
    pub suppression_timeout_streak: i64,
    pub suppression_minutes: i64,
    pub rehab_success_threshold: i64,
}

impl Default for ProbeHealthStabilizerPolicy {
    fn default() -> Self {
        Self {
            suppression_enabled: true,
            suppression_timeout_streak: ROUTER_PROBE_SUPPRESSION_TIMEOUT_STREAK_DEFAULT,
            suppression_minutes: ROUTER_PROBE_SUPPRESSION_MINUTES_DEFAULT,
            rehab_success_threshold: ROUTER_PROBE_REHAB_SUCCESS_THRESHOLD_DEFAULT,
        }
    }
}

fn clamp_request_tokens(value: i64) -> i64 {
    value.clamp(ROUTER_MIN_REQUEST_TOKENS, ROUTER_MAX_REQUEST_TOKENS)
}

fn to_bool_like_value(value: Option<&Value>, fallback: bool) -> bool {
    match value {
        Some(Value::Bool(v)) => *v,
        Some(Value::Null) | None => fallback,
        Some(Value::String(raw)) => match normalize_key(raw).as_str() {
            "1" | "true" | "yes" | "on" => true,
            "0" | "false" | "no" | "off" => false,
            _ => fallback,
        },
        _ => fallback,
    }
}

fn to_bounded_number_like(value: Option<&Value>, fallback: i64, min: i64, max: i64) -> i64 {
    let number = finite_number(value).unwrap_or(fallback as f64);
    let clamped = number.clamp(min as f64, max as f64);
    clamped as i64
}

fn to_bounded_number_like_f64(value: Option<&Value>, fallback: f64, min: f64, max: f64) -> f64 {
    finite_number(value).unwrap_or(fallback).clamp(min, max)
}

fn string_or(value: Option<&Value>, fallback: &str) -> String {
    value
        .and_then(Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| fallback.to_string())
}

fn string_like(value: Option<&Value>) -> String {
    match value {
        Some(Value::String(v)) => v.clone(),
        Some(Value::Number(v)) => v.to_string(),
        Some(Value::Bool(v)) => {
            if *v {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        _ => String::new(),
    }
}

fn bool_or_one_like(value: Option<&Value>) -> bool {
    match value {
        Some(Value::Bool(v)) => *v,
        Some(Value::Number(v)) => v
            .as_f64()
            .map(|num| num.is_finite() && (num - 1.0).abs() < f64::EPSILON)
            .unwrap_or(false),
        Some(Value::String(v)) => {
            let trimmed = v.trim();
            if trimmed == "1" {
                return true;
            }
            matches!(normalize_key(trimmed).as_str(), "true" | "yes" | "on")
        }
        _ => false,
    }
}

fn value_string_array(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|rows| {
            rows.iter()
                .map(|row| {
                    row.as_str()
                        .map(ToString::to_string)
                        .unwrap_or_else(|| row.to_string().trim_matches('"').to_string())
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default()
}

fn contains_cli_flag(raw_text: &str) -> bool {
    raw_text.split_whitespace().any(|token| {
        let tok = token.trim();
        if tok.len() < 2 || !tok.starts_with('-') {
            return false;
        }
        let tail = tok.trim_start_matches('-');
        if tail.is_empty() {
            return false;
        }
        let mut chars = tail.chars();
        let first = chars.next().unwrap_or_default();
        if !first.is_ascii_lowercase() && !first.is_ascii_digit() {
            return false;
        }
        chars.all(|ch| ch.is_ascii_lowercase() || ch.is_ascii_digit() || ch == '_' || ch == '-')
    })
}

fn contains_shell_or_path_marker(raw_text: &str) -> bool {
    let lower = raw_text.to_ascii_lowercase();
    if lower.contains("~/")
        || lower.contains("../")
        || lower.contains("./")
        || lower.contains("/users/")
    {
        return true;
    }
    lower
        .as_bytes()
        .windows(3)
        .any(|w| w[0].is_ascii_lowercase() && w[1] == b':' && w[2] == b'\\')
}

fn pattern_match_ci(pattern: &str, text: &str, raw_text: &str) -> bool {
    let pattern_key = pattern.trim().to_ascii_lowercase();
    let raw_lower = raw_text.to_ascii_lowercase();
    match pattern_key.as_str() {
        "https?:\\/\\/" => raw_lower.contains("http://") || raw_lower.contains("https://"),
        "(^|\\s)--?[a-z0-9][a-z0-9_-]*\\b" => contains_cli_flag(raw_text),
        "\\b(node|npm|pnpm|yarn|git|curl|python|bash|zsh|ollama)\\b" => {
            let tokens = tokenize(raw_text);
            [
                "node", "npm", "pnpm", "yarn", "git", "curl", "python", "bash", "zsh", "ollama",
            ]
            .iter()
            .any(|token| tokens.contains(*token))
        }
        "[`{}\\[\\]<>$;=]" => raw_text.chars().any(|ch| {
            matches!(
                ch,
                '`' | '{' | '}' | '[' | ']' | '<' | '>' | '$' | ';' | '='
            )
        }),
        "(^|\\s)(~\\/|\\.\\.?\\/|\\/users\\/|[a-z]:\\\\)" => {
            contains_shell_or_path_marker(raw_text)
        }
        _ => {
            let simplified = pattern_key
                .replace("\\b", "")
                .replace("\\s", " ")
                .replace("\\/", "/")
                .replace("\\\\", "\\");
            let needle = simplified
                .trim_matches(|ch| ch == '^' || ch == '$' || ch == '(' || ch == ')' || ch == '?');
            !needle.is_empty() && text.to_ascii_lowercase().contains(needle)
        }
    }
}

pub fn estimate_request_tokens(tokens_est: Option<f64>, intent: &str, task: &str) -> i64 {
    if let Some(direct) = tokens_est {
        if direct.is_finite() && direct > 0.0 {
            return clamp_request_tokens(direct.round() as i64);
        }
    }

    let text = format!("{intent} {task}");
    let text = text.trim();
    let chars = text.chars().count() as f64;
    let words = if text.is_empty() {
        0.0
    } else {
        text.split_whitespace().count() as f64
    };
    let heuristic = ((chars / 3.6) + (words * 1.6) + 80.0).round() as i64;
    clamp_request_tokens(heuristic)
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelTokenMultiplier {
    pub multiplier: f64,
    pub source: &'static str,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModelTokenEstimate {
    pub tokens_est: Option<i64>,
    pub multiplier: Option<f64>,
    pub source: &'static str,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RouteClassPolicy {
    pub id: String,
    pub force_risk: Option<String>,
    pub force_complexity: Option<String>,
    pub force_role: String,
    pub prefer_slot: Option<String>,
    pub prefer_model: Option<String>,
    pub fallback_slot: Option<String>,
    pub disable_fast_path: bool,
    pub max_tokens_est: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeAdjustmentInput {
    pub risk: String,
    pub complexity: String,
    pub role: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeAdjustment {
    pub risk: String,
    pub complexity: String,
    pub role: String,
    pub mode: String,
    pub mode_adjusted: bool,
    pub mode_reason: Option<String>,
    pub mode_policy_source: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommunicationFastPathPolicy {
    pub enabled: bool,
    pub match_mode: String,
    pub max_chars: i64,
    pub max_words: i64,
    pub max_newlines: i64,
    pub patterns: Vec<String>,
    pub disallow_regexes: Vec<String>,
    pub slot: String,
    pub prefer_model: String,
    pub fallback_slot: String,
    pub skip_outcome_scan: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommunicationFastPathResult {
    pub matched: bool,
    pub reason: String,
    pub policy: CommunicationFastPathPolicy,
    pub blocked_pattern: Option<String>,
    pub matched_pattern: Option<String>,
    pub text: Option<String>,
    pub slot: Option<String>,
    pub prefer_model: Option<String>,
    pub fallback_slot: Option<String>,
    pub skip_outcome_scan: Option<bool>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FallbackClassificationPolicy {
    pub enabled: bool,
    pub only_when_medium_medium: bool,
    pub prefer_chat_fast_path: bool,
    pub low_chars_max: f64,
    pub low_newlines_max: f64,
    pub high_chars_min: f64,
    pub high_newlines_min: f64,
    pub high_tokens_min: f64,
}

