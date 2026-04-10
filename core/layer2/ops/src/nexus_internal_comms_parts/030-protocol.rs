#[derive(Debug, Clone)]
struct NexusMessage {
    from: String,
    to: String,
    module: Option<String>,
    cmd: String,
    kv: BTreeMap<String, String>,
}

fn parse_kv_token(token: &str) -> Result<(String, String), String> {
    let (k, v) = token
        .split_once('=')
        .ok_or_else(|| format!("invalid_kv:{token}"))?;
    let key = normalize_token(k);
    let value = normalize_token(v);
    if key.is_empty() || value.is_empty() {
        return Err(format!("invalid_kv:{token}"));
    }
    Ok((key, value))
}

fn lexicon_expand(lexicon: &BTreeMap<String, String>, token: &str) -> String {
    lexicon
        .get(token)
        .cloned()
        .unwrap_or_else(|| token.to_ascii_lowercase())
}

fn parse_nexus_message(raw: &str) -> Result<NexusMessage, String> {
    let trimmed = raw.trim();
    if trimmed.contains('\n') || trimmed.contains('\r') {
        return Err("multiline_not_allowed".to_string());
    }
    if !trimmed.starts_with('[') {
        return Err("missing_header_open".to_string());
    }
    let close_idx = trimmed
        .find(']')
        .ok_or_else(|| "missing_header_close".to_string())?;
    let header = &trimmed[1..close_idx];
    let body = trimmed[close_idx + 1..].trim();
    if body.is_empty() {
        return Err("missing_body".to_string());
    }
    let arrow_idx = header
        .find('>')
        .ok_or_else(|| "missing_from_to_separator".to_string())?;
    let from = normalize_id(&header[..arrow_idx]);
    if from.is_empty() {
        return Err("missing_from".to_string());
    }
    let right = &header[arrow_idx + 1..];
    let (to_raw, module_raw) = if let Some(pipe_idx) = right.find('|') {
        (&right[..pipe_idx], Some(right[pipe_idx + 1..].trim()))
    } else {
        (right, None)
    };
    let to = normalize_id(to_raw);
    if to.is_empty() {
        return Err("missing_to".to_string());
    }
    let module = module_raw
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty());
    let mut parts = body.split_whitespace();
    let cmd = normalize_token(parts.next().unwrap_or_default());
    if cmd.is_empty() {
        return Err("missing_cmd".to_string());
    }
    let mut kv = BTreeMap::<String, String>::new();
    for token in parts {
        let (key, value) = parse_kv_token(token)?;
        kv.insert(key, value);
    }
    Ok(NexusMessage {
        from,
        to,
        module,
        cmd,
        kv,
    })
}

fn format_nexus_message(message: &NexusMessage) -> String {
    let mut head = format!("[{}>{}", message.from, message.to);
    if let Some(module) = &message.module {
        head.push('|');
        head.push_str(module);
    }
    head.push(']');
    let mut out = format!("{head} {}", message.cmd);
    for (k, v) in &message.kv {
        out.push(' ');
        out.push_str(k);
        out.push('=');
        out.push_str(v);
    }
    out
}

fn validate_module_rules(message: &NexusMessage, modules: &[String]) -> Result<(), String> {
    if let Some(module) = &message.module {
        if !modules.iter().any(|m| m == module) {
            return Err(format!("module_not_loaded:{module}"));
        }
    }
    if modules.len() > MAX_MODULES_PER_AGENT {
        return Err("module_limit_exceeded".to_string());
    }
    Ok(())
}

fn decompress_message(message: &NexusMessage, lexicon: &BTreeMap<String, String>) -> Value {
    let cmd_expanded = lexicon_expand(lexicon, &message.cmd);
    let mut kv = Map::<String, Value>::new();
    for (k, v) in &message.kv {
        let expanded = lexicon_expand(lexicon, v);
        kv.insert(k.to_ascii_lowercase(), Value::String(expanded));
    }
    json!({
        "from": message.from,
        "to": message.to,
        "module": message.module,
        "cmd": cmd_expanded,
        "kv": kv
    })
}

fn compress_text_to_message(
    from: &str,
    to: &str,
    module: Option<String>,
    cmd: &str,
    text: &str,
    reverse: &BTreeMap<String, String>,
) -> (NexusMessage, bool) {
    let mut mapped = Vec::<String>::new();
    for atom in text.split_whitespace().map(normalize_text_atom) {
        if atom.is_empty() {
            continue;
        }
        if let Some(code) = reverse.get(&atom) {
            mapped.push(code.clone());
        }
    }
    let fallback_used = mapped.is_empty();
    if fallback_used {
        mapped.push(format!("NL_{}", normalize_token(text)));
    }
    let mut kv = BTreeMap::<String, String>::new();
    for (idx, token) in mapped.iter().enumerate() {
        kv.insert(format!("T{idx}"), normalize_token(token));
    }
    let msg = NexusMessage {
        from: normalize_id(from),
        to: normalize_id(to),
        module: module.filter(|v| !v.trim().is_empty()),
        cmd: normalize_token(cmd),
        kv,
    };
    (msg, fallback_used)
}

fn estimate_savings(raw_tokens: usize, nexus_tokens: usize) -> f64 {
    if raw_tokens == 0 {
        return 0.0;
    }
    ((raw_tokens.saturating_sub(nexus_tokens) as f64 / raw_tokens as f64) * 10000.0).round() / 100.0
}
