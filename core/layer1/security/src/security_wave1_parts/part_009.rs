
pub fn run_truth_seeking_gate(repo_root: &Path, argv: &[String]) -> (Value, i32) {
    let args = parse_cli_args(argv);
    let cmd = args
        .positional
        .first()
        .map(|v| v.to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());
    let (policy_path, latest_path, history_path) = truth_gate_paths(repo_root);
    let policy_json = read_json_or(&policy_path, json!(TruthGatePolicy::default()));
    let policy: TruthGatePolicy = serde_json::from_value(policy_json.clone()).unwrap_or_default();

    if cmd == "status" {
        let latest = read_json_or(&latest_path, json!(null));
        let mut out = json!({
            "ok": true,
            "type": "truth_seeking_gate_status",
            "ts": now_iso(),
            "policy_path": normalize_rel_path(policy_path.display().to_string()),
            "latest_path": normalize_rel_path(latest_path.display().to_string()),
            "history_path": normalize_rel_path(history_path.display().to_string()),
            "policy": policy_json,
            "latest": latest
        });
        out["receipt_hash"] = Value::String(truth_gate_receipt_hash(&out));
        return (out, 0);
    }

    if cmd == "ingest-rule" || cmd == "ingest_rule" {
        let rule_id = normalize_token(args.flags.get("rule-id").cloned().unwrap_or_default(), 120);
        if rule_id.is_empty() {
            return (
                json!({
                    "ok": false,
                    "type": "truth_seeking_gate_ingest_rule",
                    "reason": "missing_rule_id"
                }),
                2,
            );
        }
        let trigger_tokens = normalize_tokens_csv(
            args.flags
                .get("trigger-tokens")
                .map(String::as_str)
                .unwrap_or(""),
        );
        if trigger_tokens.is_empty() {
            return (
                json!({
                    "ok": false,
                    "type": "truth_seeking_gate_ingest_rule",
                    "reason": "missing_trigger_tokens"
                }),
                2,
            );
        }
        let min_evidence_items = args
            .flags
            .get("min-evidence-items")
            .and_then(|v| v.parse::<usize>().ok())
            .unwrap_or(1)
            .clamp(0, 20);
        let require_evidence =
            bool_from_str(args.flags.get("require-evidence").map(String::as_str), true);
        let deny_reason = clean_text(
            args.flags.get("deny-reason").cloned().unwrap_or_default(),
            120,
        )
        .to_ascii_lowercase();

        let mut next_policy = policy.clone();
        next_policy.rules.retain(|rule| rule.id != rule_id);
        next_policy.rules.push(TruthGateRule {
            id: rule_id.clone(),
            trigger_tokens,
            require_evidence,
            min_evidence_items,
            deny_reason: if deny_reason.is_empty() {
                "truth_gate_rule_denied".to_string()
            } else {
                deny_reason
            },
        });
        let next_json = serde_json::to_value(&next_policy)
            .unwrap_or_else(|_| json!(TruthGatePolicy::default()));
        if let Err(err) = write_json_atomic(&policy_path, &next_json) {
            return (
                json!({
                    "ok": false,
                    "type": "truth_seeking_gate_ingest_rule",
                    "reason": err
                }),
                1,
            );
        }
        let mut out = json!({
            "ok": true,
            "type": "truth_seeking_gate_ingest_rule",
            "ts": now_iso(),
            "rule_id": rule_id,
            "rules_count": next_policy.rules.len(),
            "policy_path": normalize_rel_path(policy_path.display().to_string())
        });
        out["receipt_hash"] = Value::String(truth_gate_receipt_hash(&out));
        let _ = append_jsonl(&history_path, &out);
        let _ = write_json_atomic(&latest_path, &out);
        return (out, 0);
    }

    if cmd != "evaluate" {
        return (
            json!({
                "ok": false,
                "type": "truth_seeking_gate_error",
                "reason": format!("unknown_command:{cmd}")
            }),
            2,
        );
    }

    let claim = clean_text(args.flags.get("claim").cloned().unwrap_or_default(), 600);
    if claim.is_empty() {
        return (
            json!({
                "ok": false,
                "type": "truth_seeking_gate_evaluate",
                "reason": "missing_claim"
            }),
            2,
        );
    }
    let claim_lc = claim.to_ascii_lowercase();
    let claim_id = normalize_token(
        args.flags
            .get("claim-id")
            .cloned()
            .unwrap_or_else(|| format!("claim_{}", Utc::now().timestamp_millis())),
        120,
    );
    let persona_id = normalize_token(
        args.flags
            .get("persona-id")
            .cloned()
            .or_else(|| args.flags.get("persona_id").cloned())
            .unwrap_or_default(),
        120,
    );
    let evidence_items =
        normalize_tokens_csv(args.flags.get("evidence").map(String::as_str).unwrap_or(""));
    let evidence_count = evidence_items.len();

    let mut deny_reasons = Vec::<String>::new();
    if policy.enabled && policy.identity_binding.required && persona_id.is_empty() {
        deny_reasons.push("missing_identity_binding".to_string());
    }

    let agreement_signal = claim_has_token(&claim_lc, &policy.agreement_tokens);
    if policy.enabled
        && policy.deny_without_evidence
        && agreement_signal
        && evidence_count < policy.min_evidence_items
    {
        deny_reasons.push("agreement_without_verification_denied".to_string());
    }

    if policy.enabled {
        for rule in &policy.rules {
            if !claim_has_token(&claim_lc, &rule.trigger_tokens) {
                continue;
            }
            if rule.require_evidence && evidence_count < rule.min_evidence_items {
                deny_reasons.push(if rule.deny_reason.is_empty() {
                    format!("rule_denied:{}", rule.id)
                } else {
                    rule.deny_reason.clone()
                });
            }
        }
    }

    deny_reasons.sort();
    deny_reasons.dedup();
    let allowed = deny_reasons.is_empty();

    let mut out = json!({
        "ok": allowed,
        "type": "truth_seeking_gate_evaluate",
        "ts": now_iso(),
        "claim_id": claim_id,
        "claim": claim,
        "persona_id": if persona_id.is_empty() { Value::Null } else { Value::String(persona_id.clone()) },
        "evidence": evidence_items,
        "evidence_count": evidence_count,
        "agreement_signal": agreement_signal,
        "policy_enabled": policy.enabled,
        "decision": if allowed { "allow" } else { "deny" },
        "deny_reasons": deny_reasons,
        "policy_path": normalize_rel_path(policy_path.display().to_string()),
        "latest_path": normalize_rel_path(latest_path.display().to_string()),
        "history_path": normalize_rel_path(history_path.display().to_string())
    });
    out["receipt_hash"] = Value::String(truth_gate_receipt_hash(&out));
    let _ = append_jsonl(&history_path, &out);
    let _ = write_json_atomic(&latest_path, &out);
    (out, if allowed { 0 } else { 1 })
}

