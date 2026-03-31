fn run_quantum_broker(root: &Path, strict: bool) -> Value {
    let contract = read_json(&root.join(QUANTUM_BROKER_DOMAIN_PATH)).unwrap_or(Value::Null);
    let mut errors = Vec::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("quantum_broker_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "quantum_broker_domain"
    {
        errors.push("quantum_broker_kind_must_be_quantum_broker_domain".to_string());
    }
    let ops = contract
        .get("operations")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if ops.is_empty() {
        errors.push("quantum_broker_operations_required".to_string());
    }
    let mut op_values = Vec::new();
    for row in ops {
        let id = row.as_str().unwrap_or_default().trim().to_ascii_lowercase();
        if id.is_empty() {
            errors.push("quantum_broker_operation_id_required".to_string());
            continue;
        }
        if !is_token_id(&id) {
            errors.push("quantum_broker_operation_id_invalid".to_string());
            continue;
        }
        op_values.push(id);
    }
    let set: BTreeSet<String> = op_values.iter().cloned().collect();
    if set.len() != op_values.len() {
        errors.push("quantum_broker_duplicate_operations".to_string());
    }
    let mut missing = Vec::new();
    for op in [
        "compile", "estimate", "submit", "session", "batch", "measure",
    ] {
        if !set.contains(op) {
            missing.push(op.to_string());
        }
    }
    if !missing.is_empty() {
        errors.push("quantum_broker_missing_required_operations".to_string());
    }
    let allowed_ops: BTreeSet<String> = [
        "compile", "estimate", "submit", "session", "batch", "measure",
    ]
    .iter()
    .map(|v| v.to_string())
    .collect();
    let unknown_operations: Vec<String> = set.difference(&allowed_ops).cloned().collect();
    if !unknown_operations.is_empty() {
        errors.push("quantum_broker_unknown_operation".to_string());
    }
    let fallback = contract
        .get("classical_fallback")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let fallback_ok = fallback
        .get("enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && fallback
            .get("receipt_required")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    if !fallback_ok {
        errors.push("quantum_broker_classical_fallback_policy_invalid".to_string());
    }
    let ok = errors.is_empty();
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "contract_path": QUANTUM_BROKER_DOMAIN_PATH,
        "missing_operations": missing,
        "unknown_operations": unknown_operations,
        "fallback_ok": fallback_ok,
        "errors": errors
    })
}

fn run_neural_consent_kernel(root: &Path, strict: bool) -> Value {
    let contract = read_json(&root.join(NEURAL_CONSENT_KERNEL_PATH)).unwrap_or(Value::Null);
    let mut errors = Vec::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("neural_consent_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "neural_consent_kernel"
    {
        errors.push("neural_consent_kind_must_be_neural_consent_kernel".to_string());
    }
    let authorities = contract
        .get("authorities")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if authorities.is_empty() {
        errors.push("neural_consent_authorities_required".to_string());
    }
    let mut authority_values = Vec::new();
    for row in authorities {
        let id = row.as_str().unwrap_or_default().trim().to_ascii_lowercase();
        if id.is_empty() {
            errors.push("neural_consent_authority_id_required".to_string());
            continue;
        }
        if !is_token_id(&id) {
            errors.push("neural_consent_authority_id_invalid".to_string());
            continue;
        }
        authority_values.push(id);
    }
    let set: BTreeSet<String> = authority_values.iter().cloned().collect();
    if set.len() != authority_values.len() {
        errors.push("neural_consent_duplicate_authorities".to_string());
    }
    let mut missing = Vec::new();
    for auth in ["observe", "infer", "feedback", "stimulate"] {
        if !set.contains(auth) {
            missing.push(auth.to_string());
        }
    }
    if !missing.is_empty() {
        errors.push("neural_consent_missing_required_authorities".to_string());
    }
    let allowed: BTreeSet<String> = ["observe", "infer", "feedback", "stimulate"]
        .iter()
        .map(|v| v.to_string())
        .collect();
    let unknown_authorities: Vec<String> = set.difference(&allowed).cloned().collect();
    if !unknown_authorities.is_empty() {
        errors.push("neural_consent_unknown_authority".to_string());
    }
    let stimulate = contract
        .get("stimulate_policy")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let stimulate_ok = stimulate
        .get("consent_token_required")
        .and_then(Value::as_bool)
        .unwrap_or(false)
        && stimulate
            .get("dual_control_required")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        && stimulate
            .get("immutable_audit")
            .and_then(Value::as_bool)
            .unwrap_or(false);
    let rate_limit_ok = stimulate
        .get("rate_limit_per_minute")
        .and_then(Value::as_i64)
        .map(|v| v > 0)
        .unwrap_or(false);
    if !stimulate_ok {
        errors.push("neural_consent_stimulate_policy_missing_controls".to_string());
    }
    if !rate_limit_ok {
        errors.push("neural_consent_stimulate_rate_limit_invalid".to_string());
    }
    let ok = errors.is_empty();
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "contract_path": NEURAL_CONSENT_KERNEL_PATH,
        "missing_authorities": missing,
        "unknown_authorities": unknown_authorities,
        "stimulate_policy_ok": stimulate_ok,
        "rate_limit_ok": rate_limit_ok,
        "errors": errors
    })
}

