fn fail_closed_receipt(
    reason: impl Into<String>,
    policy_receipt_hash: impl Into<String>,
    security_receipt_hash: impl Into<String>,
) -> ValidationReceipt {
    let reason = reason.into();
    let policy_receipt_hash = policy_receipt_hash.into();
    let security_receipt_hash = security_receipt_hash.into();
    let payload = serde_json::json!({
        "ok": false,
        "fail_closed": true,
        "reason": reason,
        "policy_receipt_hash": policy_receipt_hash,
        "security_receipt_hash": security_receipt_hash,
    });
    ValidationReceipt {
        ok: false,
        fail_closed: true,
        reason,
        policy_receipt_hash,
        security_receipt_hash,
        receipt_hash: deterministic_receipt_hash(&payload),
    }
}

fn success_receipt(
    policy_receipt_hash: impl Into<String>,
    security_receipt_hash: impl Into<String>,
) -> ValidationReceipt {
    let policy_receipt_hash = policy_receipt_hash.into();
    let security_receipt_hash = security_receipt_hash.into();
    let payload = serde_json::json!({
        "ok": true,
        "fail_closed": false,
        "reason": "validated",
        "policy_receipt_hash": policy_receipt_hash,
        "security_receipt_hash": security_receipt_hash,
    });

    ValidationReceipt {
        ok: true,
        fail_closed: false,
        reason: "validated".to_string(),
        policy_receipt_hash,
        security_receipt_hash,
        receipt_hash: deterministic_receipt_hash(&payload),
    }
}

fn is_valid_sha256(raw: &str) -> bool {
    raw.len() == 64 && raw.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn is_valid_plugin_type(raw: &str) -> bool {
    matches!(
        raw,
        "cognition_reflex" | "substrate_adapter" | "memory_backend"
    )
}

pub fn process_command<P: PolicyGate, H: CommandHandler>(
    envelope: &CommandEnvelope,
    policy: &P,
    security: &mut ConduitSecurityContext,
    handler: &mut H,
) -> ResponseEnvelope {
    let validation = validate_command(envelope, policy, security);
    let command_type = command_type_name(&envelope.command);

    let event = if validation.ok {
        handler.handle(&envelope.command)
    } else {
        RustEvent::SystemFeedback {
            status: "policy_violation".to_string(),
            detail: serde_json::json!({"fail_closed": validation.fail_closed}),
            violation_reason: Some(validation.reason.clone()),
        }
    };

    let crossing = CrossingReceipt {
        crossing_id: envelope.request_id.clone(),
        direction: CrossingDirection::TsToRust,
        command_type: command_type.to_string(),
        deterministic_hash: deterministic_receipt_hash(envelope),
        ts_ms: now_ts_ms(),
    };

    let mut response = ResponseEnvelope {
        schema_id: CONDUIT_SCHEMA_ID.to_string(),
        schema_version: CONDUIT_SCHEMA_VERSION.to_string(),
        request_id: envelope.request_id.clone(),
        ts_ms: now_ts_ms(),
        event,
        validation,
        crossing,
        receipt_hash: String::new(),
    };
    response.receipt_hash = deterministic_receipt_hash(&response);
    response
}

pub fn run_stdio_once<R: BufRead, W: Write, P: PolicyGate, H: CommandHandler>(
    mut reader: R,
    writer: &mut W,
    policy: &P,
    security: &mut ConduitSecurityContext,
    handler: &mut H,
) -> io::Result<bool> {
    let mut line = String::new();
    let read = reader.read_line(&mut line)?;
    if read == 0 {
        return Ok(false);
    }

    let parsed = serde_json::from_str::<CommandEnvelope>(&line).map_err(invalid_data)?;
    let response = process_command(&parsed, policy, security, handler);
    serde_json::to_writer(&mut *writer, &response).map_err(invalid_data)?;
    writer.write_all(b"\n")?;
    writer.flush()?;
    Ok(true)
}

#[cfg(unix)]
pub fn run_unix_socket_server<P: AsRef<std::path::Path>, G: PolicyGate, H: CommandHandler>(
    socket_path: P,
    policy: &G,
    security: &mut ConduitSecurityContext,
    handler: &mut H,
) -> io::Result<()> {
    use std::io::BufReader;
    use std::os::unix::net::UnixListener;

    let path = socket_path.as_ref();
    if path.exists() {
        fs::remove_file(path)?;
    }

    let listener = UnixListener::bind(path)?;
    let (stream, _) = listener.accept()?;
    let read_stream = stream.try_clone()?;
    let mut reader = BufReader::new(read_stream);
    let mut writer = stream;

    while run_stdio_once(&mut reader, &mut writer, policy, security, handler)? {}
    Ok(())
}

fn invalid_data(err: impl fmt::Display) -> io::Error {
    io::Error::new(io::ErrorKind::InvalidData, err.to_string())
}

fn command_type_name(command: &TsCommand) -> &'static str {
    match command {
        TsCommand::StartAgent { .. } => "start_agent",
        TsCommand::StopAgent { .. } => "stop_agent",
        TsCommand::QueryReceiptChain { .. } => "query_receipt_chain",
        TsCommand::ListActiveAgents => "list_active_agents",
        TsCommand::GetSystemStatus => "get_system_status",
        TsCommand::ApplyPolicyUpdate { .. } => "apply_policy_update",
        TsCommand::InstallExtension { .. } => "install_extension",
    }
}

struct SigningPayload<'a> {
    schema_id: &'a str,
    schema_version: &'a str,
    request_id: &'a str,
    ts_ms: u64,
    command: &'a TsCommand,
    client_id: &'a str,
    key_id: &'a str,
    nonce: &'a str,
    capability_token: &'a CapabilityToken,
}

fn signing_payload(input: SigningPayload<'_>) -> Value {
    serde_json::json!({
        "schema_id": input.schema_id,
        "schema_version": input.schema_version,
        "request_id": input.request_id,
        "ts_ms": input.ts_ms,
        "command": input.command,
        "security": {
            "client_id": input.client_id,
            "key_id": input.key_id,
            "nonce": input.nonce,
            "capability_token": input.capability_token,
        }
    })
}

fn canonical_json<T: Serialize>(value: &T) -> String {
    let json = serde_json::to_value(value).expect("serialization must succeed");
    let normalized = normalize_value(json);
    serde_json::to_string(&normalized).expect("canonical serialization must succeed")
}

fn normalize_value(value: Value) -> Value {
    match value {
        Value::Array(rows) => Value::Array(rows.into_iter().map(normalize_value).collect()),
        Value::Object(map) => {
            let mut entries = map.into_iter().collect::<Vec<_>>();
            entries.sort_by(|(lhs, _), (rhs, _)| lhs.cmp(rhs));
            let mut out = Map::new();
            for (key, value) in entries {
                out.insert(key, normalize_value(value));
            }
            Value::Object(out)
        }
        other => other,
    }
}

fn now_ts_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_millis() as u64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    include!("part_003_tests_1167_parts/part_000.rs");
    include!("part_003_tests_1167_parts/part_001.rs");
    include!("part_003_tests_1167_parts/part_002.rs");
}
