
#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};
    use walkdir::WalkDir;

    fn has_claim(receipt: &Value, claim_id: &str) -> bool {
        receipt
            .get("claim_evidence")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default()
            .iter()
            .any(|row| row.get("id").and_then(Value::as_str) == Some(claim_id))
    }

    fn workspace_root() -> PathBuf {
        let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        manifest
            .ancestors()
            .nth(3)
            .expect("workspace ancestor")
            .to_path_buf()
    }

    fn copy_tree(src: &Path, dst: &Path) {
        for entry in WalkDir::new(src).into_iter().filter_map(Result::ok) {
            let rel = entry.path().strip_prefix(src).expect("strip prefix");
            let out = dst.join(rel);
            if entry.file_type().is_dir() {
                fs::create_dir_all(&out).expect("mkdir");
                continue;
            }
            if let Some(parent) = out.parent() {
                fs::create_dir_all(parent).expect("mkdir parent");
            }
            fs::copy(entry.path(), &out).expect("copy file");
        }
    }

    fn seed_batch26_contracts(root: &Path) {
        let ws = workspace_root();
        copy_tree(
            &ws.join("planes").join("contracts").join("variant_profiles"),
            &root
                .join("planes")
                .join("contracts")
                .join("variant_profiles"),
        );
        let mpu_src = ws
            .join("planes")
            .join("contracts")
            .join("mpu_compartment_profile_v1.json");
        let mpu_dst = root
            .join("planes")
            .join("contracts")
            .join("mpu_compartment_profile_v1.json");
        if let Some(parent) = mpu_dst.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        fs::copy(mpu_src, mpu_dst).expect("copy mpu");
        let wasm_src = ws
            .join("planes")
            .join("contracts")
            .join("wasm_dual_meter_policy_v1.json");
        let wasm_dst = root
            .join("planes")
            .join("contracts")
            .join("wasm_dual_meter_policy_v1.json");
        fs::copy(wasm_src, wasm_dst).expect("copy wasm");
        let hand_src = ws
            .join("planes")
            .join("contracts")
            .join("hands")
            .join("HAND.toml");
        let hand_dst = root
            .join("planes")
            .join("contracts")
            .join("hands")
            .join("HAND.toml");
        if let Some(parent) = hand_dst.parent() {
            fs::create_dir_all(parent).expect("mkdir hand");
        }
        fs::copy(hand_src, hand_dst).expect("copy hand");
    }

    #[test]
    fn native_receipt_is_deterministic() {
        let root = tempfile::tempdir().expect("tempdir");
        let args = vec![
            "run".to_string(),
            "--capability-id=test_cap".to_string(),
            "--apply=1".to_string(),
        ];
        let payload = native_receipt(root.path(), "run", &args);
        let hash = payload
            .get("receipt_hash")
            .and_then(Value::as_str)
            .expect("hash")
            .to_string();
        let mut unhashed = payload.clone();
        unhashed
            .as_object_mut()
            .expect("obj")
            .remove("receipt_hash");
        assert_eq!(receipt_hash(&unhashed), hash);
    }

    #[test]
    fn skills_enable_receipt_contains_mode() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = skills_enable_receipt(
            root.path(),
            &[
                "skills-enable".to_string(),
                "perplexity-mode".to_string(),
                "--apply=1".to_string(),
            ],
        );
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("assimilation_controller_skills_enable")
        );
        assert_eq!(
            out.get("mode").and_then(Value::as_str),
            Some("perplexity-mode")
        );
        assert!(has_claim(&out, "V6-COGNITION-012.1"));
    }

    #[test]
    fn skill_create_receipt_mints_deterministic_id() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = skill_create_receipt(
            root.path(),
            &[
                "skill-create".to_string(),
                "--task=write weekly growth recap".to_string(),
            ],
        );
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("assimilation_controller_skill_create")
        );
        let id = out.get("skill_id").and_then(Value::as_str).unwrap_or("");
        assert!(id.starts_with("skill_"));
        assert_eq!(id.len(), 18);
        assert!(has_claim(&out, "V6-COGNITION-012.2"));
    }

    #[test]
    fn skills_spawn_subagents_receipt_contains_roles() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = skills_spawn_subagents_receipt(
            root.path(),
            &[
                "skills-spawn-subagents".to_string(),
                "--task=prepare launch memo".to_string(),
                "--roles=researcher,reviewer".to_string(),
            ],
        );
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("assimilation_controller_skills_spawn_subagents")
        );
        assert_eq!(
            out.get("roles").and_then(Value::as_array).map(|v| v.len()),
            Some(2)
        );
        assert!(has_claim(&out, "V6-COGNITION-012.3"));
    }

    #[test]
    fn skills_computer_use_receipt_contains_action() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = skills_computer_use_receipt(
            root.path(),
            &[
                "skills-computer-use".to_string(),
                "--action=fill form".to_string(),
                "--target=browser".to_string(),
                "--apply=1".to_string(),
            ],
        );
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("assimilation_controller_skills_computer_use")
        );
        assert_eq!(out.get("target").and_then(Value::as_str), Some("browser"));
        assert!(has_claim(&out, "V6-COGNITION-012.4"));
        assert!(out
            .get("replay")
            .and_then(|v| v.get("replay_id"))
            .and_then(Value::as_str)
            .map(|v| !v.is_empty())
            .unwrap_or(false));
    }

    #[test]
    fn skills_dashboard_receipt_has_batch21_claim() {
        let root = tempfile::tempdir().expect("tempdir");
        let out = skills_dashboard_receipt(root.path());
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("assimilation_controller_skills_dashboard")
        );
        assert!(has_claim(&out, "V6-COGNITION-012.5"));
    }

    #[test]
    fn strict_conduit_rejects_bypass_for_skills_enable() {
        let root = tempfile::tempdir().expect("tempdir");
        let exit = run(
            root.path(),
            &[
                "skills-enable".to_string(),
                "perplexity-mode".to_string(),
                "--strict=1".to_string(),
                "--bypass=1".to_string(),
            ],
        );
        assert_eq!(exit, 1);
    }

    #[test]
    fn batch26_variant_profiles_receipt_is_validated() {
        let root = tempfile::tempdir().expect("tempdir");
        seed_batch26_contracts(root.path());
        let out = run_variant_profiles_receipt(root.path(), true);
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("assimilation_controller_variant_profiles")
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(has_claim(&out, "V7-ASSIMILATE-001.1"));
    }

    #[test]
    fn batch26_mpu_profile_receipt_is_validated() {
        let root = tempfile::tempdir().expect("tempdir");
        seed_batch26_contracts(root.path());
        let out = run_mpu_compartments_receipt(root.path(), true);
        assert_eq!(
            out.get("type").and_then(Value::as_str),
            Some("assimilation_controller_mpu_compartments")
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
        assert!(has_claim(&out, "V7-ASSIMILATE-001.2"));
    }

    #[test]
    fn batch26_capability_ledger_hash_chain_detects_tamper() {
        let root = tempfile::tempdir().expect("tempdir");
        let grant = run_capability_ledger_receipt(
            root.path(),
            &[
                "capability-ledger".to_string(),
                "--op=grant".to_string(),
                "--capability=observe".to_string(),
                "--subject=edge_node".to_string(),
                "--strict=1".to_string(),
            ],
            true,
        );
        assert_eq!(grant.get("ok").and_then(Value::as_bool), Some(true));
        let revoke = run_capability_ledger_receipt(
            root.path(),
            &[
                "capability-ledger".to_string(),
                "--op=revoke".to_string(),
                "--capability=observe".to_string(),
                "--subject=edge_node".to_string(),
                "--strict=1".to_string(),
            ],
            true,
        );
        assert_eq!(revoke.get("ok").and_then(Value::as_bool), Some(true));

        let events_path = capability_ledger_events_path(root.path());
        let mut rows = read_capability_ledger_events(&events_path);
        rows[1]["previous_hash"] = Value::String("tampered".to_string());
        let tampered = rows
            .iter()
            .map(|row| serde_json::to_string(row).expect("encode row"))
            .collect::<Vec<_>>()
            .join("\n")
            + "\n";
        fs::write(&events_path, tampered).expect("write tampered");

        let verify = run_capability_ledger_receipt(
            root.path(),
            &[
                "capability-ledger".to_string(),
                "--op=verify".to_string(),
                "--strict=1".to_string(),
            ],
            true,
        );
        assert_eq!(verify.get("ok").and_then(Value::as_bool), Some(false));
        assert!(has_claim(&verify, "V7-ASSIMILATE-001.3"));
    }

    #[test]
    fn batch26_wasm_dual_meter_fails_closed_when_budget_exhausted() {
        let root = tempfile::tempdir().expect("tempdir");
        seed_batch26_contracts(root.path());
        let out = run_wasm_dual_meter_receipt(
            root.path(),
            &[
                "wasm-dual-meter".to_string(),
                "--ticks=50".to_string(),
                "--fuel-budget=10".to_string(),
                "--epoch-budget=1".to_string(),
                "--strict=1".to_string(),
            ],
            true,
        );
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert!(has_claim(&out, "V7-ASSIMILATE-001.4"));
    }

    #[test]
    fn batch26_hands_runtime_lifecycle_is_manifest_driven() {
        let root = tempfile::tempdir().expect("tempdir");
        seed_batch26_contracts(root.path());
        let install = run_hands_runtime_receipt(
            root.path(),
            &[
                "hands-runtime".to_string(),
                "--op=install".to_string(),
                "--strict=1".to_string(),
            ],
            true,
        );
        assert_eq!(install.get("ok").and_then(Value::as_bool), Some(true));
        let rotate = run_hands_runtime_receipt(
            root.path(),
            &[
                "hands-runtime".to_string(),
                "--op=rotate".to_string(),
                "--version=2.0.1".to_string(),
                "--strict=1".to_string(),
            ],
            true,
        );
        assert_eq!(rotate.get("ok").and_then(Value::as_bool), Some(true));
        assert!(has_claim(&rotate, "V7-ASSIMILATE-001.5"));
    }

    #[test]
    fn scheduled_hands_runtime_emits_causality_and_earnings_receipts() {
        let root = tempfile::tempdir().expect("tempdir");
        seed_batch26_contracts(root.path());
        let bedrock_path = crate::core_state_root(root.path())
            .join("ops")
            .join("enterprise_hardening")
            .join("bedrock_proxy")
            .join("profile.json");
        if let Some(parent) = bedrock_path.parent() {
            fs::create_dir_all(parent).expect("mkdir");
        }
        fs::write(&bedrock_path, "{ \"ok\": true }\n").expect("write bedrock");

        let enable = run_scheduled_hands_receipt(
            root.path(),
            &[
                "scheduled-hands".to_string(),
                "--op=enable".to_string(),
                "--strict=1".to_string(),
            ],
            true,
        );
        assert_eq!(enable.get("ok").and_then(Value::as_bool), Some(true));
        assert!(has_claim(&enable, "V7-ASSIMILATE-001.5.4"));

        let run = run_scheduled_hands_receipt(
            root.path(),
            &[
                "scheduled-hands".to_string(),
                "--op=run".to_string(),
                "--iterations=3".to_string(),
                "--task=lead-intake-refresh".to_string(),
                "--cross-refs=memory,research".to_string(),
                "--strict=1".to_string(),
            ],
            true,
        );
        assert_eq!(run.get("ok").and_then(Value::as_bool), Some(true));
        assert!(has_claim(&run, "V7-ASSIMILATE-001.5.2"));
        assert!(has_claim(&run, "V7-ASSIMILATE-001.5.3"));
        assert!(run
            .pointer("/run/causality/trace_id")
            .and_then(Value::as_str)
            .map(|row| !row.is_empty())
            .unwrap_or(false));
        assert!(run
            .pointer("/run/earnings/usd")
            .and_then(Value::as_f64)
            .map(|row| row > 0.0)
            .unwrap_or(false));
    }
}
