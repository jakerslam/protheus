fn canonical_team_segment(raw: &str) -> String {
    let mut out = String::new();
    let mut prev_sep = false;
    for ch in raw.trim().chars() {
        let next = if ch.is_ascii_alphanumeric() || matches!(ch, '_' | '-') {
            ch.to_ascii_lowercase()
        } else {
            '-'
        };
        if next == '-' {
            if prev_sep {
                continue;
            }
            prev_sep = true;
        } else {
            prev_sep = false;
        }
        out.push(next);
        if out.len() >= 80 {
            break;
        }
    }
    let out = out.trim_matches('-').to_string();
    if out.is_empty() {
        "default-team".to_string()
    } else {
        out
    }
}

fn canonical_refresh_ms(requested: u64, default_refresh: u64, max_refresh: u64) -> u64 {
    let lower = 100_u64;
    let max_allowed = max_refresh.max(default_refresh).max(lower);
    requested.max(lower).min(max_allowed)
}

fn continuity_reconstruct_path(root: &Path, team: &str) -> PathBuf {
    let team = canonical_team_segment(team);
    state_root(root)
        .join("continuity")
        .join("reconstructed")
        .join(format!("{team}.json"))
}

fn throttle_state_path(root: &Path, team: &str) -> PathBuf {
    let team = canonical_team_segment(team);
    state_root(root)
        .join("throttle")
        .join(format!("{team}.json"))
}

