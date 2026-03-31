fn run_industrial_pack(root: &Path, parsed: &ParsedArgs, strict: bool) -> Value {
    let isa95_rel = parsed
        .flags
        .get("isa95")
        .map(String::as_str)
        .unwrap_or(INDUSTRIAL_ISA95_PATH);
    let rami_rel = parsed
        .flags
        .get("rami")
        .map(String::as_str)
        .unwrap_or(INDUSTRIAL_RAMI_PATH);
    let checklist_rel = parsed
        .flags
        .get("checklist")
        .map(String::as_str)
        .unwrap_or(INDUSTRIAL_CHECKLIST_PATH);
    let isa95 = load_json_or(root, isa95_rel, Value::Null);
    let rami = load_json_or(root, rami_rel, Value::Null);
    let checklist = load_json_or(root, checklist_rel, Value::Null);
    let mut errors = Vec::<String>::new();

    if isa95.is_null() {
        errors.push("isa95_template_missing".to_string());
    }
    if rami.is_null() {
        errors.push("rami_template_missing".to_string());
    }
    if checklist.is_null() {
        errors.push("industrial_checklist_missing".to_string());
    }
    if isa95
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "isa95_mapping_template"
    {
        errors.push("isa95_kind_invalid".to_string());
    }
    if rami.get("kind").and_then(Value::as_str).unwrap_or_default() != "rami40_mapping_template" {
        errors.push("rami_kind_invalid".to_string());
    }
    if checklist
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "industrial_validation_checklist"
    {
        errors.push("industrial_checklist_kind_invalid".to_string());
    }
    if isa95
        .get("levels")
        .and_then(Value::as_array)
        .map(|rows| rows.len() >= 5)
        .unwrap_or(false)
        == false
    {
        errors.push("isa95_levels_incomplete".to_string());
    }
    if rami
        .get("axes")
        .and_then(Value::as_array)
        .map(|rows| rows.len() >= 3)
        .unwrap_or(false)
        == false
    {
        errors.push("rami_axes_incomplete".to_string());
    }
    if checklist
        .get("required_checks")
        .and_then(Value::as_array)
        .map(|rows| !rows.is_empty())
        .unwrap_or(false)
        == false
    {
        errors.push("industrial_required_checks_missing".to_string());
    }

    let ok = errors.is_empty();
    json!({
        "ok": if strict { ok } else { true },
        "strict": strict,
        "type": "asm_industrial_pack",
        "lane": "core/layer0/ops",
        "isa95_path": isa95_rel,
        "rami_path": rami_rel,
        "checklist_path": checklist_rel,
        "isa95_levels": isa95.get("levels").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0),
        "rami_axes": rami.get("axes").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0),
        "required_checks": checklist.get("required_checks").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0),
        "errors": errors,
        "claim_evidence": [
            {
                "id": "V7-ASM-010",
                "claim": "industrial_templates_map_inf_ring_primitives_to_isa95_and_rami",
                "evidence": {
                    "isa95_levels": isa95.get("levels").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0),
                    "rami_axes": rami.get("axes").and_then(Value::as_array).map(|v| v.len()).unwrap_or(0)
                }
            }
        ]
    })
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
    let payload = match command.as_str() {
        "status" => run_status(root),
        "wasm-dual-meter" | "wasm_dual_meter" => run_wasm_dual_meter(root, &parsed, strict),
        "hands-runtime" | "hands_runtime" => run_hands_runtime(root, &parsed, strict),
        "crdt-adapter" | "crdt_adapter" => run_crdt_adapter(root, &parsed, strict),
        "trust-chain" | "trust_chain" => run_trust_chain(root, &parsed, strict),
        "fastpath" => run_fastpath(root, &parsed, strict),
        "industrial-pack" | "industrial_pack" => run_industrial_pack(root, &parsed, strict),
        _ => json!({
            "ok": false,
            "type": "asm_plane_error",
            "error": "unknown_command",
            "command": command
        }),
    };
    if command == "status" {
        print_payload(&payload);
        return 0;
    }
    emit(root, payload)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dual_meter_fails_when_budget_exhausted() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = parse_args(&[
            "wasm-dual-meter".to_string(),
            "--ticks=100".to_string(),
            "--fuel-budget=100".to_string(),
            "--epoch-budget=1".to_string(),
            "--fuel-per-tick=20".to_string(),
        ]);
        let out = run_wasm_dual_meter(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }

    #[test]
    fn crdt_merge_produces_deterministic_winner() {
        let left = serde_json::from_str::<Value>(
            "{\"topic\":{\"value\":\"alpha\",\"clock\":1,\"node\":\"a\"}}",
        )
        .expect("left")
        .as_object()
        .cloned()
        .expect("left obj");
        let right = serde_json::from_str::<Value>(
            "{\"topic\":{\"value\":\"beta\",\"clock\":2,\"node\":\"b\"}}",
        )
        .expect("right")
        .as_object()
        .cloned()
        .expect("right obj");
        let (merged, _) = merge_crdt(&left, &right);
        assert_eq!(
            merged
                .get("topic")
                .and_then(|v| v.get("value"))
                .and_then(Value::as_str),
            Some("beta")
        );
    }

    #[test]
    fn fastpath_detects_mismatch() {
        let root = tempfile::tempdir().expect("tempdir");
        let parsed = parse_args(&[
            "fastpath".to_string(),
            "--inject-mismatch=1".to_string(),
            "--workload=1,2,3".to_string(),
        ]);
        let out = run_fastpath(root.path(), &parsed, true);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }
}

