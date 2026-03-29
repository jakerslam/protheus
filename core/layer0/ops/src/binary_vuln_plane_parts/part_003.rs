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
                "type": "binary_vuln_plane_conduit_gate",
                "errors": ["conduit_bypass_rejected"],
                "conduit_enforcement": conduit
            }),
        );
    }

    let payload = match command.as_str() {
        "status" => status(root),
        "scan" => run_scan(root, &parsed, strict),
        "mcp-analyze" | "mcp_analyze" | "mcp" => run_mcp_analyze(root, &parsed, strict),
        "rulepack-install" | "rulepack_install" => run_rulepack_install(root, &parsed, strict),
        "rulepack-enable" | "rulepack_enable" => run_rulepack_enable(root, &parsed, strict),
        _ => json!({
            "ok": false,
            "type": "binary_vuln_plane_error",
            "error": "unknown_command",
            "command": command
        }),
    };
    if command == "status" {
        print_payload(&payload);
        return 0;
    }
    emit(root, attach_conduit(payload, conduit.as_ref()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entropy_is_zero_for_empty() {
        assert_eq!(shannon_entropy(&[]), 0.0);
    }

    #[test]
    fn detect_input_kind_defaults_binary() {
        let path = PathBuf::from("sample.unknown");
        assert_eq!(detect_input_kind(&path), "binary");
    }

    #[test]
    fn conduit_rejects_bypass() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&["scan".to_string(), "--bypass=1".to_string()]);
        let out = conduit_enforcement(root.path(), &parsed, true, "scan");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }
}