fn run_dashboard(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        DASHBOARD_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "collab_team_dashboard_contract",
            "max_refresh_ms": 2000,
            "default_refresh_ms": 1000
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("collab_dashboard_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "collab_team_dashboard_contract"
    {
        errors.push("collab_dashboard_contract_kind_invalid".to_string());
    }
    let team = team_slug(
        parsed
            .flags
            .get("team")
            .map(String::as_str)
            .unwrap_or("default-team"),
    );
    let default_refresh = contract
        .get("default_refresh_ms")
        .and_then(Value::as_u64)
        .unwrap_or(1000);
    let max_refresh = contract
        .get("max_refresh_ms")
        .and_then(Value::as_u64)
        .unwrap_or(2000);
    let requested_refresh_ms = parse_u64(parsed.flags.get("refresh-ms"), default_refresh);
    if strict && requested_refresh_ms > max_refresh {
        errors.push("collab_dashboard_refresh_exceeds_contract".to_string());
    }
    let refresh_ms = canonical_refresh_ms(requested_refresh_ms, default_refresh, max_refresh);
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "collab_plane_dashboard",
            "errors": errors
        });
    }

    let launch_contract = load_json_or(
        root,
        LAUNCHER_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "collab_role_launcher_contract",
            "limits": {
                "base_max_active_agents": BASE_STABLE_MAX_ACTIVE_AGENTS,
                "max_active_agents": DEFAULT_STABLE_MAX_ACTIVE_AGENTS,
                "max_agents_per_cell": DEFAULT_MAX_AGENTS_PER_CELL,
                "director_fanout_cells": DEFAULT_DIRECTOR_FANOUT_CELLS,
                "max_directors": DEFAULT_MAX_DIRECTORS,
                "decentralized_min_agents": DEFAULT_DECENTRALIZED_MIN_AGENTS,
                "auto_director_spawn": true
            }
        }),
    );
    let limits = parse_launcher_limits(&launch_contract);
    let team_state_path = team_state_path(root, &team);
    let mut team_state = read_json(&team_state_path).unwrap_or_else(|| default_team_state(&team));
    refresh_team_topology(&mut team_state, limits);
    let _ = write_json(&team_state_path, &team_state);
    let handoffs = team_state
        .get("handoffs")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let tasks = team_state
        .get("tasks")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let agents = team_state
        .get("agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let topology = team_state.get("topology").cloned().unwrap_or_else(|| {
        json!({
            "version": "v2",
            "mode": "decentralized_hierarchy"
        })
    });
    let receipt_drilldown = vec![
        json!({
            "lane": "collab_plane",
            "latest_path": latest_path(root).display().to_string()
        }),
        json!({
            "lane": "agency_plane",
            "latest_path": state_root(root)
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join("agency_plane")
                .join("latest.json")
                .display()
                .to_string()
        }),
    ];
    let dashboard = json!({
        "version": "v1",
        "team": team,
        "refresh_ms": refresh_ms,
        "requested_refresh_ms": requested_refresh_ms,
        "refresh_normalized": requested_refresh_ms != refresh_ms,
        "target_refresh_ms": max_refresh,
        "agents": agents,
        "tasks": tasks,
        "handoff_history": handoffs,
        "topology": topology,
        "receipt_drilldown": receipt_drilldown,
        "rendered_at": crate::now_iso()
    });
    let path = state_root(root)
        .join("dashboard")
        .join(format!("{team}.json"));
    let _ = write_json(&path, &dashboard);

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "collab_plane_dashboard",
        "lane": "core/layer0/ops",
        "dashboard": dashboard,
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&dashboard.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-COLLAB-001.1",
                "claim": "team_dashboard_exposes_agent_status_tasks_handoffs_with_receipt_drilldown_and_sub_two_second_refresh",
                "evidence": {
                    "team": team,
                    "refresh_ms": refresh_ms,
                    "agent_count": agents.len(),
                    "task_count": tasks.len(),
                    "handoff_count": handoffs.len(),
                    "hierarchy_mode": dashboard
                        .get("topology")
                        .and_then(|row| row.get("mode"))
                        .and_then(Value::as_str)
                        .unwrap_or("unknown")
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn run_launch_role(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        LAUNCHER_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "collab_role_launcher_contract",
            "limits": {
                "base_max_active_agents": BASE_STABLE_MAX_ACTIVE_AGENTS,
                "max_active_agents": DEFAULT_STABLE_MAX_ACTIVE_AGENTS,
                "max_agents_per_cell": DEFAULT_MAX_AGENTS_PER_CELL,
                "director_fanout_cells": DEFAULT_DIRECTOR_FANOUT_CELLS,
                "max_directors": DEFAULT_MAX_DIRECTORS,
                "decentralized_min_agents": DEFAULT_DECENTRALIZED_MIN_AGENTS,
                "auto_director_spawn": true
            },
            "roles": {
                "director": {"default_tools": ["plan", "route", "govern"], "policy_mode": "safe"},
                "cell_coordinator": {"default_tools": ["route", "handoff"], "policy_mode": "safe"},
                "coordinator": {"default_tools": ["plan", "route"], "policy_mode": "safe"},
                "researcher": {"default_tools": ["search", "extract"], "policy_mode": "safe"},
                "builder": {"default_tools": ["compile", "verify"], "policy_mode": "safe"},
                "reviewer": {"default_tools": ["audit", "report"], "policy_mode": "safe"},
                "analyst": {"default_tools": ["summarize", "score"], "policy_mode": "safe"}
            }
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("collab_role_launcher_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "collab_role_launcher_contract"
    {
        errors.push("collab_role_launcher_contract_kind_invalid".to_string());
    }
    let team = team_slug(
        parsed
            .flags
            .get("team")
            .map(String::as_str)
            .unwrap_or("default-team"),
    );
    let role = clean(
        parsed
            .flags
            .get("role")
            .cloned()
            .or_else(|| parsed.positional.get(1).cloned())
            .unwrap_or_default(),
        80,
    );
    if role.is_empty() {
        errors.push("collab_role_required".to_string());
    }
    let role_table = contract
        .get("roles")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let limits = parse_launcher_limits(&contract);
    if strict && limits.max_active_agents < limits.base_max_active_agents.saturating_mul(2) {
        errors.push("collab_role_launcher_max_agents_not_doubled".to_string());
    }
    let role_cfg = role_table.get(&role).cloned().unwrap_or(Value::Null);
    if strict && role_cfg.is_null() {
        errors.push("collab_role_unknown".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "collab_plane_launch_role",
            "errors": errors
        });
    }

    let shadow = clean(
        parsed
            .flags
            .get("shadow")
            .cloned()
            .unwrap_or_else(|| format!("{}-{}", role, &sha256_hex_str(&team)[..8])),
        120,
    );
    let activation = json!({
        "team": team,
        "shadow": shadow,
        "role": role,
        "policy_mode": role_cfg
            .get("policy_mode")
            .cloned()
            .unwrap_or(json!("safe")),
        "default_tools": role_cfg
            .get("default_tools")
            .cloned()
            .unwrap_or_else(|| json!(["plan"])),
        "activated_at": crate::now_iso(),
        "activation_hash": sha256_hex_str(&format!("{}:{}:{}", team, shadow, role))
    });

    let team_path = team_state_path(root, &team);
    let mut team_state = read_json(&team_path).unwrap_or_else(|| default_team_state(&team));
    let mut agents = team_state
        .get("agents")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let already_active = agents.iter().any(|row| {
        row.get("shadow").and_then(Value::as_str) == Some(shadow.as_str())
            && status_from_agent(row) == "active"
    });
    let active_count = active_agents(&agents).len() as u64;
    if strict && !already_active && active_count >= limits.max_active_agents {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "collab_plane_launch_role",
            "errors": ["collab_role_team_at_capacity"],
            "capacity": {
                "active_agents": active_count,
                "stable_agent_cap": limits.max_active_agents,
                "stable_agent_cap_base": limits.base_max_active_agents
            }
        });
    }

    let mut assigned_cell = String::new();
    let mut director_shadow = String::new();
    let mut auto_spawned_director = false;
    if !already_active {
        let is_director_launch = is_director_role(role.as_str());
        if !is_director_launch {
            let worker_active_count = active_agents(&agents)
                .iter()
                .filter(|row| !is_director_role(role_from_agent(row).as_str()))
                .count();
            let cell_index = worker_active_count / limits.max_agents_per_cell.max(1) as usize;
            assigned_cell = worker_cell_id(cell_index);
            director_shadow =
                director_shadow_for_cell(&team, cell_index, limits.director_fanout_cells);
            let director_exists = agents.iter().any(|row| {
                row.get("shadow").and_then(Value::as_str) == Some(director_shadow.as_str())
                    && status_from_agent(row) == "active"
            });
            let active_directors = active_agents(&agents)
                .iter()
                .filter(|row| is_director_role(role_from_agent(row).as_str()))
                .count() as u64;
            if limits.auto_director_spawn && !director_exists {
                if strict && active_directors >= limits.max_directors {
                    return json!({
                        "ok": false,
                        "strict": strict,
                        "type": "collab_plane_launch_role",
                        "errors": ["collab_director_capacity_reached"],
                        "director_capacity": {
                            "active_directors": active_directors,
                            "max_directors": limits.max_directors
                        }
                    });
                }
                agents.push(json!({
                    "shadow": director_shadow,
                    "role": ROLE_DIRECTOR,
                    "status": "active",
                    "activated_at": crate::now_iso(),
                    "auto_managed": true,
                    "coordination": {
                        "tier": "director",
                        "decentralized": true
                    }
                }));
                auto_spawned_director = true;
            }
        } else {
            director_shadow = shadow.clone();
        }
        let coordination = if is_director_role(role.as_str()) {
            json!({
                "tier": "director",
                "decentralized": true
            })
        } else {
            json!({
                "tier": "worker",
                "decentralized": true,
                "cell_id": assigned_cell,
                "director_shadow": director_shadow,
                "routing_ring": format!("ring-{}", clean(director_shadow.clone(), 120))
            })
        };
        agents.push(json!({
            "shadow": shadow,
            "role": role,
            "status": "active",
            "activated_at": crate::now_iso(),
            "coordination": coordination
        }));
    }

    team_state["agents"] = Value::Array(agents.clone());
    refresh_team_topology(&mut team_state, limits);
    let _ = write_json(&team_path, &team_state);
    let _ = append_jsonl(
        &state_root(root).join("launch").join("history.jsonl"),
        &activation,
    );

    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "collab_plane_launch_role",
        "lane": "core/layer0/ops",
        "activation": activation,
        "topology": team_state.get("topology").cloned().unwrap_or_else(|| json!({})),
        "artifact": {
            "path": team_path.display().to_string(),
            "sha256": sha256_hex_str(&team_state.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-COLLAB-001.2",
                "claim": "instant_role_launcher_starts_policy_safe_shadows_with_deterministic_activation_receipts",
                "evidence": {
                    "team": team,
                    "role": role,
                    "shadow": shadow,
                    "stable_agent_cap_base": limits.base_max_active_agents,
                    "stable_agent_cap": limits.max_active_agents,
                    "auto_spawned_director": auto_spawned_director,
                    "director_shadow": director_shadow,
                    "cell_id": assigned_cell
                }
            },
            {
                "id": "V6-COLLAB-002.1",
                "claim": "decentralized_director_cell_hierarchy_assigns_agents_without_single_coordinator_chokepoint",
                "evidence": {
                    "team": team,
                    "auto_spawned_director": auto_spawned_director,
                    "director_shadow": director_shadow,
                    "cell_id": assigned_cell,
                    "max_agents_per_cell": limits.max_agents_per_cell,
                    "director_fanout_cells": limits.director_fanout_cells
                }
            },
            {
                "id": "V6-COLLAB-002.2",
                "claim": "stable_agent_capacity_is_explicitly_doubled_with_fail_closed_admission_control",
                "evidence": {
                    "stable_agent_cap_base": limits.base_max_active_agents,
                    "stable_agent_cap": limits.max_active_agents
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
