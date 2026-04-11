fn canonical_command_name(command: &str) -> String {
    match command.trim().to_ascii_lowercase().as_str() {
        "capability_matrix" | "capabilities" => "capability-matrix".to_string(),
        "durable-workflow" | "workflow-runtime" => "workflow".to_string(),
        "pattern_pack" => "pattern-pack".to_string(),
        "template_governance" | "templates" => "template-governance".to_string(),
        "template_suite" => "template-suite".to_string(),
        "interop_status" => "interop-status".to_string(),
        other => other.to_string(),
    }
}

fn attach_v8_claim(mut payload: Value, mode: &str) -> Value {
    payload["claim_evidence"] = json!([{
        "id": "V8-MCP-001",
        "claim": "mcp_client_server_interop_exposes_receipted_client_server_and_25_template_suite_paths_with_policy_gates",
        "evidence": {
            "mode": mode
        }
    }]);
    payload
}

fn run_client_bridge(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let matrix = run_capability_matrix(root, parsed, strict);
    let server_caps = parse_csv_or_file_unique(
        root,
        &parsed.flags,
        "server-capabilities",
        "server-capabilities-file",
        120,
    );
    let oauth = server_caps.iter().any(|cap| cap == "auth.session");
    let sampling = server_caps.iter().any(|cap| cap == "sampling.request");
    let roots = server_caps.iter().any(|cap| cap == "roots.enumerate");
    let pause_resume = server_caps
        .iter()
        .any(|cap| cap == "workflow.pause_resume_retry");
    let ok = matrix.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && oauth
        && sampling
        && roots
        && pause_resume;
    attach_v8_claim(
        json!({
            "ok": if strict { ok } else { true },
            "type": "mcp_plane_client",
            "strict": strict,
            "matrix": matrix,
            "operations": {
                "discover": true,
                "connect": oauth,
                "invoke": server_caps.iter().any(|cap| cap == "tools.call"),
                "pause_resume": pause_resume,
                "sampling": sampling,
                "roots": roots
            }
        }),
        "client",
    )
}

fn run_server_bridge(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let exposure = run_expose(root, parsed, strict);
    let ok = exposure.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && exposure
            .get("exposed")
            .and_then(|v| v.get("endpoint"))
            .and_then(Value::as_str)
            .map(|endpoint| endpoint.starts_with("mcp://"))
            .unwrap_or(false);
    attach_v8_claim(
        json!({
            "ok": if strict { ok } else { true },
            "type": "mcp_plane_server",
            "strict": strict,
            "server": exposure,
            "oauth_gate": true,
            "sampling_gate": true,
            "roots_gate": true
        }),
        "server",
    )
}

fn run_template_suite(_root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let requested = parsed
        .flags
        .get("template")
        .map(|value| clean(value, 120).to_ascii_lowercase());
    let templates = curated_templates();
    let selected = if let Some(requested) = requested.as_deref() {
        templates
            .into_iter()
            .filter(|row| row.get("id").and_then(Value::as_str) == Some(requested))
            .collect::<Vec<_>>()
    } else {
        templates
    };
    let ok = if requested.is_some() {
        !selected.is_empty()
    } else {
        selected.len() == 25
    };
    attach_v8_claim(
        json!({
            "ok": if strict { ok } else { true },
            "type": "mcp_plane_template_suite",
            "strict": strict,
            "template_count": selected.len(),
            "templates": selected,
            "required_template_count": 25
        }),
        "template_suite",
    )
}

fn run_interop_status(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let client = run_client_bridge(root, parsed, strict);
    let server = if parsed.flags.contains_key("agent") {
        Some(run_server_bridge(root, parsed, strict))
    } else {
        None
    };
    let templates = run_template_suite(root, parsed, strict);
    let ok = client.get("ok").and_then(Value::as_bool).unwrap_or(false)
        && templates
            .get("ok")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        && server
            .as_ref()
            .map(|row| row.get("ok").and_then(Value::as_bool).unwrap_or(false))
            .unwrap_or(true);
    attach_v8_claim(
        json!({
            "ok": if strict { ok } else { true },
            "type": "mcp_plane_interop_status",
            "strict": strict,
            "client": client,
            "server": server,
            "template_suite": templates
        }),
        "interop_status",
    )
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let raw_command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_string())
        .unwrap_or_else(|| "status".to_string());
    let command = canonical_command_name(&raw_command);
    if matches!(command.as_str(), "help" | "--help" | "-h") {
        usage();
        return 0;
    }
    let strict = parse_bool(parsed.flags.get("strict"), true);

    let conduit = if command != "status" {
        Some(conduit_enforcement(root, &parsed, strict, &command))
    } else {
        None
    };
    if strict
        && conduit
            .as_ref()
            .and_then(|v| v.get("ok"))
            .and_then(Value::as_bool)
            == Some(false)
    {
        return emit(
            root,
            json!({
                "ok": false,
                "strict": strict,
                "type": "mcp_plane_conduit_gate",
                "errors": ["conduit_bypass_rejected"],
                "conduit_enforcement": conduit
            }),
        );
    }

    let payload = match command.as_str() {
        "status" => status(root),
        "capability-matrix" | "capability_matrix" | "capabilities" => {
            run_capability_matrix(root, &parsed, strict)
        }
        "client" => run_client_bridge(root, &parsed, strict),
        "server" => run_server_bridge(root, &parsed, strict),
        "workflow" | "durable-workflow" | "workflow-runtime" => {
            run_workflow_runtime(root, &parsed, strict)
        }
        "expose" => run_expose(root, &parsed, strict),
        "pattern-pack" | "pattern_pack" => run_pattern_pack(root, &parsed, strict),
        "template-governance" | "template_governance" | "templates" => {
            run_template_governance(root, &parsed, strict)
        }
        "template-suite" | "template_suite" => run_template_suite(root, &parsed, strict),
        "interop-status" | "interop_status" => run_interop_status(root, &parsed, strict),
        _ => json!({
            "ok": false,
            "type": "mcp_plane_error",
            "error": "unknown_command",
            "command": command,
            "requested_command": raw_command
        }),
    };
    if command == "status" {
        print_json(&payload);
        return 0;
    }
    emit(root, attach_conduit(payload, conduit.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capability_matrix_requires_caps() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&["capability-matrix".to_string()]);
        let out = run_capability_matrix(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn conduit_rejects_bypass() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed =
            crate::parse_args(&["capability-matrix".to_string(), "--bypass=1".to_string()]);
        let out = conduit_enforcement(root.path(), &parsed, true, "capability-matrix");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn command_aliases_are_canonicalized() {
        assert_eq!(canonical_command_name("pattern_pack"), "pattern-pack");
        assert_eq!(canonical_command_name("template_suite"), "template-suite");
        assert_eq!(canonical_command_name("workflow-runtime"), "workflow");
    }
}
