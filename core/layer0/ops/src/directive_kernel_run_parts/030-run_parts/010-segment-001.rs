
pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        println!("Usage:");
        println!("  protheus-ops directive-kernel status");
        println!("  protheus-ops directive-kernel dashboard");
        println!("  protheus-ops directive-kernel prime-sign [--directive=<text>] [--signer=<id>] [--allow-unsigned=1|0]");
        println!("  protheus-ops directive-kernel derive [--parent=<id|text>] [--directive=<text>] [--signer=<id>] [--allow-unsigned=1|0]");
        println!("  protheus-ops directive-kernel supersede [--target=<id|text>] [--directive=<text>] [--signer=<id>] [--allow-unsigned=1|0]");
        println!("  protheus-ops directive-kernel compliance-check [--action=<text>]");
        println!("  protheus-ops directive-kernel bridge-rsi [--proposal=<text>] [--apply=1|0]");
        println!(
            "  protheus-ops directive-kernel migrate [--apply=1|0] [--allow-unsigned=1|0] [--repair-signatures=1|0]"
        );
        println!("  protheus-ops directive-kernel parse-yaml [--text-base64=<base64>]");
        println!(
            "  protheus-ops directive-kernel validate-tier1-quality [--directive-id=<id>] [--text-base64=<base64>]"
        );
        println!(
            "  protheus-ops directive-kernel active-directives [--allow-missing=1|0] [--allow-weak-tier1=1|0]"
        );
        println!(
            "  protheus-ops directive-kernel merge-constraints [--payload-base64=<base64_json>]"
        );
        println!(
            "  protheus-ops directive-kernel validate-action-envelope [--payload-base64=<base64_json>]"
        );
        println!("  protheus-ops directive-kernel tier-conflict [--payload-base64=<base64_json>]");
        return 0;
    }

    let status_dashboard =
        command == "dashboard" || parse_bool(parsed.flags.get("dashboard"), false);

    if command == "status" && !status_dashboard {
        let vault = load_vault(root);
        let (signature_total, signature_valid) = signature_counts(&vault);
        let integrity = directive_vault_integrity(root);
        return emit_receipt(
            root,
            json!({
                "ok": integrity.get("ok").and_then(Value::as_bool).unwrap_or(false),
                "type": "directive_kernel_status",
                "lane": "core/layer0/ops",
                "vault": vault,
                "policy_hash": directive_vault_hash(root),
                "signature_summary": {
                    "total_entries": signature_total,
                    "valid_entries": signature_valid,
                    "invalid_entries": signature_total.saturating_sub(signature_valid)
                },
                "integrity": integrity,
                "latest": read_json(&latest_path(root))
            }),
        );
    }

    if status_dashboard {
        let vault = load_vault(root);
        let integrity = directive_vault_integrity(root);
        let prime_rows = vault
            .get("prime")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let derived_rows = vault
            .get("derived")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        let prime_count = prime_rows.len();
        let derived_count = derived_rows.len();
        let supersession_count = derived_rows
            .iter()
            .filter(|row| {
                row.get("supersedes")
                    .and_then(Value::as_str)
                    .map(|v| !v.trim().is_empty())
                    .unwrap_or(false)
            })
            .count();
        let parent_linked_derived = derived_rows
            .iter()
            .filter(|row| {
                row.get("parent_id")
                    .and_then(Value::as_str)
                    .map(|v| !v.trim().is_empty())
                    .unwrap_or(false)
            })
            .count();
        let parent_missing_derived = derived_count.saturating_sub(parent_linked_derived);
        let compliance_actions = vec![
            "blob:mutate".to_string(),
            "rsi:unsafe".to_string(),
            "organism:dream".to_string(),
            "network:gossip".to_string(),
        ];
        let compliance_preview = compliance_actions
            .iter()
            .map(|action| evaluate_action(root, action))
            .collect::<Vec<_>>();
        let denied_count = compliance_preview
            .iter()
            .filter(|row| !row.get("allowed").and_then(Value::as_bool).unwrap_or(false))
            .count();

        return emit_receipt(
            root,
            json!({
                "ok": integrity.get("ok").and_then(Value::as_bool).unwrap_or(false),
                "type": "directive_kernel_dashboard",
                "lane": "core/layer0/ops",
                "dashboard": {
                    "hierarchy": {
                        "prime_count": prime_count,
                        "derived_count": derived_count,
                        "supersession_count": supersession_count,
                        "parent_linked_derived": parent_linked_derived,
                        "parent_missing_derived": parent_missing_derived
                    },
                    "compliance": {
                        "actions_sampled": compliance_actions,
                        "preview": compliance_preview,
                        "denied_count": denied_count,
                        "integrity_ok": integrity.get("ok").and_then(Value::as_bool).unwrap_or(false)
                    }
                },
                "policy_hash": directive_vault_hash(root),
                "commands": ["protheus directives migrate", "protheus directives status", "protheus directives dashboard"],
                "layer_map": ["0","1","2","client","app"],
                "claim_evidence": [
                    {
                        "id": "V8-DIRECTIVES-001.5",
                        "claim": "directive_migration_and_visibility_dashboard_are_available_as_one_command_core_paths",
                        "evidence": {
                            "prime_count": prime_count,
                            "derived_count": derived_count,
                            "supersession_count": supersession_count,
                            "denied_preview_count": denied_count
                        }
                    }
                ]
            }),
        );
    }