#[cfg(test)]
mod capability_switchboard_tests {
    use super::*;
    use tempfile::tempdir;

    fn write_json(path: &Path, value: &Value) {
        write_json_atomic(path, value).expect("write json");
    }

    #[test]
    fn capability_switchboard_emits_grant_revoke_hash_chain() {
        let tmp = tempdir().expect("tempdir");
        let policy_path = tmp.path().join("policy.json");
        let state_path = tmp.path().join("state.json");
        let audit_path = tmp.path().join("audit.jsonl");
        let chain_path = tmp.path().join("chain.jsonl");

        write_json(
            &policy_path,
            &json!({
                "version": "1.0",
                "require_dual_control": false,
                "policy_root": {"required": false, "scope": "capability_switchboard_toggle"},
                "switches": {
                    "autonomy": {
                        "default_enabled": true,
                        "security_locked": false,
                        "require_policy_root": false,
                        "description": "Autonomy lane"
                    }
                }
            }),
        );

        std::env::set_var("CAPABILITY_SWITCHBOARD_POLICY_PATH", &policy_path);
        std::env::set_var("CAPABILITY_SWITCHBOARD_STATE_PATH", &state_path);
        std::env::set_var("CAPABILITY_SWITCHBOARD_AUDIT_PATH", &audit_path);
        std::env::set_var("CAPABILITY_SWITCHBOARD_CHAIN_PATH", &chain_path);

        let (_, code_revoke) = run_capability_switchboard(
            tmp.path(),
            &[
                "set".to_string(),
                "--switch=autonomy".to_string(),
                "--state=off".to_string(),
                "--approver-id=op1".to_string(),
                "--approval-note=disable autonomy for maintenance".to_string(),
            ],
        );
        assert_eq!(code_revoke, 0);

        let (_, code_grant) = run_capability_switchboard(
            tmp.path(),
            &[
                "set".to_string(),
                "--switch=autonomy".to_string(),
                "--state=on".to_string(),
                "--approver-id=op1".to_string(),
                "--approval-note=re-enable autonomy after checks".to_string(),
            ],
        );
        assert_eq!(code_grant, 0);

        let (verify, verify_code) =
            run_capability_switchboard(tmp.path(), &["verify-chain".to_string()]);
        assert_eq!(verify_code, 0);
        assert_eq!(verify.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            verify
                .get("chain")
                .and_then(|v| v.get("entries"))
                .and_then(Value::as_u64),
            Some(2)
        );

        std::env::remove_var("CAPABILITY_SWITCHBOARD_POLICY_PATH");
        std::env::remove_var("CAPABILITY_SWITCHBOARD_STATE_PATH");
        std::env::remove_var("CAPABILITY_SWITCHBOARD_AUDIT_PATH");
        std::env::remove_var("CAPABILITY_SWITCHBOARD_CHAIN_PATH");
    }

