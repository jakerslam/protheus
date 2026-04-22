
fn authorize_stomach_command_with_nexus_inner(
    command: &str,
    force_block_pair: bool,
) -> Result<Value, String> {
    let mut policy = DefaultNexusPolicy::default();
    if force_block_pair {
        policy.block_pair("client_ingress", "stomach");
    }
    let mut nexus = MainNexusControlPlane::new(
        NexusFeatureFlags {
            hierarchical_nexus_enabled: true,
            coexist_with_flat_routing: true,
        },
        policy,
    );
    let _ = nexus.register_v1_adapters("stomach_kernel")?;
    let schema = format!("stomach.kernel.{}", clean(command, 64));
    let lease = nexus.issue_route_lease(
        "stomach_kernel",
        LeaseIssueRequest {
            source: "client_ingress".to_string(),
            target: "stomach".to_string(),
            schema_ids: vec![schema.clone()],
            verbs: vec!["invoke".to_string()],
            required_verity: VerityClass::Standard,
            trust_class: TrustClass::InterModuleData,
            requested_ttl_ms: 30_000,
            template_id: None,
            template_version: None,
        },
    )?;
    let delivery = nexus.authorize_direct_delivery(
        "stomach_kernel",
        DeliveryAuthorizationInput {
            lease_id: Some(lease.lease_id.clone()),
            source: "client_ingress".to_string(),
            target: "stomach".to_string(),
            schema_id: schema,
            verb: "invoke".to_string(),
            offered_verity: VerityClass::Standard,
            now_ms: None,
        },
    );
    if !delivery.allowed {
        return Err(format!(
            "stomach_nexus_delivery_denied:{}",
            clean(delivery.reason.as_str(), 200)
        ));
    }
    let receipt_ids = nexus
        .receipts()
        .iter()
        .map(|row| row.receipt_id.clone())
        .collect::<Vec<_>>();
    Ok(json!({
      "enabled": true,
      "route": {"source":"client_ingress","target":"stomach","verb":"invoke"},
      "lease_id": lease.lease_id,
      "delivery": delivery,
      "metrics": nexus.metrics(),
      "receipt_ids": receipt_ids
    }))
}

fn authorize_stomach_command_with_nexus(command: &str) -> Result<Value, String> {
    authorize_stomach_command_with_nexus_inner(command, nexus_force_block_pair_enabled())
}

fn print_json_line(value: &Value) {
    crate::contract_lane_utils::print_json_line(value);
}

fn receipt_envelope(kind: &str, ok: bool) -> Value {
    let ts = now_iso();
    json!({
        "ok": ok,
        "type": kind,
        "ts": ts,
        "date": ts[..10].to_string()
    })
}

fn json_error(kind: &str, error: &str) -> Value {
    let mut out = receipt_envelope(kind, false);
    out["error"] = Value::String(error.to_string());
    out["fail_closed"] = Value::Bool(true);
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn json_receipt(kind: &str, payload: Value) -> Value {
    let mut out = receipt_envelope(kind, true);
    out["payload"] = payload;
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn stomach_state_root(root: &Path) -> PathBuf {
    root.join("local").join("state").join("stomach")
}

fn ensure_state_dirs(state_root: &Path) -> Result<(), String> {
    for rel in [
        "quarantine",
        "fetch",
        "snapshots",
        "provenance",
        "analysis",
        "proposals",
        "state",
    ] {
        fs::create_dir_all(state_root.join(rel))
            .map_err(|e| format!("stomach_state_dir_create_failed:{rel}:{e}"))?;
    }
    Ok(())
}

fn write_json(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("stomach_write_parent_create_failed:{e}"))?;
    }
    let encoded = serde_json::to_string_pretty(value)
        .map_err(|e| format!("stomach_write_encode_failed:{e}"))?;
    fs::write(path, format!("{encoded}\n")).map_err(|e| format!("stomach_write_failed:{e}"))
}

fn read_json(path: &Path) -> Result<Value, String> {
    let raw = fs::read_to_string(path).map_err(|e| format!("stomach_read_failed:{e}"))?;
    serde_json::from_str(&raw).map_err(|e| format!("stomach_decode_failed:{e}"))
}

fn append_jsonl(path: &Path, value: &Value) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .map_err(|e| format!("stomach_jsonl_parent_create_failed:{e}"))?;
    }
    let line =
        serde_json::to_string(value).map_err(|e| format!("stomach_jsonl_encode_failed:{e}"))?;
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| format!("stomach_jsonl_open_failed:{e}"))?;
    writeln!(file, "{line}").map_err(|e| format!("stomach_jsonl_write_failed:{e}"))
}

fn parse_transform(argv: &[String]) -> TransformRequest {
    let transform = parse_flag(argv, "transform").unwrap_or_else(|| "header_injection".to_string());
    let targets = csv_list(parse_flag(argv, "targets"));
    match transform.to_ascii_lowercase().as_str() {
        "namespace_fix" => TransformRequest {
            kind: TransformKind::NamespaceFix,
            target_paths: targets,
            namespace_from: parse_flag(argv, "namespace-from"),
            namespace_to: parse_flag(argv, "namespace-to"),
            header_text: None,
            path_prefix_from: None,
            path_prefix_to: None,
            adapter_name: None,
        },
        "path_remap" => TransformRequest {
            kind: TransformKind::PathRemap,
            target_paths: targets,
            namespace_from: None,
            namespace_to: None,
            header_text: None,
            path_prefix_from: parse_flag(argv, "path-from"),
            path_prefix_to: parse_flag(argv, "path-to"),
            adapter_name: None,
        },
        "adapter_scaffold" => TransformRequest {
            kind: TransformKind::AdapterScaffold,
            target_paths: targets,
            namespace_from: None,
            namespace_to: None,
            header_text: None,
            path_prefix_from: None,
            path_prefix_to: None,
            adapter_name: parse_flag(argv, "adapter-name"),
        },
        _ => TransformRequest::header_injection(
            targets,
            parse_flag(argv, "header").unwrap_or_else(|| "// staged by stomach".to_string()),
        ),
    }
}
