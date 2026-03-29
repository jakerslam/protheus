
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct TruthGateRule {
    id: String,
    trigger_tokens: Vec<String>,
    require_evidence: bool,
    min_evidence_items: usize,
    deny_reason: String,
}

impl Default for TruthGateRule {
    fn default() -> Self {
        Self {
            id: "default_unverified_agreement".to_string(),
            trigger_tokens: vec!["agree".to_string(), "approved".to_string()],
            require_evidence: true,
            min_evidence_items: 1,
            deny_reason: "agreement_without_verification_denied".to_string(),
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(default)]
struct TruthGatePolicy {
    version: String,
    enabled: bool,
    identity_binding: TruthGateIdentityBinding,
    deny_without_evidence: bool,
    min_evidence_items: usize,
    agreement_tokens: Vec<String>,
    rules: Vec<TruthGateRule>,
}

impl Default for TruthGatePolicy {
    fn default() -> Self {
        Self {
            version: "1.0".to_string(),
            enabled: true,
            identity_binding: TruthGateIdentityBinding::default(),
            deny_without_evidence: true,
            min_evidence_items: 1,
            agreement_tokens: vec![
                "agree".to_string(),
                "agreed".to_string(),
                "approved".to_string(),
                "sounds good".to_string(),
                "yes".to_string(),
            ],
            rules: vec![TruthGateRule::default()],
        }
    }
}

fn abac_paths(repo_root: &Path) -> (PathBuf, PathBuf, PathBuf, PathBuf) {
    let policy_path = std::env::var("ABAC_POLICY_PATH")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| runtime_config_path(repo_root, "abac_policy_plane.json"));
    let latest_path = std::env::var("ABAC_LATEST_PATH")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            runtime_state_root(repo_root)
                .join("security")
                .join("abac_policy_plane_latest.json")
        });
    let history_path = std::env::var("ABAC_HISTORY_PATH")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            runtime_state_root(repo_root)
                .join("security")
                .join("abac_policy_plane_history.jsonl")
        });
    let flight_recorder_path = std::env::var("ABAC_FLIGHT_RECORDER_PATH")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            runtime_state_root(repo_root)
                .join("security")
                .join("abac_flight_recorder.jsonl")
        });
    (policy_path, latest_path, history_path, flight_recorder_path)
}

fn parse_object_map(raw: Option<&String>) -> Result<Map<String, Value>, String> {
    let text = match raw {
        Some(v) => v.trim(),
        None => return Ok(Map::new()),
    };
    if text.is_empty() {
        return Ok(Map::new());
    }
    let parsed = serde_json::from_str::<Value>(text)
        .map_err(|err| format!("invalid_json_object_payload:{err}"))?;
    parsed
        .as_object()
        .cloned()
        .ok_or_else(|| "json_object_payload_must_be_object".to_string())
}

fn normalized_value(raw: Option<&str>) -> Option<String> {
    raw.map(|v| clean_text(v, 160).to_ascii_lowercase())
        .filter(|v| !v.is_empty())
}

fn rule_dimension_allows(scope: Option<&Value>, fields: &Map<String, Value>) -> bool {
    let Some(scope_obj) = scope.and_then(Value::as_object) else {
        return true;
    };
    for (key, expected) in scope_obj {
        let actual = fields
            .get(key)
            .and_then(Value::as_str)
            .map(|v| v.to_ascii_lowercase())
            .unwrap_or_default();
        let expected_values = expected
            .as_array()
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(|v| v.as_str().map(|x| x.to_ascii_lowercase()))
            .collect::<Vec<_>>();
        if expected_values.is_empty() {
            continue;
        }
        if !expected_values
            .iter()
            .any(|v| v == "*" || (!actual.is_empty() && v == &actual))
        {
            return false;
        }
    }
    true
}

fn abac_trace_hash(payload: &Value) -> String {
    let mut basis = payload.clone();
    if let Some(obj) = basis.as_object_mut() {
        obj.remove("hash");
        obj.remove("receipt_hash");
    }
    sha256_hex(&stable_json_string(&basis))
}

