fn run_parse_doc(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let parse_contract = load_json_or(
        root,
        PARSE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "mapping_rule_parser_contract",
            "supported_strategies": ["title", "between", "prefix_line", "constant", "contains"]
        }),
    );

    let mut errors = Vec::<String>::new();
    if parse_contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("parse_contract_version_must_be_v1".to_string());
    }
    if parse_contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "mapping_rule_parser_contract"
    {
        errors.push("parse_contract_kind_invalid".to_string());
    }

    let (source_path, source_raw) = match load_source(root, parsed) {
        Ok(ok) => ok,
        Err(err) => {
            errors.push(err);
            ("".to_string(), "".to_string())
        }
    };
    let (mapping_path, mapping) = match load_mapping(root, parsed) {
        Ok(ok) => ok,
        Err(err) => {
            errors.push(err);
            ("".to_string(), Value::Null)
        }
    };

    if mapping.is_null() {
        errors.push("mapping_missing".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "parse_plane_parse_doc",
            "errors": errors
        });
    }

    let mapping_version = mapping
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default();
    let mapping_kind = mapping
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default();
    if mapping_version != "v1" {
        errors.push("mapping_version_must_be_v1".to_string());
    }
    if mapping_kind != "mapping_rule_set" {
        errors.push("mapping_kind_invalid".to_string());
    }

    let rules = mapping
        .get("rules")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if rules.is_empty() {
        errors.push("mapping_rules_required".to_string());
    }

    let source_plain = strip_tags(&source_raw);
    let mut instructions = Vec::<Value>::new();
    let mut structured = Map::<String, Value>::new();
    let mut validation = Vec::<Value>::new();

    for (idx, rule) in rules.iter().enumerate() {
        let strategy = rule
            .get("strategy")
            .and_then(Value::as_str)
            .map(|v| clean(v, 80))
            .unwrap_or_else(|| "contains".to_string());
        instructions.push(json!({
            "index": idx,
            "strategy": strategy,
            "field": clean(rule.get("field").and_then(Value::as_str).unwrap_or("field"), 120)
        }));
        let (field, value, valid) = apply_rule(rule, &source_raw, &source_plain);
        let required = rule
            .get("required")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        validation.push(json!({"field": field, "required": required, "valid": valid}));
        structured.insert(field, value);
    }

    if strict
        && validation.iter().any(|row| {
            row.get("required")
                .and_then(Value::as_bool)
                .unwrap_or(false)
                && !row.get("valid").and_then(Value::as_bool).unwrap_or(false)
        })
    {
        errors.push("required_mapping_rule_validation_failed".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "parse_plane_parse_doc",
            "errors": errors,
            "validation": validation
        });
    }

    let stage_receipts = vec![
        json!({
            "stage": "source",
            "source_path": source_path,
            "source_sha256": sha256_hex_str(&source_raw),
            "length": source_raw.len()
        }),
        json!({
            "stage": "instructions",
            "mapping_path": mapping_path,
            "mapping_sha256": sha256_hex_str(&mapping.to_string()),
            "instruction_count": instructions.len()
        }),
        json!({
            "stage": "structured_dict",
            "field_count": structured.len(),
            "structured_sha256": sha256_hex_str(&Value::Object(structured.clone()).to_string())
        }),
    ];

    let artifact = json!({
        "source_path": source_path,
        "mapping_path": mapping_path,
        "pipeline": {
            "source": {
                "raw_sha256": sha256_hex_str(&source_raw),
                "plain_preview": clean(&source_plain, 300)
            },
            "instructions": instructions,
            "structured": Value::Object(structured.clone())
        },
        "validation": validation,
        "stage_receipts": stage_receipts
    });
    let artifact_path = state_root(root).join("parse_doc").join("latest.json");
    let _ = write_json(&artifact_path, &artifact);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "parse_plane_parse_doc",
        "lane": "core/layer0/ops",
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&artifact.to_string())
        },
        "pipeline": artifact.get("pipeline").cloned().unwrap_or(Value::Null),
        "validation": validation,
        "stage_receipts": stage_receipts,
        "claim_evidence": [
            {
                "id": "V6-PARSE-001.1",
                "claim": "versioned_mapping_rule_parser_runs_with_policy_scoped_load_and_deterministic_receipts",
                "evidence": {
                    "mapping_path": mapping_path,
                    "rule_count": rules.len()
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_visualize(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let vis_contract = load_json_or(
        root,
        VISUALIZE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "parse_instruction_pipeline_contract",
            "pipeline_order": ["source", "instructions", "structured_dict"]
        }),
    );
    let mut errors = Vec::<String>::new();
    if vis_contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("visualize_contract_version_must_be_v1".to_string());
    }
    if vis_contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "parse_instruction_pipeline_contract"
    {
        errors.push("visualize_contract_kind_invalid".to_string());
    }

    let from_path = parsed.flags.get("from-path").cloned().unwrap_or_else(|| {
        state_root(root)
            .join("parse_doc")
            .join("latest.json")
            .display()
            .to_string()
    });
    let from = if Path::new(&from_path).is_absolute() {
        PathBuf::from(&from_path)
    } else {
        root.join(&from_path)
    };
    let artifact = read_json(&from).unwrap_or(Value::Null);
    if artifact.is_null() {
        errors.push(format!("parse_artifact_missing:{}", from.display()));
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "parse_plane_visualize",
            "errors": errors
        });
    }

    let stage_order = vis_contract
        .get("pipeline_order")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_else(|| {
            vec![
                json!("source"),
                json!("instructions"),
                json!("structured_dict"),
            ]
        })
        .iter()
        .filter_map(Value::as_str)
        .map(|v| clean(v, 120))
        .collect::<Vec<_>>();

    let diagram = format!(
        "source -> instructions -> structured_dict\norder={}",
        stage_order.join(" -> ")
    );
    let stage_receipts = vec![
        json!({
            "stage": "source",
            "artifact_path": from.display().to_string(),
            "artifact_sha256": sha256_hex_str(&artifact.to_string())
        }),
        json!({
            "stage": "visualization",
            "diagram_sha256": sha256_hex_str(&diagram)
        }),
    ];

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "parse_plane_visualize",
        "lane": "core/layer0/ops",
        "visualization": {
            "diagram": diagram,
            "stage_order": stage_order
        },
        "source_artifact": from.display().to_string(),
        "source_pipeline": artifact.get("pipeline").cloned().unwrap_or(Value::Null),
        "stage_receipts": stage_receipts,
        "claim_evidence": [
            {
                "id": "V6-PARSE-001.2",
                "claim": "instruction_stage_pipeline_is_inspectable_and_visualizable_with_deterministic_receipts",
                "evidence": {
                    "from_path": from.display().to_string(),
                    "stage_count": 3
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn parse_table_input(
    root: &Path,
    parsed: &crate::ParsedArgs,
) -> Result<(String, Vec<Vec<String>>), String> {
    if let Some(raw) = parsed.flags.get("table-json") {
        let parsed_value: Value =
            serde_json::from_str(raw).map_err(|_| "table_json_invalid".to_string())?;
        if let Some(table) = value_to_table(&parsed_value) {
            return Ok(("table-json".to_string(), table));
        }
        return Err("table_json_invalid_shape".to_string());
    }
    if let Some(rel_or_abs) = parsed.flags.get("table-path") {
        let path = if Path::new(rel_or_abs).is_absolute() {
            PathBuf::from(rel_or_abs)
        } else {
            root.join(rel_or_abs)
        };
        let raw = fs::read_to_string(&path)
            .map_err(|_| format!("table_path_not_found:{}", path.display()))?;
        let parsed_value: Value =
            serde_json::from_str(&raw).map_err(|_| "table_path_json_invalid".to_string())?;
        if let Some(table) = value_to_table(&parsed_value) {
            return Ok((path.display().to_string(), table));
        }
        return Err("table_path_invalid_shape".to_string());
    }

    let from_path = parsed.flags.get("from-path").cloned().unwrap_or_else(|| {
        state_root(root)
            .join("parse_doc")
            .join("latest.json")
            .display()
            .to_string()
    });
    let path = if Path::new(&from_path).is_absolute() {
        PathBuf::from(&from_path)
    } else {
        root.join(&from_path)
    };
    let artifact =
        read_json(&path).ok_or_else(|| format!("from_path_not_found:{}", path.display()))?;
    let structured = artifact
        .get("pipeline")
        .and_then(|v| v.get("structured"))
        .cloned()
        .unwrap_or(Value::Null);
    let candidate = structured
        .get("table")
        .cloned()
        .or_else(|| {
            structured
                .get("tables")
                .and_then(Value::as_array)
                .and_then(|rows| rows.first())
                .cloned()
        })
        .or_else(|| structured.get("rows").cloned())
        .unwrap_or(structured);
    if let Some(table) = value_to_table(&candidate) {
        return Ok((path.display().to_string(), table));
    }
    Err("table_unavailable_in_artifact".to_string())
}