fn run_attestation_graph(root: &Path, strict: bool) -> Value {
    let graph = read_json(&root.join(ATTESTATION_GRAPH_PATH)).unwrap_or(Value::Null);
    let mut errors = Vec::new();
    if graph
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("attestation_graph_version_must_be_v1".to_string());
    }
    if graph
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "attestation_graph"
    {
        errors.push("attestation_graph_kind_must_be_attestation_graph".to_string());
    }
    let edges = graph
        .get("edges")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if edges.is_empty() {
        errors.push("attestation_graph_missing_edges".to_string());
    }
    let allowed_domains: BTreeSet<String> = ["code", "model", "policy", "data", "effect"]
        .iter()
        .map(|v| v.to_string())
        .collect();
    let mut domains = BTreeSet::new();
    let mut edge_keys = BTreeSet::new();
    let mut duplicate_edges = Vec::new();
    for edge in edges {
        let from = edge
            .get("from")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        let to = edge
            .get("to")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        if from.is_empty() || to.is_empty() {
            errors.push("attestation_edge_missing_endpoints".to_string());
            continue;
        }
        let key = format!("{from}->{to}");
        if !edge_keys.insert(key.clone()) {
            duplicate_edges.push(key);
        }
        let fdom = from.split(':').next().unwrap_or_default().to_string();
        let tdom = to.split(':').next().unwrap_or_default().to_string();
        let fval = from
            .split_once(':')
            .map(|(_, tail)| tail.trim().to_string())
            .unwrap_or_default();
        let tval = to
            .split_once(':')
            .map(|(_, tail)| tail.trim().to_string())
            .unwrap_or_default();
        if fdom.is_empty() || tdom.is_empty() || fval.is_empty() || tval.is_empty() {
            errors.push("attestation_edge_invalid_domain_path_format".to_string());
            continue;
        }
        if from == to {
            errors.push("attestation_edge_self_link_forbidden".to_string());
        }
        if !allowed_domains.contains(&fdom) || !allowed_domains.contains(&tdom) {
            errors.push("attestation_edge_domain_unknown".to_string());
        }
        if !fdom.is_empty() {
            domains.insert(fdom);
        }
        if !tdom.is_empty() {
            domains.insert(tdom);
        }
    }
    for dom in ["code", "model", "policy", "data", "effect"] {
        if !domains.contains(dom) {
            errors.push(format!("attestation_graph_missing_domain::{dom}"));
        }
    }
    if !duplicate_edges.is_empty() {
        errors.push("attestation_graph_duplicate_edges".to_string());
    }
    let ok = errors.is_empty();
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "graph_path": ATTESTATION_GRAPH_PATH,
        "duplicate_edges": duplicate_edges,
        "errors": errors
    })
}

fn run_degradation_contracts(root: &Path, strict: bool) -> Value {
    let contract = read_json(&root.join(DEGRADATION_CONTRACT_PATH)).unwrap_or(Value::Null);
    let mut errors = Vec::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("degradation_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "degradation_contracts"
    {
        errors.push("degradation_contract_kind_must_be_degradation_contracts".to_string());
    }
    let lanes = contract
        .get("critical_lanes")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if lanes.is_empty() {
        errors.push("degradation_contract_missing_critical_lanes".to_string());
    }
    let mut lane_ids = BTreeSet::new();
    let mut lane_ids_vec = Vec::new();
    for lane in lanes {
        let id = lane
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_ascii_lowercase();
        let fallback = lane
            .get("fallback")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .trim()
            .to_string();
        let widens = lane
            .get("fallback_widens_privilege")
            .and_then(Value::as_bool)
            .unwrap_or(true);
        if id.is_empty() {
            errors.push("degradation_lane_id_required".to_string());
        } else {
            if !is_token_id(&id) {
                errors.push("degradation_lane_id_invalid".to_string());
            }
            if !lane_ids.insert(id.clone()) {
                errors.push("degradation_lane_duplicate_id".to_string());
            }
            lane_ids_vec.push(id);
        }
        if fallback.is_empty() {
            errors.push("degradation_lane_missing_fallback".to_string());
        } else if !is_token_id(&fallback.to_ascii_lowercase()) {
            errors.push("degradation_lane_fallback_invalid".to_string());
        }
        if widens {
            errors.push("degradation_fallback_widens_privilege".to_string());
        }
    }
    let scenario_map = contract
        .get("scenarios")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    for scenario in ["no-network", "no-ternary", "no-qpu", "neural-link-loss"] {
        let refs = scenario_map
            .get(scenario)
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        if refs.is_empty() {
            errors.push(format!("degradation_scenario_missing_or_empty::{scenario}"));
            continue;
        }
        for id in refs {
            let lane_id = id.as_str().unwrap_or_default().trim().to_ascii_lowercase();
            if lane_id.is_empty() || !lane_ids.contains(&lane_id) {
                errors.push(format!("degradation_scenario_unknown_lane::{scenario}"));
                break;
            }
        }
    }
    let ok = errors.is_empty();
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "contract_path": DEGRADATION_CONTRACT_PATH,
        "critical_lane_ids": lane_ids_vec,
        "errors": errors
    })
}