    #[test]
    fn capability_switchboard_verify_chain_detects_tamper() {
        let tmp = tempdir().expect("tempdir");
        let chain_path = tmp.path().join("chain.jsonl");
        append_jsonl(
            &chain_path,
            &json!({
                "type": "capability_switchboard_chain_event",
                "ts": "2026-03-13T00:00:00Z",
                "action": "grant",
                "switch": "autonomy",
                "enabled": true,
                "approver_id": "op1",
                "second_approver_id": "",
                "reason": "ok",
                "policy_scope": "capability_switchboard_toggle",
                "prev_hash": "GENESIS",
                "hash": "tampered"
            }),
        )
        .expect("append");
        let verify = capability_switchboard_verify_chain(&chain_path);
        assert_eq!(verify.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(
            verify.get("error").and_then(Value::as_str),
            Some("chain_hash_mismatch")
        );
    }
}

#[cfg(test)]
mod truth_gate_tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn truth_gate_denies_unverified_agreement() {
        let root = tempdir().expect("tempdir");
        let policy_path = root
            .path()
            .join("client")
            .join("runtime")
            .join("config")
            .join("truth_gate_policy.json");
        ensure_parent(&policy_path).expect("policy parent");
        write_json_atomic(
            &policy_path,
            &json!({
              "version": "1.0",
              "enabled": true,
              "identity_binding": { "required": true },
              "deny_without_evidence": true,
              "min_evidence_items": 1,
              "agreement_tokens": ["agree","approved"],
              "rules": []
            }),
        )
        .expect("write policy");

        let (out, code) = run_truth_seeking_gate(
            root.path(),
            &[
                "evaluate".to_string(),
                "--claim=I agree with the proposal".to_string(),
                "--persona-id=core_guardian".to_string(),
                "--evidence=".to_string(),
            ],
        );
        assert_eq!(code, 1);
        assert_eq!(out.get("decision").and_then(Value::as_str), Some("deny"));
        let deny_rows = out
            .get("deny_reasons")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(deny_rows.iter().any(|row| {
            row.as_str()
                .map(|v| v == "agreement_without_verification_denied")
                .unwrap_or(false)
        }));
    }

    #[test]
    fn truth_gate_allows_when_evidence_present() {
        let root = tempdir().expect("tempdir");
        let (out, code) = run_truth_seeking_gate(
            root.path(),
            &[
                "evaluate".to_string(),
                "--claim=I agree based on logs".to_string(),
                "--persona-id=security_warden".to_string(),
                "--evidence=receipt:abc123".to_string(),
            ],
        );
        assert_eq!(code, 0);
        assert_eq!(out.get("decision").and_then(Value::as_str), Some("allow"));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
    }
}
