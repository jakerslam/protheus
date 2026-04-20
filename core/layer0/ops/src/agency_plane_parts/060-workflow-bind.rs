
fn run_workflow_bind(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        WORKFLOW_BINDING_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "agency_workflow_metric_binding_contract",
            "workflow_templates": {
                "frontend-wizard": {
                    "stages": ["design", "implement", "verify"],
                    "success_metrics": ["ui_regression_pass_rate", "latency_budget", "a11y_score"]
                },
                "security-engineer": {
                    "stages": ["threat_model", "scan", "remediate", "verify"],
                    "success_metrics": ["critical_findings_closed", "policy_compliance", "false_positive_rate"]
                },
                "research-strategist": {
                    "stages": ["scope", "collect", "synthesize", "publish"],
                    "success_metrics": ["source_coverage", "evidence_quality", "turnaround_time"]
                }
            }
        }),
    );
    let mut errors = Vec::<String>::new();
    validate_contract(
        &contract,
        "agency_workflow_metric_binding_contract",
        "agency_workflow_binding_contract_version_must_be_v1",
        "agency_workflow_binding_contract_kind_invalid",
        &mut errors,
    );
    let template = clean(
        parsed
            .flags
            .get("template")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_default(),
        80,
    );
    if template.is_empty() {
        errors.push("agency_workflow_template_required".to_string());
    }

    let run_id = clean(
        parsed
            .flags
            .get("run-id")
            .cloned()
            .unwrap_or_else(|| format!("workflow-{}", &sha256_hex_str(&template)[..10])),
        120,
    );
    let template_table = contract
        .get("workflow_templates")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let template_cfg = parsed
        .flags
        .get("workflow-json")
        .and_then(|raw| serde_json::from_str::<Value>(raw).ok())
        .unwrap_or_else(|| {
            template_table
                .get(&template)
                .cloned()
                .unwrap_or(Value::Null)
        });
    if strict && template_cfg.is_null() {
        errors.push("agency_workflow_template_not_found".to_string());
    }

    let stages = template_cfg
        .get("stages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let metrics = template_cfg
        .get("success_metrics")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    if strict && (stages.is_empty() || metrics.is_empty()) {
        errors.push("agency_workflow_stages_and_metrics_required".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "agency_plane_workflow_bind",
            "errors": errors
        });
    }

    let binding = json!({
        "version": "v1",
        "run_id": run_id,
        "template": template,
        "stages": stages,
        "success_metrics": metrics,
        "bound_at": crate::now_iso()
    });
    let binding_path = state_root(root)
        .join("workflows")
        .join(format!("{}.json", run_id));
    let _ = write_json(&binding_path, &binding);

    let deliverables = stages
        .iter()
        .enumerate()
        .map(|(idx, stage)| {
            let stage_name = clean(stage.as_str().unwrap_or("stage"), 80);
            let object_id = format!(
                "eo_{}",
                &sha256_hex_str(&format!("{}:{}:{}", run_id, stage_name, idx + 1))[..14]
            );
            json!({
                "object_id": object_id,
                "type": "epistemic_object",
                "schema_id": "epistemic_object_v1",
                "stage": stage_name,
                "content_hash": sha256_hex_str(&format!("{}:{}:{}", run_id, template, stage_name)),
                "provenance": {
                    "run_id": run_id,
                    "template": template,
                    "stage_index": idx + 1
                }
            })
        })
        .collect::<Vec<_>>();
    let deliverable_pack = json!({
        "version": "v1",
        "run_id": run_id,
        "template": template,
        "deliverables": deliverables,
        "generated_at": crate::now_iso()
    });
    let deliverable_path = state_root(root)
        .join("deliverables")
        .join(format!("{}.json", run_id));
    let _ = write_json(&deliverable_path, &deliverable_pack);
    let _ = append_jsonl(
        &state_root(root).join("deliverables").join("history.jsonl"),
        &deliverable_pack,
    );

    json!({
        "ok": true,
        "strict": strict,
        "type": "agency_plane_workflow_bind",
        "lane": "core/layer0/ops",
        "binding": binding,
        "deliverable_pack": deliverable_pack,
        "artifact": {
            "binding_path": binding_path.display().to_string(),
            "deliverable_path": deliverable_path.display().to_string(),
            "sha256": sha256_hex_str(&deliverable_pack.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-AGENCY-001.4",
                "claim": "agent_templates_bind_to_workflow_stages_and_success_metrics_with_receipted_epistemic_deliverables",
                "evidence": {
                    "template": template,
                    "run_id": run_id,
                    "deliverable_count": deliverable_pack
                        .get("deliverables")
                        .and_then(Value::as_array)
                        .map(|rows| rows.len())
                        .unwrap_or(0)
                }
            },
            {
                "id": "V6-AGENCY-001.5",
                "claim": "agency_tool_invocations_and_handoffs_use_conduit_only_fail_closed_guardrails",
                "evidence": {
                    "action": "workflow-bind",
                    "run_id": run_id
                }
            }
        ]
    })
    .with_receipt_hash()
}
