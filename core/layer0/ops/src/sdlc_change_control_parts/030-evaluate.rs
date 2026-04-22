
fn evaluate(root: &Path, policy: &Policy, pr_body_path: &Path, changed_paths_path: &Path) -> Value {
    let pr_body = fs::read_to_string(pr_body_path).unwrap_or_default();
    let fields = parse_pr_body_fields(&pr_body);
    let changed_paths = load_changed_paths(changed_paths_path);

    let inferred = infer_risk_class(&changed_paths, policy);
    let declared = RiskClass::parse(&fields.risk_class_raw).unwrap_or(RiskClass::Standard);

    let mut checks = BTreeMap::<String, Value>::new();
    let declared_valid = RiskClass::parse(&fields.risk_class_raw).is_some();
    insert_check(
        &mut checks,
        "declared_risk_class_valid",
        json!({
            "ok": declared_valid,
            "declared": fields.risk_class_raw,
            "allowed": ["standard", "major", "high-risk"]
        }),
    );
    insert_check(
        &mut checks,
        "declared_not_understated",
        json!({
            "ok": declared >= inferred,
            "declared": declared.as_str(),
            "inferred": inferred.as_str()
        }),
    );

    let rollback_plan_ok = insert_presence_check(
        &mut checks,
        "rollback_plan_present",
        &fields.rollback_plan,
    );
    let rollback_owner_ok = insert_presence_check(
        &mut checks,
        "rollback_owner_present",
        &fields.rollback_owner,
    );

    let require_rfc = declared >= RiskClass::Major && policy.require_rfc_for_major;
    insert_required_ref_check(
        &mut checks,
        "rfc_link_requirement",
        root,
        require_rfc,
        &fields.rfc_link,
    );

    let require_adr = declared == RiskClass::HighRisk && policy.require_adr_for_high_risk;
    insert_required_ref_check(
        &mut checks,
        "adr_link_requirement",
        root,
        require_adr,
        &fields.adr_link,
    );

    let approver_req = if declared == RiskClass::HighRisk {
        policy.required_approvers_high_risk
    } else if declared == RiskClass::Major {
        policy.required_approvers_major
    } else {
        0
    };
    insert_approver_check(&mut checks, approver_req, &fields.approvers);

    let require_approval_receipts =
        declared >= RiskClass::Major && policy.require_approval_receipts_for_major;
    let approval_receipts_ok = insert_approval_receipts_check(
        &mut checks,
        root,
        require_approval_receipts,
        &fields.approval_receipts,
    );

    let require_rollback_drill =
        declared == RiskClass::HighRisk && policy.require_rollback_drill_for_high_risk;
    let rollback_drill_ok = insert_required_ref_check(
        &mut checks,
        "rollback_drill_requirement",
        root,
        require_rollback_drill,
        &fields.rollback_drill_receipt,
    );

    let blocking_checks = checks
        .iter()
        .filter_map(|(k, v)| {
            if v.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                None
            } else {
                Some(k.clone())
            }
        })
        .collect::<Vec<_>>();

    let ok = blocking_checks.is_empty();

    json!({
        "ok": ok,
        "type": "sdlc_change_control_run",
        "schema_id": "sdlc_change_control",
        "schema_version": "1.0",
        "lane": LANE_ID,
        "ts": now_iso(),
        "declared_risk_class": declared.as_str(),
        "inferred_risk_class": inferred.as_str(),
        "checks": checks,
        "blocking_checks": blocking_checks,
        "inputs": {
            "pr_body_path": pr_body_path,
            "changed_paths_path": changed_paths_path,
            "changed_paths_count": changed_paths.len()
        },
        "claim_evidence": [
            {
                "id": "sdlc_change_class_enforcement",
                "claim": "risk_classes_enforce_rfc_adr_approvals_and_rollback_ownership",
                "evidence": {
                    "declared": declared.as_str(),
                    "inferred": inferred.as_str(),
                    "approver_requirement": approver_req,
                    "approver_count": fields.approvers.len(),
                    "rollback_owner_present": rollback_owner_ok,
                    "rollback_plan_present": rollback_plan_ok
                }
            },
            {
                "id": "sdlc_high_risk_merge_gate",
                "claim": "high_risk_changes_fail_closed_without_approval_receipts_and_rollback_drill_evidence",
                "evidence": {
                    "high_risk": declared == RiskClass::HighRisk,
                    "approval_receipts_ok": approval_receipts_ok,
                    "rollback_drill_ok": rollback_drill_ok
                }
            }
        ]
    })
}

fn run_cmd(
    root: &Path,
    policy: &Policy,
    strict: bool,
    pr_body_path: &Path,
    changed_paths_path: &Path,
) -> Result<(Value, i32), String> {
    let mut payload = evaluate(root, policy, pr_body_path, changed_paths_path);
    payload["strict"] = Value::Bool(strict);
    payload["policy_path"] = Value::String(policy.policy_path.to_string_lossy().to_string());
    payload["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&payload));

    write_text_atomic(
        &policy.latest_path,
        &format!(
            "{}\n",
            serde_json::to_string_pretty(&payload)
                .map_err(|e| format!("encode_latest_failed:{e}"))?
        ),
    )?;
    append_jsonl(&policy.history_path, &payload)?;

    let code = if strict && !payload.get("ok").and_then(Value::as_bool).unwrap_or(false) {
        1
    } else {
        0
    };

    Ok((payload, code))
}

fn status_cmd(policy: &Policy) -> Value {
    let latest = fs::read_to_string(&policy.latest_path)
        .ok()
        .and_then(|s| serde_json::from_str::<Value>(&s).ok())
        .unwrap_or_else(|| {
            json!({
                "ok": false,
                "type": "sdlc_change_control_status",
                "error": "latest_missing"
            })
        });

    let mut out = json!({
        "ok": latest.get("ok").and_then(Value::as_bool).unwrap_or(false),
        "type": "sdlc_change_control_status",
        "lane": LANE_ID,
        "ts": now_iso(),
        "latest": latest,
        "policy_path": policy.policy_path,
        "latest_path": policy.latest_path,
        "history_path": policy.history_path
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn cli_error_receipt(argv: &[String], err: &str, code: i32) -> Value {
    let mut out = json!({
        "ok": false,
        "type": "sdlc_change_control_cli_error",
        "lane": LANE_ID,
        "ts": now_iso(),
        "argv": argv,
        "error": err,
        "exit_code": code
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
