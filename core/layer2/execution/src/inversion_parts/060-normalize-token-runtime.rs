fn normalize_token_runtime(raw: &str, max_len: usize) -> String {
    let src = clean_text_runtime(raw, max_len).to_lowercase();
    let mut out = String::new();
    let mut prev_underscore = false;
    for ch in src.chars() {
        let keep = ch.is_ascii_lowercase()
            || ch.is_ascii_digit()
            || ch == '_'
            || ch == '.'
            || ch == ':'
            || ch == '-';
        if keep {
            out.push(ch);
            prev_underscore = false;
        } else if !prev_underscore {
            out.push('_');
            prev_underscore = true;
        }
    }
    out.trim_matches('_').to_string()
}

fn parse_number_like(value: Option<&Value>) -> Option<f64> {
    let v = value?;
    if let Some(n) = v.as_f64() {
        return Some(n);
    }
    if let Some(s) = v.as_str() {
        return s.trim().parse::<f64>().ok();
    }
    if let Some(b) = v.as_bool() {
        return Some(if b { 1.0 } else { 0.0 });
    }
    None
}

fn value_to_string(value: Option<&Value>) -> String {
    let Some(v) = value else {
        return String::new();
    };
    if let Some(s) = v.as_str() {
        return s.to_string();
    }
    if v.is_null() {
        return String::new();
    }
    v.to_string()
}

fn push_unique(values: &mut Vec<String>, next: String) {
    if !values.iter().any(|item| item == &next) {
        values.push(next);
    }
}

fn round6(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

fn js_truthy(value: Option<&Value>) -> bool {
    let Some(v) = value else {
        return false;
    };
    match v {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::Number(n) => n
            .as_f64()
            .map(|x| x != 0.0 && x.is_finite())
            .unwrap_or(false),
        Value::String(s) => !s.is_empty(),
        Value::Array(arr) => !arr.is_empty(),
        Value::Object(map) => !map.is_empty(),
    }
}

fn js_or_number(value: Option<&Value>, fallback: f64) -> f64 {
    let Some(v) = value else {
        return fallback;
    };
    if !js_truthy(Some(v)) {
        return fallback;
    }
    parse_number_like(Some(v)).unwrap_or(fallback)
}
fn to_bool_like(value: Option<&Value>, fallback: bool) -> bool {
    let Some(v) = value else {
        return fallback;
    };
    let raw = match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => {
            if *b {
                "true".to_string()
            } else {
                "false".to_string()
            }
        }
        _ => v.to_string(),
    }
    .trim()
    .to_lowercase();
    if ["1", "true", "yes", "on"].contains(&raw.as_str()) {
        return true;
    }
    if ["0", "false", "no", "off"].contains(&raw.as_str()) {
        return false;
    }
    fallback
}

fn clamp_int_value(value: Option<&Value>, lo: i64, hi: i64, fallback: i64) -> i64 {
    let parsed = parse_number_like(value).unwrap_or(fallback as f64).floor() as i64;
    parsed.clamp(lo, hi)
}

fn map_number_key(map_value: Option<&Value>, key: &str, lo: f64, hi: f64, fallback: f64) -> f64 {
    let v = map_value
        .and_then(|v| v.as_object())
        .and_then(|m| m.get(key))
        .and_then(|row| parse_number_like(Some(row)))
        .unwrap_or(fallback);
    clamp_number(v, lo, hi)
}

fn map_int_key(map_value: Option<&Value>, key: &str, lo: i64, hi: i64, fallback: i64) -> i64 {
    clamp_int_value(
        map_value
            .and_then(|v| v.as_object())
            .and_then(|m| m.get(key)),
        lo,
        hi,
        fallback,
    )
}

fn map_bool_key(map_value: Option<&Value>, key: &str, fallback: bool) -> bool {
    to_bool_like(
        map_value
            .and_then(|v| v.as_object())
            .and_then(|m| m.get(key)),
        fallback,
    )
}

fn normalize_target_for_key(target: &str) -> String {
    compute_normalize_target(&NormalizeTargetInput {
        value: Some(target.to_string()),
    })
    .value
}

fn number_path(root: Option<&Value>, path: &[&str], fallback: f64) -> f64 {
    let mut cursor = root;
    for key in path {
        cursor = cursor.and_then(|v| v.as_object()).and_then(|m| m.get(*key));
    }
    parse_number_like(cursor).unwrap_or(fallback)
}