pub fn run_abac_policy_plane(repo_root: &Path, argv: &[String]) -> (Value, i32) {
    let args = parse_cli_args(argv);
    let cmd = args
        .positional
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let (policy_path, latest_path, history_path, flight_path) = abac_paths(repo_root);
    let policy_json = read_json_or(
        &policy_path,
        json!({
            "version": "v1",
            "kind": "abac_policy_plane",
            "default_effect": "deny",
            "rules": [],
            "flight_recorder": {
                "immutable": true,
                "hash_chain": true,
                "redact_subject_fields": []
            }
        }),
    );
    let rules = policy_json
        .get("rules")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let default_effect = policy_json
        .get("default_effect")
        .and_then(Value::as_str)
        .unwrap_or("deny")
        .to_ascii_lowercase();
    let redact_fields = policy_json
        .get("flight_recorder")
        .and_then(|v| v.get("redact_subject_fields"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(|v| v.as_str().map(|x| x.to_ascii_lowercase()))
        .collect::<HashSet<_>>();

    if cmd == "status" {
        let latest = read_json_or(&latest_path, Value::Null);
        let mut out = json!({
            "ok": true,
            "type": "abac_policy_plane_status",
            "ts": now_iso(),
            "policy_path": normalize_rel_path(policy_path.display().to_string()),
            "latest_path": normalize_rel_path(latest_path.display().to_string()),
            "history_path": normalize_rel_path(history_path.display().to_string()),
            "flight_recorder_path": normalize_rel_path(flight_path.display().to_string()),
            "rules_count": rules.len(),
            "latest": latest
        });
        out["receipt_hash"] = Value::String(abac_trace_hash(&out));
        return (out, 0);
    }

    if cmd != "evaluate" {
        return (
            json!({
                "ok": false,
                "type": "abac_policy_plane_error",
                "reason": format!("unknown_command:{cmd}")
            }),
            2,
        );
    }

    let mut subject = match parse_object_map(args.flags.get("subject-json")) {
        Ok(v) => v,
        Err(err) => {
            return (
                json!({
                    "ok": false,
                    "type": "abac_policy_plane_evaluate",
                    "reason": err
                }),
                2,
            )
        }
    };
    if let Some(role) = normalized_value(args.flags.get("subject-role").map(String::as_str)) {
        subject.insert("role".to_string(), Value::String(role));
    }
    if let Some(id) = normalized_value(args.flags.get("subject-id").map(String::as_str)) {
        subject.insert("id".to_string(), Value::String(id));
    }

    let mut object = match parse_object_map(args.flags.get("object-json")) {
        Ok(v) => v,
        Err(err) => {
            return (
                json!({
                    "ok": false,
                    "type": "abac_policy_plane_evaluate",
                    "reason": err
                }),
                2,
            )
        }
    };
    if let Some(classification) =
        normalized_value(args.flags.get("object-classification").map(String::as_str))
    {
        object.insert("classification".to_string(), Value::String(classification));
    }
    if let Some(id) = normalized_value(args.flags.get("object-id").map(String::as_str)) {
        object.insert("id".to_string(), Value::String(id));
    }

    let mut context = match parse_object_map(args.flags.get("context-json")) {
        Ok(v) => v,
        Err(err) => {
            return (
                json!({
                    "ok": false,
                    "type": "abac_policy_plane_evaluate",
                    "reason": err
                }),
                2,
            )
        }
    };
    if let Some(env) = normalized_value(args.flags.get("context-env").map(String::as_str)) {
        context.insert("env".to_string(), Value::String(env));
    }
    if let Some(trust) = normalized_value(args.flags.get("context-trust").map(String::as_str)) {
        context.insert("trust".to_string(), Value::String(trust));
    }
    let action = normalized_value(args.flags.get("action").map(String::as_str)).unwrap_or_default();
    if action.is_empty() {
        return (
            json!({
                "ok": false,
                "type": "abac_policy_plane_evaluate",
                "reason": "missing_action"
            }),
            2,
        );
    }

    let subject_value = Value::Object(subject.clone());
    let object_value = Value::Object(object.clone());
    let context_value = Value::Object(context.clone());

    let mut decision_effect = default_effect.clone();
    let mut matched_rule_id = Value::Null;
    for rule in &rules {
        let action_ok = rule
            .get("action")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .filter_map(|v| v.as_str().map(|x| x.to_ascii_lowercase()))
            .any(|v| v == "*" || v == action);
        if !action_ok {
            continue;
        }
        if !rule_dimension_allows(rule.get("subject"), &subject) {
            continue;
        }
        if !rule_dimension_allows(rule.get("object"), &object) {
            continue;
        }
        if !rule_dimension_allows(rule.get("context"), &context) {
            continue;
        }
        decision_effect = rule
            .get("effect")
            .and_then(Value::as_str)
            .unwrap_or("deny")
            .to_ascii_lowercase();
        matched_rule_id = rule
            .get("id")
            .and_then(Value::as_str)
            .map(|v| Value::String(v.to_string()))
            .unwrap_or(Value::Null);
        break;
    }

    let allowed = decision_effect == "allow";
    let mut redacted_subject = subject.clone();
    for key in &redact_fields {
        if redacted_subject.contains_key(key) {
            redacted_subject.insert(key.clone(), Value::String("***".to_string()));
        }
    }

    let previous_hash = read_jsonl(&flight_path)
        .last()
        .and_then(|v| v.get("hash").and_then(Value::as_str))
        .map(ToString::to_string)
        .unwrap_or_else(|| "GENESIS".to_string());
    let mut flight_row = json!({
        "type": "abac_flight_recorder_event",
        "ts": now_iso(),
        "subject": redacted_subject,
        "object": object,
        "context": context,
        "action": action,
        "decision": if allowed { "allow" } else { "deny" },
        "matched_rule_id": matched_rule_id,
        "prev_hash": previous_hash
    });
    let hash = abac_trace_hash(&flight_row);
    flight_row["hash"] = Value::String(hash.clone());
    let _ = append_jsonl(&flight_path, &flight_row);

    let mut out = json!({
        "ok": allowed,
        "type": "abac_policy_plane_evaluate",
        "ts": now_iso(),
        "subject": subject_value,
        "object": object_value,
        "context": context_value,
        "action": action,
        "decision": if allowed { "allow" } else { "deny" },
        "matched_rule_id": matched_rule_id,
        "policy_path": normalize_rel_path(policy_path.display().to_string()),
        "flight_recorder_path": normalize_rel_path(flight_path.display().to_string()),
        "flight_recorder_hash": hash,
        "claim_evidence": [
            {
                "id": "V7-ASM-006",
                "claim": "abac_evaluation_emits_immutable_flight_recorder_trace",
                "evidence": {
                    "matched_rule_id": matched_rule_id,
                    "decision": if allowed { "allow" } else { "deny" }
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(abac_trace_hash(&out));
    let _ = append_jsonl(&history_path, &out);
    let _ = write_json_atomic(&latest_path, &out);
    (out, if allowed { 0 } else { 1 })
}

fn truth_gate_paths(repo_root: &Path) -> (PathBuf, PathBuf, PathBuf) {
    let policy_path = std::env::var("TRUTH_GATE_POLICY_PATH")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| runtime_config_path(repo_root, "truth_gate_policy.json"));
    let latest_path = std::env::var("TRUTH_GATE_LATEST_PATH")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            runtime_state_root(repo_root)
                .join("security")
                .join("truth_gate_latest.json")
        });
    let history_path = std::env::var("TRUTH_GATE_HISTORY_PATH")
        .ok()
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            runtime_state_root(repo_root)
                .join("security")
                .join("truth_gate_history.jsonl")
        });
    (policy_path, latest_path, history_path)
}

fn normalize_tokens_csv(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(|part| clean_text(part, 120).to_ascii_lowercase())
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
}

fn truth_gate_receipt_hash(payload: &Value) -> String {
    let mut basis = payload.clone();
    if let Some(obj) = basis.as_object_mut() {
        obj.remove("receipt_hash");
    }
    sha256_hex(&stable_json_string(&basis))
}

fn claim_has_token(claim_lc: &str, tokens: &[String]) -> bool {
    tokens.iter().any(|token| {
        let clean_token = clean_text(token, 120).to_ascii_lowercase();
        !clean_token.is_empty() && claim_lc.contains(&clean_token)
    })
}
