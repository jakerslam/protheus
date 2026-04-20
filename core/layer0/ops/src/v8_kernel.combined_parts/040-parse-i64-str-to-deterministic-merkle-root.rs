
pub fn parse_i64_str(raw: Option<&str>, fallback: i64) -> i64 {
    raw.and_then(|v| v.trim().parse::<i64>().ok())
        .unwrap_or(fallback)
}

pub fn parse_flag(argv: &[String], key: &str) -> Option<String> {
    let needle = format!("--{key}");
    for idx in 0..argv.len() {
        let token = &argv[idx];
        if token == &needle {
            return argv.get(idx + 1).cloned();
        }
        let prefix = format!("{needle}=");
        if let Some(value) = token.strip_prefix(&prefix) {
            return Some(value.to_string());
        }
    }
    None
}

pub fn load_json_or(root: &Path, rel: &str, fallback: Value) -> Value {
    read_json(&root.join(rel)).unwrap_or(fallback)
}

pub fn canonicalize_json(value: &Value) -> Value {
    match value {
        Value::Object(map) => {
            let mut keys = map.keys().cloned().collect::<Vec<_>>();
            keys.sort();
            let mut out = serde_json::Map::new();
            for key in keys {
                if let Some(v) = map.get(&key) {
                    out.insert(key, canonicalize_json(v));
                }
            }
            Value::Object(out)
        }
        Value::Array(rows) => Value::Array(rows.iter().map(canonicalize_json).collect()),
        _ => value.clone(),
    }
}

pub fn canonical_json_string(value: &Value) -> String {
    serde_json::to_string(&canonicalize_json(value)).unwrap_or_else(|_| "null".to_string())
}

pub fn conduit_bypass_requested(flags: &HashMap<String, String>) -> bool {
    parse_bool(flags.get("bypass"), false)
        || parse_bool(flags.get("direct"), false)
        || parse_bool(flags.get("unsafe-client-route"), false)
        || parse_bool(flags.get("client-bypass"), false)
}

pub fn conduit_claim_rows(
    action: &str,
    bypass_requested: bool,
    claim: &str,
    claim_ids: &[&str],
) -> Vec<Value> {
    claim_ids
        .iter()
        .map(|id| {
            json!({
                "id": id,
                "claim": clean(claim, 240),
                "evidence": {
                    "action": clean(action, 120),
                    "bypass_requested": bypass_requested
                }
            })
        })
        .collect()
}

pub fn build_conduit_enforcement(
    root: &Path,
    env_key: &str,
    scope: &str,
    strict: bool,
    action: &str,
    receipt_type: &str,
    required_path: &str,
    bypass_requested: bool,
    claim_evidence: Vec<Value>,
) -> Value {
    let ok = !bypass_requested;
    let mut out = json!({
        "ok": if strict { ok } else { true },
        "type": clean(receipt_type, 120),
        "action": clean(action, 120),
        "required_path": clean(required_path, 240),
        "bypass_requested": bypass_requested,
        "errors": if ok { Value::Array(Vec::new()) } else { json!(["conduit_bypass_rejected"]) },
        "claim_evidence": claim_evidence
    });
    out.set_receipt_hash();
    let _ = append_jsonl(
        &scoped_state_root(root, env_key, scope)
            .join("conduit")
            .join("history.jsonl"),
        &out,
    );
    out
}

pub fn build_plane_conduit_enforcement(
    root: &Path,
    env_key: &str,
    scope: &str,
    strict: bool,
    action: &str,
    receipt_type: &str,
    required_path: &str,
    bypass_requested: bool,
    claim: &str,
    claim_ids: &[&str],
) -> Value {
    build_conduit_enforcement(
        root,
        env_key,
        scope,
        strict,
        action,
        receipt_type,
        required_path,
        bypass_requested,
        conduit_claim_rows(action, bypass_requested, claim, claim_ids),
    )
}

pub fn attach_conduit(mut payload: Value, conduit: Option<&Value>) -> Value {
    if let Some(gate) = conduit {
        payload["conduit_enforcement"] = gate.clone();
        let mut claims = payload
            .get("claim_evidence")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if let Some(rows) = gate.get("claim_evidence").and_then(Value::as_array) {
            claims.extend(rows.iter().cloned());
        }
        if !claims.is_empty() {
            payload["claim_evidence"] = Value::Array(claims);
        }
    }
    payload.set_receipt_hash();
    payload
}

pub fn sha256_hex_bytes(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    hex::encode(hasher.finalize())
}

pub fn sha256_hex_str(value: &str) -> String {
    sha256_hex_bytes(value.as_bytes())
}

pub fn sha256_file(path: &Path) -> Result<String, String> {
    let bytes =
        fs::read(path).map_err(|err| format!("read_file_failed:{}:{err}", path.display()))?;
    Ok(sha256_hex_bytes(&bytes))
}

pub fn keyed_digest_hex(secret: &str, payload: &Value) -> String {
    let rendered = serde_json::to_string(payload).unwrap_or_default();
    sha256_hex_str(&format!("{}:{}", clean(secret, 4096), rendered))
}

pub fn next_chain_hash(prev_hash: Option<&str>, payload: &Value) -> String {
    let prev = prev_hash.unwrap_or("genesis");
    let rendered = serde_json::to_string(payload).unwrap_or_default();
    sha256_hex_str(&format!("{prev}|{rendered}"))
}

pub fn deterministic_merkle_root(leaves: &[String]) -> String {
    if leaves.is_empty() {
        return sha256_hex_str("merkle:empty");
    }
    let mut level = leaves
        .iter()
        .map(|leaf| sha256_hex_str(&format!("leaf:{leaf}")))
        .collect::<Vec<_>>();
    while level.len() > 1 {
        let mut next = Vec::new();
        let mut i = 0usize;
        while i < level.len() {
            let left = &level[i];
            let right = if i + 1 < level.len() {
                &level[i + 1]
            } else {
                &level[i]
            };
            next.push(sha256_hex_str(&format!("node:{left}:{right}")));
            i += 2;
        }
        level = next;
    }
    level[0].clone()
}
