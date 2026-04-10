fn normalized_command(positional: &[String]) -> String {
    positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string())
}

fn is_help(command: &str) -> bool {
    matches!(command, "help" | "--help" | "-h")
}

fn should_fail_strict_gate(strict: bool, conduit: Option<&Value>) -> bool {
    strict && conduit.and_then(|v| v.get("ok")).and_then(Value::as_bool) == Some(false)
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = normalized_command(parsed.positional.as_slice());
    if is_help(command.as_str()) {
        usage();
        return 0;
    }
    let strict = parse_bool(parsed.flags.get("strict"), true);

    let conduit = if command != "status" {
        Some(conduit_enforcement(root, &parsed, strict, &command))
    } else {
        None
    };
    if should_fail_strict_gate(strict, conduit.as_ref()) {
        return emit(
            root,
            json!({
                "ok": false,
                "strict": strict,
                "type": "flow_plane_conduit_gate",
                "errors": ["conduit_bypass_rejected"],
                "conduit_enforcement": conduit
            }),
        );
    }

    let payload = match command.as_str() {
        "status" => status(root),
        "compile" | "build" => run_compile(root, &parsed, strict),
        "run" => {
            let mut alias = parsed.clone();
            alias.positional = vec!["playground".to_string()];
            alias
                .flags
                .entry("op".to_string())
                .or_insert_with(|| "play".to_string());
            run_playground(root, &alias, strict)
        }
        "playground" | "debug" => run_playground(root, &parsed, strict),
        "component-marketplace" | "component_marketplace" | "components" => {
            run_component_marketplace(root, &parsed, strict)
        }
        "export" | "package" => run_export(root, &parsed, strict),
        "install" => run_template_governance(root, &parsed, strict),
        "template-governance" | "template_governance" | "templates" => {
            run_template_governance(root, &parsed, strict)
        }
        _ => json!({
            "ok": false,
            "type": "flow_plane_error",
            "error": "unknown_command",
            "command": command
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
    fn compile_requires_canvas() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&["compile".to_string()]);
        let out = run_compile(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn conduit_rejects_bypass_when_strict() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&["compile".to_string(), "--bypass=1".to_string()]);
        let out = conduit_enforcement(root.path(), &parsed, true, "compile");
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }
}
