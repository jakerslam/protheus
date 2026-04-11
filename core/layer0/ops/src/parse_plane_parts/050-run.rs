fn run_export(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let format = parsed
        .flags
        .get("format")
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "json".to_string());
    let extension = match format.as_str() {
        "json" => "json",
        "jsonl" => "jsonl",
        "md" | "markdown" => "md",
        _ => {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "parse_plane_export_error",
                "errors": ["invalid_export_format"],
                "format": clean(&format, 24)
            });
        }
    };

    let source_path = parsed
        .flags
        .get("from-path")
        .map(|raw| resolve_plane_path(root, raw))
        .unwrap_or_else(|| latest_path(root));
    let source_value = match read_json(&source_path) {
        Some(v) => v,
        None => {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "parse_plane_export_error",
                "errors": ["export_source_missing_or_invalid_json"],
                "source_path": source_path.display().to_string()
            });
        }
    };

    let output_path = parsed
        .flags
        .get("output-path")
        .or_else(|| parsed.flags.get("out-path"))
        .map(|raw| resolve_plane_path(root, raw))
        .unwrap_or_else(|| {
            state_root(root)
                .join("exports")
                .join(format!("latest.{extension}"))
        });

    let canonical = canonicalize_json(&source_value);
    let body = match extension {
        "json" => {
            let mut out = serde_json::to_string_pretty(&canonical)
                .unwrap_or_else(|_| canonical_json_string(&canonical));
            out.push('\n');
            out
        }
        "jsonl" => {
            let mut out = canonical_json_string(&canonical);
            out.push('\n');
            out
        }
        "md" => {
            format!(
                "# Parse Export\n\n```json\n{}\n```\n",
                serde_json::to_string_pretty(&canonical)
                    .unwrap_or_else(|_| canonical_json_string(&canonical))
            )
        }
        _ => String::new(),
    };

    if let Some(parent) = output_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Err(err) = fs::write(&output_path, body.as_bytes()) {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "parse_plane_export_error",
            "errors": ["export_write_failed"],
            "path": output_path.display().to_string(),
            "error": clean(err.to_string(), 220)
        });
    }

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "parse_plane_export",
        "lane": "core/layer0/ops",
        "source_path": source_path.display().to_string(),
        "output_path": output_path.display().to_string(),
        "format": extension,
        "artifact": {
            "path": output_path.display().to_string(),
            "sha256": sha256_hex_str(&body)
        },
        "claim_evidence": [
            {
                "id": "V6-PARSE-001.6",
                "claim": "parse_export_actions_route_through_conduit_with_fail_closed_policy_checks_and_deterministic_receipts",
                "evidence": {
                    "source_path": source_path.display().to_string(),
                    "output_path": output_path.display().to_string(),
                    "format": extension
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

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
                "type": "parse_plane_conduit_gate",
                "errors": ["conduit_bypass_rejected"],
                "conduit_enforcement": conduit
            }),
        );
    }

    let payload = match command.as_str() {
        "status" => status(root),
        "parse-doc" | "parse_doc" | "doc" => run_parse_doc(root, &parsed, strict),
        "visualize" | "viz" => run_visualize(root, &parsed, strict),
        "postprocess-table" | "postprocess_table" | "postprocess" => {
            run_postprocess_table(root, &parsed, strict)
        }
        "flatten" | "unnest" => run_flatten_transform(root, &parsed, strict),
        "export" => run_export(root, &parsed, strict),
        "template-governance" | "template_governance" | "templates" => {
            run_template_governance(root, &parsed, strict)
        }
        _ => json!({
            "ok": false,
            "type": "parse_plane_error",
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
    fn parse_doc_requires_source() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&["parse-doc".to_string(), "--mapping=default".to_string()]);
        let out = run_parse_doc(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert!(out
            .get("errors")
            .and_then(Value::as_array)
            .map(|rows| rows
                .iter()
                .any(|row| row.as_str() == Some("missing_source")))
            .unwrap_or(false));
    }

    #[test]
    fn conduit_rejects_bypass_when_strict() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = crate::parse_args(&["parse-doc".to_string(), "--bypass=1".to_string()]);
        let gate = conduit_enforcement(root.path(), &parsed, true, "parse-doc");
        assert_eq!(gate.get("ok").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn export_resolves_relative_paths_against_root() {
        let root = tempfile::tempdir().expect("tempdir");
        let source_path = root.path().join("fixtures").join("source.json");
        std::fs::create_dir_all(source_path.parent().expect("source parent")).expect("mkdir");
        std::fs::write(&source_path, "{\n  \"ok\": true,\n  \"type\": \"parse_plane_status\"\n}\n")
            .expect("write source");
        let parsed = crate::parse_args(&[
            "export".to_string(),
            "--from-path=fixtures/source.json".to_string(),
            "--output-path=artifacts/out.json".to_string(),
            "--format=json".to_string(),
        ]);
        let out = run_export(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        let source_display = source_path.display().to_string();
        assert_eq!(
            out.get("source_path").and_then(Value::as_str),
            Some(source_display.as_str())
        );
        let output_path = root.path().join("artifacts").join("out.json");
        let output_display = output_path.display().to_string();
        assert!(output_path.exists());
        assert_eq!(
            out.get("output_path").and_then(Value::as_str),
            Some(output_display.as_str())
        );
    }
}
