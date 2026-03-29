fn run_install(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let skill_path = parsed
        .flags
        .get("skill-path")
        .cloned()
        .or_else(|| parsed.positional.get(1).cloned())
        .unwrap_or_default();
    if skill_path.trim().is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_install",
            "errors": ["skill_path_required"]
        });
    }
    let path = if Path::new(&skill_path).is_absolute() {
        PathBuf::from(&skill_path)
    } else {
        root.join(&skill_path)
    };
    let yaml_path = path.join("skill.yaml");
    if strict && !yaml_path.exists() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_install",
            "errors": [format!("skill_yaml_missing:{}", yaml_path.display())]
        });
    }
    let parsed_yaml = parse_skill_yaml(&yaml_path);
    let id = clean(
        parsed_yaml
            .get("name")
            .and_then(Value::as_str)
            .unwrap_or_default(),
        120,
    );
    let requested_version = clean(
        parsed_yaml
            .get("version")
            .and_then(Value::as_str)
            .unwrap_or("v1"),
        40,
    );
    let backward_compat_policy = load_backward_compat_policy(root);
    let backward_compat_policy_name = backward_compat_policy
        .get("policy")
        .and_then(Value::as_str)
        .unwrap_or("semver_major")
        .to_ascii_lowercase();
    let backward_compat_min_version = clean(
        backward_compat_policy
            .get("min_version")
            .and_then(Value::as_str)
            .unwrap_or("v1"),
        40,
    );
    let backward_compat_migration_lane = clean(
        backward_compat_policy
            .get("migration_lane")
            .and_then(Value::as_str)
            .unwrap_or("skill_forced_migration"),
        120,
    );
    let backward_compat_receipt_required = backward_compat_policy
        .get("receipt_required")
        .and_then(Value::as_bool)
        .unwrap_or(true);
    let force_migration = parse_bool(parsed.flags.get("force-migration"), false);
    let allow_downgrade = parse_bool(parsed.flags.get("allow-downgrade"), false);
    let deprecation_policy = clean(
        parsed
            .flags
            .get("deprecation-policy")
            .cloned()
            .unwrap_or_else(|| "warn".to_string()),
        40,
    )
    .to_ascii_lowercase();
    let deprecation_ticket = clean(
        parsed
            .flags
            .get("deprecation-ticket")
            .cloned()
            .unwrap_or_else(String::new),
        120,
    );
    let migration_reason = clean(
        parsed
            .flags
            .get("migration-reason")
            .cloned()
            .unwrap_or_else(String::new),
        240,
    );
    if strict && id.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_install",
            "errors": ["skill_name_missing_in_yaml"]
        });
    }
    let requested_parsed = parse_skill_version(&requested_version);
    if strict && requested_parsed.is_none() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_install",
            "errors": [format!("skill_version_invalid:{}", requested_version)]
        });
    }
    let min_version_parsed = parse_skill_version(&backward_compat_min_version);
    if strict && min_version_parsed.is_none() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_install",
            "errors": [format!("compat_min_version_invalid:{}", backward_compat_min_version)]
        });
    }
    if strict {
        if let (Some(requested), Some(minimum)) = (requested_parsed, min_version_parsed) {
            if version_cmp(requested, minimum).is_lt() {
                return json!({
                    "ok": false,
                    "strict": strict,
                    "type": "skills_plane_install",
                    "errors": ["requested_skill_version_below_minimum"],
                    "compatibility": {
                        "requested_version": requested_version,
                        "min_version": backward_compat_min_version,
                        "policy": backward_compat_policy_name
                    }
                });
            }
        }
    }
    let registry_path = state_root(root).join("registry.json");
    let mut registry = load_registry(&registry_path);
    if !registry
        .get("installed")
        .map(Value::is_object)
        .unwrap_or(false)
    {
        registry["installed"] = Value::Object(Map::new());
    }
    let mut installed = registry
        .get("installed")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let previous_version_raw = installed
        .get(&id)
        .and_then(|row| row.get("version"))
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string();
    let previous_entry = installed.get(&id).cloned();
    let previous_version = parse_skill_version(&previous_version_raw);
    if strict && !previous_version_raw.is_empty() && previous_version.is_none() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_install",
            "errors": [format!("existing_skill_version_invalid:{}", previous_version_raw)]
        });
    }
    let migration_required = match (previous_version, requested_parsed) {
        (Some(prev), Some(next)) if backward_compat_policy_name == "semver_major" => {
            next.major > prev.major
        }
        (Some(prev), Some(next)) => version_cmp(next, prev).is_gt(),
        _ => false,
    };
    let downgrade_detected = match (previous_version, requested_parsed) {
        (Some(prev), Some(next)) => version_cmp(next, prev).is_lt(),
        _ => false,
    };
    let migration_lane_path = parsed
        .flags
        .get("migration-lane")
        .map(PathBuf::from)
        .map(|lane| {
            if lane.is_absolute() {
                lane
            } else {
                root.join(lane)
            }
        })
        .unwrap_or_else(|| {
            default_migration_lane_path_with_policy(
                root,
                &id,
                &previous_version_raw,
                &requested_version,
                &backward_compat_migration_lane,
            )
        });
    let mut migration_lane_exists = migration_lane_path.exists();
    let mut migration_lane_created = false;
    if strict && migration_required && !force_migration {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_install",
            "errors": ["backward_compat_break_requires_force_migration"],
            "compatibility": {
                "previous_version": previous_version_raw,
                "requested_version": requested_version,
                "migration_required": true,
                "forced_migration": false,
                "migration_lane_path": migration_lane_path.display().to_string()
            }
        });
    }
    if strict && downgrade_detected && !allow_downgrade {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_install",
            "errors": ["version_downgrade_requires_allow_downgrade"],
            "compatibility": {
                "previous_version": previous_version_raw,
                "requested_version": requested_version,
                "downgrade_detected": true,
                "migration_lane_path": migration_lane_path.display().to_string()
            }
        });
    }
    if strict && migration_required && force_migration && migration_reason.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_install",
            "errors": ["migration_reason_required_for_forced_migration"]
        });
    }
    if strict
        && migration_required
        && force_migration
        && deprecation_policy == "enforce"
        && deprecation_ticket.is_empty()
    {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_install",
            "errors": ["deprecation_ticket_required_for_enforced_migration"],
            "compatibility": {
                "deprecation_policy": deprecation_policy
            }
        });
    }
    let mut migration_receipt_emitted = false;
    let migration_latest_path = state_root(root).join("migrations").join("latest.json");
    let migration_history_path = state_root(root).join("migrations").join("history.jsonl");
    let rollback_checkpoint_path = rollback_checkpoint_path(root, &id);
    let mut rollback_checkpoint_written = false;
    if migration_required && force_migration && !migration_lane_exists {
        let lane_doc = json!({
            "ok": true,
            "type": "skills_plane_migration_lane",
            "skill_id": id.clone(),
            "from_version": previous_version_raw.clone(),
            "to_version": requested_version.clone(),
            "deprecation_policy": deprecation_policy.clone(),
            "deprecation_ticket": if deprecation_ticket.is_empty() { Value::Null } else { Value::String(deprecation_ticket.clone()) },
            "created_at": crate::now_iso(),
            "reason": migration_reason.clone()
        });
        if write_json(&migration_lane_path, &lane_doc).is_ok() {
            migration_lane_exists = true;
            migration_lane_created = true;
        } else if strict {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "skills_plane_install",
                "errors": [format!("migration_lane_write_failed:{}", migration_lane_path.display())]
            });
        }
    }
    if migration_required && force_migration && backward_compat_receipt_required {
        let mut migration_receipt = json!({
            "ok": true,
            "type": "skills_plane_migration_receipt",
            "skill_id": id.clone(),
            "from_version": previous_version_raw.clone(),
            "to_version": requested_version.clone(),
            "forced_migration": true,
            "reason": migration_reason.clone(),
            "migration_lane_path": migration_lane_path.display().to_string(),
            "deprecation_policy": deprecation_policy.clone(),
            "deprecation_ticket": if deprecation_ticket.is_empty() { Value::Null } else { Value::String(deprecation_ticket.clone()) },
            "policy": backward_compat_policy_name.clone(),
            "ts": crate::now_iso()
        });
        migration_receipt["receipt_hash"] =
            Value::String(sha256_hex_str(&canonical_json_string(&migration_receipt)));
        if append_jsonl(&migration_history_path, &migration_receipt).is_err() {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "skills_plane_install",
                "errors": [format!("migration_receipt_append_failed:{}", migration_history_path.display())]
            });
        }
        if write_json(&migration_latest_path, &migration_receipt).is_err() {
            return json!({
                "ok": false,
                "strict": strict,
                "type": "skills_plane_install",
                "errors": [format!("migration_receipt_write_failed:{}", migration_latest_path.display())]
            });
        }
        migration_receipt_emitted = true;
    }
    if migration_required && force_migration {
        let checkpoint = json!({
            "ok": true,
            "type": "skills_plane_rollback_checkpoint",
            "skill_id": id.clone(),
            "from_version": previous_version_raw.clone(),
            "to_version": requested_version.clone(),
            "previous_entry": previous_entry.clone().unwrap_or(Value::Null),
            "created_at": crate::now_iso()
        });
        if write_json(&rollback_checkpoint_path, &checkpoint).is_err() {
            if strict {
                return json!({
                    "ok": false,
                    "strict": strict,
                    "type": "skills_plane_install",
                    "errors": [format!("rollback_checkpoint_write_failed:{}", rollback_checkpoint_path.display())]
                });
            }
        } else {
            rollback_checkpoint_written = true;
        }
    }
    installed.insert(
        id.clone(),
        json!({
            "path": path.display().to_string(),
            "installed_at": crate::now_iso(),
            "version": requested_version
        }),
    );
    registry["installed"] = Value::Object(installed);
    let _ = write_json(&registry_path, &registry);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "skills_plane_install",
        "lane": "core/layer0/ops",
        "registry_path": registry_path.display().to_string(),
        "skill_id": id,
        "compatibility": {
            "previous_version": if previous_version_raw.is_empty() { Value::Null } else { Value::String(previous_version_raw.clone()) },
            "requested_version": requested_version,
            "previous_version_parsed": parse_skill_version_value(&previous_version_raw),
            "requested_version_parsed": parse_skill_version_value(
                parsed_yaml.get("version").and_then(Value::as_str).unwrap_or("v1")
            ),
            "migration_required": migration_required,
            "forced_migration": force_migration,
            "migration_receipt_emitted": migration_receipt_emitted,
            "migration_latest_path": migration_latest_path.display().to_string(),
            "migration_history_path": migration_history_path.display().to_string(),
            "migration_lane_path": migration_lane_path.display().to_string(),
            "migration_lane_exists": migration_lane_exists,
            "migration_lane_created": migration_lane_created,
            "rollback_checkpoint_path": rollback_checkpoint_path.display().to_string(),
            "rollback_checkpoint_written": rollback_checkpoint_written,
            "downgrade_detected": downgrade_detected,
            "allow_downgrade": allow_downgrade,
            "policy": backward_compat_policy_name.clone(),
            "min_version": backward_compat_min_version.clone(),
            "migration_lane": backward_compat_migration_lane.clone(),
            "receipt_required": backward_compat_receipt_required,
            "deprecation_policy": deprecation_policy.clone(),
            "deprecation_ticket": if deprecation_ticket.is_empty() { Value::Null } else { Value::String(deprecation_ticket.clone()) }
        },
        "claim_evidence": [
            {
                "id": "V6-SKILLS-001.4",
                "claim": "skill_install_run_share_actions_route_through_layer0_conduit_with_deterministic_audit_receipts",
                "evidence": {
                    "action": "install"
                }
            },
            {
                "id": "V8-SKILL-006",
                "claim": "skill_install_path_is_receipted_and_governed_through_authoritative_lane",
                "evidence": {
                    "skill_id": id
                }
            },
            {
                "id": "V8-SKILL-002",
                "claim": "skill_install_enforces_backward_compatibility_gates_and_emits_forced_migration_receipts",
                "evidence": {
                    "previous_version": if previous_version_raw.is_empty() { Value::Null } else { Value::String(previous_version_raw) },
                    "requested_version": parsed_yaml.get("version").cloned().unwrap_or(json!("v1")),
                    "migration_required": migration_required,
                    "forced_migration": force_migration,
                    "migration_receipt_emitted": migration_receipt_emitted,
                    "migration_lane_exists": migration_lane_exists,
                    "rollback_checkpoint_written": rollback_checkpoint_written,
                    "policy": backward_compat_policy_name.clone(),
                    "min_version": backward_compat_min_version.clone(),
                    "receipt_required": backward_compat_receipt_required,
                    "deprecation_policy": deprecation_policy.clone(),
                    "compatibility_gate_passed": true
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
