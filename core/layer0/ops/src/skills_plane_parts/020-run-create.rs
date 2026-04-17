fn normalize_skill_tool_profile(raw: &str) -> String {
    let token = clean(raw, 40)
        .to_ascii_lowercase()
        .replace('-', "_")
        .replace(' ', "_");
    match token.as_str() {
        "minimal" | "min" => "minimal".to_string(),
        "messaging" | "message" | "msg" => "messaging".to_string(),
        "full" | "all" => "full".to_string(),
        "coding" | "code" | "dev" => "coding".to_string(),
        _ => "coding".to_string(),
    }
}

fn parse_skill_tool_groups(raw: Option<&String>) -> Vec<String> {
    let mut groups = Vec::<String>::new();
    let source = raw.map(|value| clean(value, 512)).unwrap_or_default();
    for token in source.split(|ch: char| ch == ',' || ch == ';' || ch.is_whitespace()) {
        let normalized = clean(token, 80)
            .to_ascii_lowercase()
            .replace('-', "_")
            .replace(' ', "_");
        if normalized.is_empty() {
            continue;
        }
        let canonical = if normalized.starts_with("group:") {
            normalized
        } else {
            format!("group:{normalized}")
        };
        if !groups.iter().any(|row| row == &canonical) {
            groups.push(canonical);
        }
        if groups.len() >= 24 {
            break;
        }
    }
    if groups.is_empty() {
        groups.push("group:openclaw".to_string());
    }
    groups
}

