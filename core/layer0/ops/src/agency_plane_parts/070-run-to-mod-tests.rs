
pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
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
    let conduit_lifecycle = if command == "status" {
        load_conduit_lifecycle(root)
    } else {
        record_conduit_lifecycle(root, &command, strict, conduit.as_ref())
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
                "type": "agency_plane_conduit_gate",
                "errors": ["conduit_bypass_rejected"],
                "conduit_enforcement": conduit,
                "conduit_lifecycle": conduit_lifecycle
            }),
        );
    }

    let mut payload = match command.as_str() {
        "status" => status(root),
        "create-shadow" | "create" => run_create_shadow(root, &parsed, strict),
        "topology" => run_topology(root, &parsed, strict),
        "orchestrate" => run_orchestrate(root, &parsed, strict),
        "workflow-bind" => run_workflow_bind(root, &parsed, strict),
        _ => json!({
            "ok": false,
            "type": "agency_plane_error",
            "error": "unknown_command",
            "command": command
        }),
    };
    payload["conduit_lifecycle"] = conduit_lifecycle;
    if command == "status" {
        crate::v8_kernel::print_json(&payload);
        return 0;
    }
    emit(root, attach_conduit(payload, conduit.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn conduit_rejects_bypass() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&["create".to_string(), "--bypass=1".to_string()]);
        let out = conduit_enforcement(root.path(), &parsed, true, "create-shadow");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }
}
