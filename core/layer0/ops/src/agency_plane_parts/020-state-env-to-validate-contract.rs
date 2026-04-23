
const STATE_ENV: &str = "AGENCY_PLANE_STATE_ROOT";
const STATE_SCOPE: &str = "agency_plane";

const TEMPLATE_CONTRACT_PATH: &str =
    "planes/contracts/agency/personality_template_pack_contract_v1.json";
const TOPOLOGY_CONTRACT_PATH: &str = "planes/contracts/agency/division_topology_contract_v1.json";
const ORCHESTRATOR_CONTRACT_PATH: &str =
    "planes/contracts/agency/multi_agent_orchestrator_contract_v1.json";
const WORKFLOW_BINDING_CONTRACT_PATH: &str =
    "planes/contracts/agency/workflow_metric_binding_contract_v1.json";

fn usage() {
    println!("Usage:");
    println!("  infring-ops agency-plane status");
    println!(
        "  infring-ops agency-plane create-shadow --template=<id> [--name=<shadow-name>] [--strict=1|0]"
    );
    println!("  infring-ops agency-plane topology [--manifest-json=<json>] [--strict=1|0]");
    println!(
        "  infring-ops agency-plane orchestrate [--team=<id>] [--run-id=<id>] [--agents=<n>] [--strict=1|0]"
    );
    println!(
        "  infring-ops agency-plane workflow-bind --template=<id> [--run-id=<id>] [--workflow-json=<json>] [--strict=1|0]"
    );
}

fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn emit(root: &Path, payload: Value) -> i32 {
    emit_plane_receipt(root, STATE_ENV, STATE_SCOPE, "agency_plane_error", payload)
}

fn status(root: &Path) -> Value {
    let mut out = plane_status(root, STATE_ENV, STATE_SCOPE, "agency_plane_status");
    out["conduit_lifecycle"] = load_conduit_lifecycle(root);
    out
}

fn conduit_enforcement(
    root: &Path,
    parsed: &crate::ParsedArgs,
    strict: bool,
    action: &str,
) -> Value {
    let claim_ids = match action {
        "create-shadow" | "create" => vec!["V6-AGENCY-001.1", "V6-AGENCY-001.5"],
        "topology" => vec!["V6-AGENCY-001.2", "V6-AGENCY-001.5"],
        "orchestrate" => vec!["V6-AGENCY-001.3", "V6-AGENCY-001.5"],
        "workflow-bind" => vec!["V6-AGENCY-001.4", "V6-AGENCY-001.5"],
        _ => vec!["V6-AGENCY-001.5"],
    };
    let bypass_requested = conduit_bypass_requested(&parsed.flags);
    build_plane_conduit_enforcement(
        root,
        STATE_ENV,
        STATE_SCOPE,
        strict,
        action,
        "agency_conduit_enforcement",
        "core/layer0/ops/agency_plane",
        bypass_requested,
        "agency_surface_routes_through_layer0_conduit_with_fail_closed_policy",
        &claim_ids,
    )
}

fn conduit_lifecycle_paths(root: &Path) -> (PathBuf, PathBuf) {
    let base = state_root(root).join("conduit");
    (base.join("lifecycle.json"), base.join("history.jsonl"))
}

fn load_conduit_lifecycle(root: &Path) -> Value {
    let (path, _) = conduit_lifecycle_paths(root);
    fs::read_to_string(&path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| {
            json!({
                "version": "v1",
                "state": "healthy",
                "transition_count": 0,
                "degraded_count": 0,
                "recovered_count": 0,
                "failed_closed_count": 0,
                "last_command": "status",
                "updated_at": crate::now_iso()
            })
        })
}

fn next_conduit_state(previous: &str, enforcement_ok: bool, strict: bool, bypass_requested: bool) -> &'static str {
    if !enforcement_ok {
        if strict && bypass_requested {
            "failed_closed"
        } else if strict {
            "quarantined"
        } else {
            "degraded"
        }
    } else if matches!(previous, "degraded" | "quarantined" | "failed_closed") {
        "reconnecting"
    } else {
        "healthy"
    }
}

fn record_conduit_lifecycle(root: &Path, command: &str, strict: bool, conduit: Option<&Value>) -> Value {
    let previous = load_conduit_lifecycle(root);
    let previous_state = clean(
        previous
            .get("state")
            .and_then(Value::as_str)
            .unwrap_or("healthy"),
        40,
    )
    .to_ascii_lowercase();
    let enforcement_ok = conduit
        .and_then(|row| row.get("ok"))
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let bypass_requested = conduit
        .and_then(|row| row.get("bypass_requested"))
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let mut state = next_conduit_state(&previous_state, enforcement_ok, strict, bypass_requested);
    if previous_state == "reconnecting" && enforcement_ok {
        state = "healthy";
    }
    let transitioned = previous_state != state;
    let transition_count = previous
        .get("transition_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        + if transitioned { 1 } else { 0 };
    let degraded_count = previous
        .get("degraded_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        + if state == "degraded" { 1 } else { 0 };
    let failed_closed_count = previous
        .get("failed_closed_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        + if state == "failed_closed" { 1 } else { 0 };
    let recovered_count = previous
        .get("recovered_count")
        .and_then(Value::as_u64)
        .unwrap_or(0)
        + if previous_state == "reconnecting" && state == "healthy" {
            1
        } else {
            0
        };

    let entry = json!({
        "version": "v1",
        "state": state,
        "previous_state": previous_state,
        "transitioned": transitioned,
        "transition_count": transition_count,
        "degraded_count": degraded_count,
        "recovered_count": recovered_count,
        "failed_closed_count": failed_closed_count,
        "strict": strict,
        "enforcement_ok": enforcement_ok,
        "bypass_requested": bypass_requested,
        "last_command": clean(command, 80),
        "updated_at": crate::now_iso(),
        "sequence": format!(
            "acl_{}",
            &sha256_hex_str(&format!(
                "{}:{}:{}:{}",
                command, state, transition_count, enforcement_ok
            ))[..16]
        )
    });
    let (lifecycle_path, history_path) = conduit_lifecycle_paths(root);
    let _ = write_json(&lifecycle_path, &entry);
    let _ = append_jsonl(&history_path, &entry);
    entry
}

fn validate_contract(
    contract: &Value,
    expected_kind: &str,
    version_error: &str,
    kind_error: &str,
    errors: &mut Vec<String>,
) {
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push(version_error.to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != expected_kind
    {
        errors.push(kind_error.to_string());
    }
}
