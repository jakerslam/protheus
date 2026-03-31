#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        crate::test_env_guard()
    }

    fn temp_root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("protheus_directive_kernel_{name}_{nonce}"));
        fs::create_dir_all(&root).expect("mkdir");
        root
    }

    fn write_active_directive_fixture(root: &Path) {
        let directives = directives_dir(root);
        fs::create_dir_all(&directives).expect("directive fixture dir");
        fs::write(
            directives.join("ACTIVE.yaml"),
            r#"
metadata:
  last_updated: "2026-03-17"
active_directives:
  - id: T0_invariants
    tier: 0
    status: active
  - id: T1_build_sovereign_capital_v1
    tier: 1
    status: active
"#,
        )
        .expect("write active");
        fs::write(
            directives.join("T0_invariants.yaml"),
            r#"
metadata:
  id: T0_invariants
  tier: 0
hard_blocks:
  - rule: secret_redaction
    description: Secrets must always be redacted
approval_required:
  - rule: irreversible_actions
    description: No irreversible actions without explicit approval
high_stakes_domains:
  - domain: finance
    escalation_required: true
"#,
        )
        .expect("write t0");
        fs::write(
            directives.join("T1_build_sovereign_capital_v1.yaml"),
            r#"
metadata:
  id: T1_build_sovereign_capital_v1
  tier: 1
intent:
  primary: "Generate wealth through scalable, automated systems"
  definitions:
    timeframe_years: 5
constraints:
  risk_limits:
    max_drawdown_pct: 10
success_metrics:
  leading:
    - "Monthly recurring revenue growth rate"
  lagging:
    - "Net worth progression"
scope:
  included:
    - "Income-generating automation"
  excluded:
    - "Pure consumption"
approval_policy:
  additional_gates:
    - "Impact on 5-year trajectory"
"#,
        )
        .expect("write t1");
    }

    #[test]
    fn derive_requires_parent_prime_rule() {
        let _guard = env_guard();
        std::env::set_var(SIGNING_ENV, "test-signing-key");
        let root = temp_root("derive");
        let fail = run(
            &root,
            &[
                "derive".to_string(),
                "--parent=missing".to_string(),
                "--directive=allow:child".to_string(),
                "--allow-unsigned=1".to_string(),
            ],
        );
        assert_eq!(fail, 2);

        let ok_prime = run(
            &root,
            &[
                "prime-sign".to_string(),
                "--directive=allow:missing".to_string(),
                "--signer=operator".to_string(),
                "--allow-unsigned=1".to_string(),
            ],
        );
        assert_eq!(ok_prime, 0);

        let pass = run(
            &root,
            &[
                "derive".to_string(),
                "--parent=allow:missing".to_string(),
                "--directive=allow:child".to_string(),
                "--allow-unsigned=1".to_string(),
            ],
        );
        assert_eq!(pass, 0);

        let eval = evaluate_action(&root, "child");
        assert_eq!(eval.get("allowed").and_then(Value::as_bool), Some(true));

        std::env::remove_var(SIGNING_ENV);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn tampered_signature_is_rejected_by_compliance_gate() {
        let _guard = env_guard();
        std::env::set_var(SIGNING_ENV, "test-signing-key");
        let root = temp_root("signature_tamper");
        assert_eq!(
            run(
                &root,
                &[
                    "prime-sign".to_string(),
                    "--directive=allow:graph:pagerank".to_string(),
                    "--signer=tester".to_string(),
                ],
            ),
            0
        );

        let mut vault = load_vault(&root);
        if let Some(rows) = vault.get_mut("prime").and_then(Value::as_array_mut) {
            if let Some(first) = rows.first_mut() {
                first["signature"] = Value::String("sig:tampered".to_string());
            }
        }
        write_vault(&root, &vault).expect("write vault");

        let eval = evaluate_action(&root, "graph:pagerank");
        assert_eq!(eval.get("allowed").and_then(Value::as_bool), Some(false));
        assert_eq!(
            eval.get("integrity")
                .and_then(|v| v.get("ok"))
                .and_then(Value::as_bool),
            Some(false)
        );
        assert_eq!(
            eval.get("integrity")
                .and_then(|v| v.get("errors"))
                .and_then(Value::as_array)
                .map(|rows| rows
                    .iter()
                    .any(|row| row.as_str().unwrap_or("").contains("signature_invalid"))),
            Some(true)
        );

        std::env::remove_var(SIGNING_ENV);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn legacy_placeholder_entries_are_ignored_by_integrity_gate() {
        let _guard = env_guard();
        std::env::set_var(SIGNING_ENV, "test-signing-key");
        let root = temp_root("legacy_placeholder_integrity");
        assert_eq!(
            run(
                &root,
                &[
                    "prime-sign".to_string(),
                    "--directive=allow:blob:status".to_string(),
                    "--signer=tester".to_string(),
                    "--allow-unsigned=1".to_string(),
                ],
            ),
            0
        );

        let mut vault = load_vault(&root);
        if let Some(rows) = vault.get_mut("prime").and_then(Value::as_array_mut) {
            rows.insert(
                0,
                json!({
                    "directive": "Protect operator intent",
                    "signer": "tester",
                    "supersedes_previous": true,
                    "ts": now_iso()
                }),
            );
        }
        write_vault(&root, &vault).expect("write vault");

        let integrity = directive_vault_integrity(&root);
        assert_eq!(integrity.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            integrity
                .get("ignored_legacy_entry_count")
                .and_then(Value::as_u64),
            Some(1)
        );

        let eval = evaluate_action(&root, "blob:status");
        assert_eq!(eval.get("allowed").and_then(Value::as_bool), Some(true));

        std::env::remove_var(SIGNING_ENV);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn derive_rejects_wildcard_inheritance_conflicts() {
        let _guard = env_guard();
        std::env::set_var(SIGNING_ENV, "test-signing-key");
        let root = temp_root("derive_wildcard_conflict");
        assert_eq!(
            run(
                &root,
                &[
                    "prime-sign".to_string(),
                    "--directive=deny:rsi:*".to_string(),
                    "--signer=operator".to_string(),
                    "--allow-unsigned=1".to_string(),
                ],
            ),
            0
        );
        assert_eq!(
            run(
                &root,
                &[
                    "derive".to_string(),
                    "--parent=deny:rsi:*".to_string(),
                    "--directive=allow:rsi:ignite:conduit".to_string(),
                    "--signer=system".to_string(),
                    "--allow-unsigned=1".to_string(),
                ],
            ),
            2
        );
        let eval = evaluate_action(&root, "rsi:ignite:conduit");
        assert_eq!(eval.get("allowed").and_then(Value::as_bool), Some(false));
        std::env::remove_var(SIGNING_ENV);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn signature_repair_rebuilds_chain_when_key_is_missing() {
        let _guard = env_guard();
        std::env::set_var(SIGNING_ENV, "orig-key");
        let root = temp_root("signature_repair_missing_key");
        assert_eq!(
            run(
                &root,
                &[
                    "prime-sign".to_string(),
                    "--directive=allow:blob:status".to_string(),
                    "--signer=tester".to_string(),
                    "--allow-unsigned=1".to_string(),
                ],
            ),
            0
        );
        assert_eq!(
            run(
                &root,
                &[
                    "prime-sign".to_string(),
                    "--directive=allow:credits:workspace-view".to_string(),
                    "--signer=tester".to_string(),
                    "--allow-unsigned=1".to_string(),
                ],
            ),
            0
        );

        std::env::remove_var(SIGNING_ENV);
        let before = directive_vault_integrity(&root);
        assert_eq!(before.get("ok").and_then(Value::as_bool), Some(false));

        let repair = repair_vault_signatures(&root, true, true).expect("repair signatures");
        assert_eq!(repair.get("apply").and_then(Value::as_bool), Some(true));
        assert_eq!(
            repair.get("repaired_entries").and_then(Value::as_u64),
            Some(2)
        );

        let after = directive_vault_integrity(&root);
        assert_eq!(after.get("ok").and_then(Value::as_bool), Some(true));

        let vault = load_vault(&root);
        let first_sig = vault
            .get("prime")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(|row| row.get("signature"))
            .and_then(Value::as_str)
            .unwrap_or("");
        assert!(first_sig.starts_with("unsigned:"));

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn signed_supersession_disables_targeted_rule_without_inplace_mutation() {
        let _guard = env_guard();
        std::env::set_var(SIGNING_ENV, "test-signing-key");
        let root = temp_root("supersession");
        assert_eq!(
            run(
                &root,
                &[
                    "prime-sign".to_string(),
                    "--directive=allow:blob:settle:demo".to_string(),
                    "--signer=operator".to_string(),
                ],
            ),
            0
        );
        let before = load_vault(&root);
        let before_id = before
            .get("prime")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(|row| row.get("id"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        let before_hash = before
            .get("prime")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(|row| row.get("entry_hash"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();

        let before_eval = evaluate_action(&root, "blob:settle:demo");
        assert_eq!(
            before_eval.get("allowed").and_then(Value::as_bool),
            Some(true)
        );

        assert_eq!(
            run(
                &root,
                &[
                    "supersede".to_string(),
                    "--target=allow:blob:settle:demo".to_string(),
                    "--directive=deny:blob:settle:demo".to_string(),
                    "--signer=operator".to_string(),
                ],
            ),
            0
        );
        let after_eval = evaluate_action(&root, "blob:settle:demo");
        assert_eq!(
            after_eval.get("allowed").and_then(Value::as_bool),
            Some(false)
        );

        let after = load_vault(&root);
        let after_hash = after
            .get("prime")
            .and_then(Value::as_array)
            .and_then(|rows| rows.first())
            .and_then(|row| row.get("entry_hash"))
            .and_then(Value::as_str)
            .unwrap_or("")
            .to_string();
        assert_eq!(before_hash, after_hash);
        assert_eq!(
            after_eval
                .get("superseded_ids")
                .and_then(Value::as_array)
                .map(|rows| rows
                    .iter()
                    .any(|row| row.as_str() == Some(before_id.as_str()))),
            Some(true)
        );

        std::env::remove_var(SIGNING_ENV);
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn active_directive_loader_and_merge_constraints_work() {
        let root = temp_root("active_directives");
        write_active_directive_fixture(&root);

        let directives =
            load_active_directives(&root, false, false).expect("load active directives");
        assert_eq!(directives.len(), 2);

        let merged = merge_active_constraints(&directives);
        assert_eq!(
            merged
                .get("hard_blocks")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
        assert_eq!(
            merged
                .get("approval_required")
                .and_then(Value::as_array)
                .map(|rows| rows.len()),
            Some(1)
        );
        assert_eq!(
            merged
                .get("high_stakes_domains")
                .and_then(Value::as_array)
                .map(|rows| rows.iter().any(|row| row.as_str() == Some("finance"))),
            Some(true)
        );

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn validate_action_envelope_fails_closed_for_secrets_and_requires_approval_for_irreversible() {
        let root = temp_root("validate_action_envelope");
        write_active_directive_fixture(&root);

        let blocked = validate_action_envelope(
            &root,
            &json!({
                "action_id": "act_secret",
                "tier": 2,
                "type": "other",
                "summary": "Inspect payload",
                "risk": "low",
                "payload": {
                    "token": "moltbook_sk_1234567890123456789012345"
                }
            }),
        )
        .expect("blocked result");
        assert_eq!(blocked.get("allowed").and_then(Value::as_bool), Some(false));
        assert_eq!(
            blocked
                .get("blocked_reason")
                .and_then(Value::as_str)
                .map(|text| text.contains("Secrets must always be redacted")),
            Some(true)
        );

        let approval = validate_action_envelope(
            &root,
            &json!({
                "action_id": "act_rm",
                "tier": 2,
                "type": "other",
                "summary": "cleanup deployment",
                "risk": "low",
                "payload": {},
                "metadata": {
                    "command_text": "rm -rf /tmp/demo"
                }
            }),
        )
        .expect("approval result");
        assert_eq!(
            approval.get("requires_approval").and_then(Value::as_bool),
            Some(true)
        );
        assert_eq!(approval.get("allowed").and_then(Value::as_bool), Some(true));

        let _ = fs::remove_dir_all(root);
    }
}
