) -> (serde_json::Map<String, Value>, Vec<String>) {
    let mut checks = serde_json::Map::new();
    let mut violations = Vec::<String>::new();

    match profile.id {
        "V9-AUDIT-026.1" => {
            let targets = payload_string_array(
                payload,
                "audit_targets",
                &[
                    "origin_integrity",
                    "supply_chain_provenance_v2",
                    "alpha_readiness",
                ],
            );
            let missing = missing_required_tokens(
                &targets,
                &[
                    "origin_integrity",
                    "supply_chain_provenance_v2",
                    "alpha_readiness",
                ],
            );
            checks.insert("audit_targets".to_string(), json!(targets));
            checks.insert("audit_targets_missing".to_string(), json!(missing));
            if !missing.is_empty() {
                violations.push(format!(
                    "specific_missing_audit_targets:{}",
                    missing.join("|")
                ));
            }
        }
        "V9-AUDIT-026.2" => {
            let actions = payload_string_array(
                payload,
                "self_healing_actions",
                &[
                    "refresh_spine_receipt",
                    "rebuild_supply_chain_bundle",
                    "reconcile_workspace_churn",
                ],
            );
            let missing = missing_required_tokens(
                &actions,
                &[
                    "refresh_spine_receipt",
                    "rebuild_supply_chain_bundle",
                    "reconcile_workspace_churn",
                ],
            );
            checks.insert("self_healing_actions".to_string(), json!(actions));
            checks.insert("self_healing_actions_missing".to_string(), json!(missing));
            if !missing.is_empty() {
                violations.push(format!(
                    "specific_missing_self_healing_actions:{}",
                    missing.join("|")
                ));
            }
        }
        "V9-AUDIT-026.3" => {
            let range = payload_string(payload, "confidence_range", "0.0-1.0");
            checks.insert("confidence_range".to_string(), json!(range.clone()));
            if range != "0.0-1.0" {
                violations.push(format!("specific_confidence_range_mismatch:{range}"));
            }
        }
        "V9-AUDIT-026.4" => {
            let consensus = payload_string(payload, "consensus_mode", "strict_match");
            checks.insert("consensus_mode".to_string(), json!(consensus.clone()));
            if consensus != "strict_match" {
                violations.push(format!("specific_consensus_mode_mismatch:{consensus}"));
            }
        }
        "V6-DASHBOARD-007.3" => {
            checks.insert(
                "dashboard_contract_guard".to_string(),
                dashboard_contract_guard_from_payload(payload),
            );
        }
        _ if profile.id.starts_with("V6-DASHBOARD-007.") => {
            checks.insert(
                "dashboard_runtime_authority".to_string(),
                dashboard_runtime_authority_from_payload(payload),
            );
        }
        _ if profile.id.starts_with("V6-DASHBOARD-008.") => {
            checks.insert(
                "dashboard_auto_route_authority".to_string(),
                dashboard_auto_route_from_payload(payload),
            );
        }
        "V6-DASHBOARD-009.1" => {
            let (check, mut check_violations) = dashboard_message_stack_guard_from_payload(payload);
            checks.insert("dashboard_message_stack_guard".to_string(), check);
            violations.append(&mut check_violations);
        }
        "V6-DASHBOARD-009.2" => {
            let (check, mut check_violations) = dashboard_boot_retry_guard_from_payload(payload);
            checks.insert("dashboard_boot_retry_guard".to_string(), check);
            violations.append(&mut check_violations);
        }
        _ if profile.id.starts_with("V6-INFRING-GAP-001.") => {
            let (check, mut check_violations) = infring_gap_guard_from_payload(profile.id, payload);
            checks.insert("infring_gap_guard".to_string(), check);
            violations.append(&mut check_violations);
        }
        _ if profile.id.starts_with("V10-PERF-001.") => {
            let (check, mut check_violations) = perf_guard_from_payload(profile.id, payload);
            checks.insert("perf_guard".to_string(), check);
            violations.append(&mut check_violations);
        }
        _ if profile.id.starts_with("V4-DUAL-CON-") || profile.id.starts_with("V4-DUAL-MEM-") => {
            let (check, mut check_violations) = duality_guard_from_payload(profile.id, payload);
            checks.insert("duality_guard".to_string(), check);
            violations.append(&mut check_violations);
        }
        _ => {}
    }

    (checks, violations)
}

fn count_lines(path: &Path) -> u64 {
    fs::read_to_string(path)
        .ok()
        .map(|raw| raw.lines().count() as u64)
        .unwrap_or(0)
}

fn collect_repo_language_lines(dir: &Path, rs_lines: &mut u64, ts_lines: &mut u64) {
    let Ok(read) = fs::read_dir(dir) else {
        return;
    };
    for entry in read.flatten() {
        let path = entry.path();
        let Some(name) = path.file_name().and_then(|v| v.to_str()) else {
            continue;
        };
        if path.is_dir() {
            if matches!(
                name,
                ".git"
                    | "target"
                    | "node_modules"
                    | "dist"
                    | "build"
                    | "coverage"
                    | "tmp"
                    | "local"
            ) {
                continue;
            }
            collect_repo_language_lines(&path, rs_lines, ts_lines);
            continue;
        }
        if name.ends_with(".rs") {
            *rs_lines += count_lines(&path);
        } else if name.ends_with(".ts") {
            *ts_lines += count_lines(&path);
        }
    }
}

fn repo_language_share(root: &Path) -> (u64, u64, f64) {
    let mut rs_lines = 0u64;
    let mut ts_lines = 0u64;
    collect_repo_language_lines(root, &mut rs_lines, &mut ts_lines);
    let total = rs_lines.saturating_add(ts_lines);
    let rust_share_pct = if total == 0 {
        0.0
    } else {
        (rs_lines as f64) * 100.0 / (total as f64)
    };
    (rs_lines, ts_lines, rust_share_pct)
}

#[derive(Debug, Clone)]
struct ContractExecution {
    summary: Value,
    claims: Vec<Value>,
    artifacts: Vec<String>,
}

#[derive(Debug, Clone, Copy)]
struct FamilyContractRequirements {
    required_true: &'static [&'static str],
    min_values: &'static [(&'static str, f64)],
    max_values: &'static [(&'static str, f64)],
}

const EMPTY_REQUIRED_TRUE: &[&str] = &[];
const EMPTY_NUM_GATES: &[(&str, f64)] = &[];