fn run_create(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        SCAFFOLD_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "skill_scaffold_contract",
            "required_files": ["SKILL.md", "skill.yaml", "scripts/run.sh", "assets/.keep", "tests/smoke.sh"],
            "default_version": "v1"
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("skill_scaffold_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "skill_scaffold_contract"
    {
        errors.push("skill_scaffold_contract_kind_invalid".to_string());
    }

    let name = clean(
        parsed
            .flags
            .get("name")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_default(),
        120,
    );
    if name.is_empty() {
        errors.push("skill_name_required".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_create",
            "errors": errors
        });
    }

    let id = slugify(&name);
    let deterministic_skill_id = format!(
        "skill_{}",
        &sha256_hex_str(&name.trim().to_ascii_lowercase())[..12]
    );
    let version = clean(
        contract
            .get("default_version")
            .and_then(Value::as_str)
            .unwrap_or("v1"),
        20,
    );
    let tool_profile = normalize_skill_tool_profile(
        parsed
            .flags
            .get("tool-profile")
            .map(String::as_str)
            .unwrap_or("coding"),
    );
    let tool_groups = parse_skill_tool_groups(parsed.flags.get("tool-groups"));
    let tool_groups_yaml = tool_groups
        .iter()
        .map(|group| format!("  - {group}\n"))
        .collect::<String>();
    let root_path = skills_root(root, parsed).join(&id);
    let skill_md = format!(
        "# {name}\n\nGenerated skill package.\n\n## Trigger\n- mention:{id}\n\n## Run\nUse `scripts/run.sh`.\n"
    );
    let skill_yaml = format!(
        "name: {id}\nversion: {version}\ndescription: Generated skill scaffold\ntriggers:\n  - mention:{id}\nentrypoint: scripts/run.sh\ntool_profile: {tool_profile}\ntool_groups:\n{tool_groups_yaml}"
    );
    let run_sh = "#!/usr/bin/env bash\nset -euo pipefail\necho \"skill_run_ok\"\n";
    let smoke_sh = "#!/usr/bin/env bash\nset -euo pipefail\nbash \"$(dirname \"$0\")/../scripts/run.sh\" >/dev/null\n";

    let mut generated = Vec::<String>::new();
    let files = [
        ("SKILL.md", skill_md),
        ("skill.yaml", skill_yaml),
        ("scripts/run.sh", run_sh.to_string()),
        ("assets/.keep", "".to_string()),
        ("tests/smoke.sh", smoke_sh.to_string()),
    ];
    for (rel, body) in files {
        let target = root_path.join(rel);
        if let Err(err) = write_file(&target, &body) {
            errors.push(err);
            continue;
        }
        #[cfg(unix)]
        if rel.ends_with(".sh") {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&target, fs::Permissions::from_mode(0o755));
        }
        generated.push(target.display().to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_create",
            "errors": errors
        });
    }

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "skills_plane_create",
        "lane": "core/layer0/ops",
        "skill": {
            "id": id,
            "deterministic_id": deterministic_skill_id,
            "name": name,
            "version": version,
            "tool_profile": tool_profile,
            "tool_groups": tool_groups,
            "root": root_path.display().to_string()
        },
        "generated_files": generated,
        "claim_evidence": [
            {
                "id": "V6-SKILLS-001.1",
                "claim": "skill_create_generates_markdown_yaml_scripts_assets_scaffold_package",
                "evidence": {
                    "file_count": generated.len(),
                    "tool_group_count": tool_groups.len()
                }
            },
            {
                "id": "V6-COGNITION-012.2",
                "claim": "natural_language_skill_creation_mints_deterministic_skill_ids_and_receipted_contracts",
                "evidence": {
                    "skill_id": deterministic_skill_id
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn parse_skill_yaml(path: &Path) -> Value {
    let raw = fs::read_to_string(path).unwrap_or_default();
    let mut name = String::new();
    let mut version = String::new();
    let mut entrypoint = String::new();
    let mut triggers = Vec::<String>::new();
    let mut in_triggers = false;
    for line in raw.lines() {
        let t = line.trim();
        if t.is_empty() {
            continue;
        }
        if t.starts_with("name:") {
            name = clean(t.trim_start_matches("name:").trim(), 120);
            in_triggers = false;
            continue;
        }
        if t.starts_with("version:") {
            version = clean(t.trim_start_matches("version:").trim(), 40);
            in_triggers = false;
            continue;
        }
        if t.starts_with("entrypoint:") {
            entrypoint = clean(t.trim_start_matches("entrypoint:").trim(), 260);
            in_triggers = false;
            continue;
        }
        if t.starts_with("triggers:") {
            in_triggers = true;
            continue;
        }
        if in_triggers && t.starts_with("- ") {
            let trigger = clean(t.trim_start_matches("- ").trim(), 180);
            if !trigger.is_empty() {
                triggers.push(trigger);
            }
            continue;
        }
        in_triggers = false;
    }
    json!({
        "name": name,
        "version": version,
        "entrypoint": entrypoint,
        "triggers": triggers
    })
}

fn run_activate(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        ACTIVATION_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "skill_activation_contract",
            "progressive_stages": ["metadata", "scripts", "assets"],
            "max_trigger_chars": 240
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("skill_activation_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "skill_activation_contract"
    {
        errors.push("skill_activation_contract_kind_invalid".to_string());
    }

    let skill = clean(
        parsed
            .flags
            .get("skill")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_default(),
        120,
    );
    let trigger = clean(
        parsed.flags.get("trigger").cloned().unwrap_or_default(),
        contract
            .get("max_trigger_chars")
            .and_then(Value::as_u64)
            .unwrap_or(240) as usize,
    );
    if skill.is_empty() {
        errors.push("skill_required".to_string());
    }
    if trigger.is_empty() {
        errors.push("trigger_required".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_activate",
            "errors": errors
        });
    }

    let skill_dir = skills_root(root, parsed).join(&skill);
    let yaml_path = skill_dir.join("skill.yaml");
    if strict && !yaml_path.exists() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_activate",
            "errors": [format!("skill_yaml_missing:{}", yaml_path.display())]
        });
    }
    let parsed_yaml = parse_skill_yaml(&yaml_path);
    let triggers = parsed_yaml
        .get("triggers")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|v| clean(v, 180).to_ascii_lowercase())
        .collect::<Vec<_>>();
    let trigger_lc = trigger.to_ascii_lowercase();
    let activated = triggers.iter().any(|row| trigger_lc.contains(row))
        || trigger_lc.contains(&format!("mention:{skill}"))
        || trigger_lc.contains(&skill);

    if strict && !activated {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "skills_plane_activate",
            "errors": ["trigger_not_matched"]
        });
    }

    let stages = contract
        .get("progressive_stages")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|stage| clean(stage, 40))
        .collect::<Vec<_>>();
    let stage_receipts = stages
        .iter()
        .enumerate()
        .map(|(idx, stage)| {
            let loaded = match stage.as_str() {
                "metadata" => true,
                "scripts" => skill_dir.join("scripts").exists(),
                "assets" => skill_dir.join("assets").exists(),
                _ => false,
            };
            json!({
                "stage": stage,
                "index": idx,
                "loaded": loaded,
                "stage_hash": sha256_hex_str(&format!("{}:{}:{}", skill, stage, idx))
            })
        })
        .collect::<Vec<_>>();

    let state_path = state_root(root).join("activation").join("latest.json");
    let state_payload = json!({
        "skill": skill,
        "trigger": trigger,
        "activated": activated,
        "stages": stage_receipts
    });
    let _ = write_json(&state_path, &state_payload);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "skills_plane_activate",
        "lane": "core/layer0/ops",
        "state_path": state_path.display().to_string(),
        "activation": state_payload,
        "claim_evidence": [
            {
                "id": "V6-SKILLS-001.2",
                "claim": "trigger_based_skill_activation_uses_progressive_loading_stages_with_deterministic_receipts",
                "evidence": {
                    "activated": activated,
                    "stage_count": stages.len()
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn parse_chain_input(root: &Path, parsed: &crate::ParsedArgs) -> Result<Value, String> {
    if let Some(raw) = parsed.flags.get("chain-json") {
        return serde_json::from_str::<Value>(raw).map_err(|_| "chain_json_invalid".to_string());
    }
    if let Some(rel_or_abs) = parsed.flags.get("chain-path") {
        let path = if Path::new(rel_or_abs).is_absolute() {
            PathBuf::from(rel_or_abs)
        } else {
            root.join(rel_or_abs)
        };
        return read_json(&path).ok_or_else(|| format!("chain_path_not_found:{}", path.display()));
    }
    Err("chain_required".to_string())
}