fn value_path<'a>(root: Option<&'a Value>, path: &[&str]) -> Option<&'a Value> {
    let mut cursor = root;
    for key in path {
        cursor = cursor.and_then(|v| v.as_object()).and_then(|m| m.get(*key));
    }
    cursor
}

fn stable_id_runtime(seed: &str, prefix: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    let digest = format!("{:x}", hasher.finalize());
    format!("{}_{}", prefix, &digest[..16])
}

fn normalize_slashes(value: &str) -> String {
    value.replace('\\', "/")
}

fn split_path_components(value: &str) -> (String, Vec<String>) {
    let normalized = normalize_slashes(value.trim());
    let mut prefix = String::new();
    let mut cursor = normalized.as_str();

    let bytes = normalized.as_bytes();
    if bytes.len() >= 2 && bytes[1] == b':' {
        prefix = normalized[..2].to_lowercase();
        cursor = &normalized[2..];
    } else if let Some(stripped) = normalized.strip_prefix('/') {
        prefix = "/".to_string();
        cursor = stripped;
    }

    let mut parts: Vec<String> = Vec::new();
    for raw in cursor.split('/') {
        if raw.is_empty() || raw == "." {
            continue;
        }
        if raw == ".." {
            if !parts.is_empty() && parts.last().map(|last| last != "..").unwrap_or(false) {
                parts.pop();
            } else if prefix.is_empty() {
                parts.push("..".to_string());
            }
            continue;
        }
        parts.push(raw.to_string());
    }
    (prefix, parts)
}

fn rel_path_runtime(root: &str, file_path: &str) -> String {
    let root_clean = root.trim();
    let file_clean = file_path.trim();
    if file_clean.is_empty() {
        return String::new();
    }
    let normalized_file = normalize_slashes(file_clean);
    if root_clean.is_empty() {
        return normalized_file;
    }

    let (root_prefix, root_parts) = split_path_components(root_clean);
    let (file_prefix, file_parts) = split_path_components(file_clean);
    if root_prefix != file_prefix {
        return normalized_file;
    }

    let mut common = 0usize;
    while common < root_parts.len() && common < file_parts.len() {
        if root_parts[common] != file_parts[common] {
            break;
        }
        common += 1;
    }

    let mut out: Vec<String> = Vec::new();
    for _ in common..root_parts.len() {
        out.push("..".to_string());
    }
    for part in file_parts.iter().skip(common) {
        out.push(part.to_string());
    }

    out.join("/")
}

fn js_number_for_extract(value: Option<&Value>) -> Option<f64> {
    let v = value?;
    match v {
        Value::Null => Some(0.0),
        Value::Bool(b) => Some(if *b { 1.0 } else { 0.0 }),
        Value::Number(n) => n.as_f64(),
        Value::String(s) => {
            let trimmed = s.trim();
            if trimmed.is_empty() {
                return Some(0.0);
            }
            trimmed.parse::<f64>().ok()
        }
        _ => None,
    }
}

pub fn compute_normalize_band_map(input: &NormalizeBandMapInput) -> NormalizeBandMapOutput {
    let lo = input.lo.unwrap_or(0.0);
    let hi = input.hi.unwrap_or(1.0);
    let base = input.base.as_ref();
    let raw = input.raw.as_ref();
    NormalizeBandMapOutput {
        novice: map_number_key(
            raw,
            "novice",
            lo,
            hi,
            map_number_key(base, "novice", lo, hi, lo),
        ),
        developing: map_number_key(
            raw,
            "developing",
            lo,
            hi,
            map_number_key(base, "developing", lo, hi, lo),
        ),
        mature: map_number_key(
            raw,
            "mature",
            lo,
            hi,
            map_number_key(base, "mature", lo, hi, lo),
        ),
        seasoned: map_number_key(
            raw,
            "seasoned",
            lo,
            hi,
            map_number_key(base, "seasoned", lo, hi, lo),
        ),
        legendary: map_number_key(
            raw,
            "legendary",
            lo,
            hi,
            map_number_key(base, "legendary", lo, hi, lo),
        ),
    }
}

pub fn compute_normalize_impact_map(input: &NormalizeImpactMapInput) -> NormalizeImpactMapOutput {
    let lo = input.lo.unwrap_or(0.0);
    let hi = input.hi.unwrap_or(1.0);
    let base = input.base.as_ref();
    let raw = input.raw.as_ref();
    NormalizeImpactMapOutput {
        low: map_number_key(raw, "low", lo, hi, map_number_key(base, "low", lo, hi, lo)),
        medium: map_number_key(
            raw,
            "medium",
            lo,
            hi,
            map_number_key(base, "medium", lo, hi, lo),
        ),
        high: map_number_key(
            raw,
            "high",
            lo,
            hi,
            map_number_key(base, "high", lo, hi, lo),
        ),
        critical: map_number_key(
            raw,
            "critical",
            lo,
            hi,
            map_number_key(base, "critical", lo, hi, lo),
        ),
    }
}

pub fn compute_normalize_target_map(input: &NormalizeTargetMapInput) -> NormalizeTargetMapOutput {
    let lo = input.lo.unwrap_or(0.0);
    let hi = input.hi.unwrap_or(1.0);
    let base = input.base.as_ref();
    let raw = input.raw.as_ref();
    NormalizeTargetMapOutput {
        tactical: map_number_key(
            raw,
            "tactical",
            lo,
            hi,
            map_number_key(base, "tactical", lo, hi, lo),
        ),
        belief: map_number_key(
            raw,
            "belief",
            lo,
            hi,
            map_number_key(base, "belief", lo, hi, lo),
        ),
        identity: map_number_key(
            raw,
            "identity",
            lo,
            hi,
            map_number_key(base, "identity", lo, hi, lo),
        ),
        directive: map_number_key(
            raw,
            "directive",
            lo,
            hi,
            map_number_key(base, "directive", lo, hi, lo),
        ),
        constitution: map_number_key(
            raw,
            "constitution",
            lo,
            hi,
            map_number_key(base, "constitution", lo, hi, lo),
        ),
    }
}

pub fn compute_normalize_target_policy(
    input: &NormalizeTargetPolicyInput,
) -> NormalizeTargetPolicyOutput {
    let raw = input.raw.as_ref();
    let base = input.base.as_ref();
    NormalizeTargetPolicyOutput {
        rank: map_int_key(raw, "rank", 1, 10, map_int_key(base, "rank", 1, 10, 1)),
        live_enabled: map_bool_key(
            raw,
            "live_enabled",
            map_bool_key(base, "live_enabled", false),
        ),
        test_enabled: map_bool_key(
            raw,
            "test_enabled",
            map_bool_key(base, "test_enabled", true),
        ),
        require_human_veto_live: map_bool_key(
            raw,
            "require_human_veto_live",
            map_bool_key(base, "require_human_veto_live", false),
        ),
        min_shadow_hours: map_int_key(
            raw,
            "min_shadow_hours",
            0,
            24 * 365,
            map_int_key(base, "min_shadow_hours", 0, 24 * 365, 0),
        ),
    }
}

pub fn compute_window_days_for_target(
    input: &WindowDaysForTargetInput,
) -> WindowDaysForTargetOutput {
    let target = normalize_target_for_key(input.target.as_deref().unwrap_or("tactical"));
    let fallback = input.fallback.unwrap_or(90).clamp(1, 3650);
    let days = map_int_key(input.window_map.as_ref(), &target, 1, 3650, fallback);
    WindowDaysForTargetOutput { days }
}

pub fn compute_tier_retention_days(input: &TierRetentionDaysInput) -> TierRetentionDaysOutput {
    let policy = input.policy.as_ref();
    let transition = value_path(policy, &["tier_transition", "window_days_by_target"])
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let transition_min = value_path(
        policy,
        &["tier_transition", "minimum_window_days_by_target"],
    )
    .and_then(|v| v.as_object())
    .cloned()
    .unwrap_or_default();
    let shadow = value_path(policy, &["shadow_pass_gate", "window_days_by_target"])
        .and_then(|v| v.as_object())
        .cloned()
        .unwrap_or_default();
    let mut all = Vec::new();
    for map in [transition, transition_min, shadow] {
        for value in map.values() {
            all.push(clamp_int_value(Some(value), 1, 3650, 1));
        }
    }
    let mut max_days = 365i64;
    for days in all {
        if days > max_days {
            max_days = days;
        }
    }
    if max_days < 30 {
        max_days = 30;
    }
    TierRetentionDaysOutput { days: max_days }
}
