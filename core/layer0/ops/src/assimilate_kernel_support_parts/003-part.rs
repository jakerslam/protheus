fn framework_targets_from_surfaces(
    hints: &[String],
    api_surface: &[Value],
    structure_surface: &Value,
) -> Vec<String> {
    let mut out = framework_targets_from_hints(hints)
        .into_iter()
        .collect::<BTreeSet<_>>();
    for api in api_surface {
        match api.get("kind").and_then(Value::as_str).unwrap_or("") {
            "openapi" => {
                out.insert("workflow://openapi-service".to_string());
            }
            "proto" => {
                out.insert("workflow://grpc-service".to_string());
            }
            "graphql" => {
                out.insert("workflow://graphql-service".to_string());
            }
            _ => {}
        }
    }
    let top_ext = structure_surface
        .get("top_extensions")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    for row in top_ext {
        let ext = row.get("extension").and_then(Value::as_str).unwrap_or("");
        match ext {
            "rs" => {
                out.insert("stack://rust".to_string());
            }
            "py" => {
                out.insert("stack://python".to_string());
            }
            "ts" | "tsx" => {
                out.insert("stack://typescript".to_string());
            }
            "go" => {
                out.insert("stack://go".to_string());
            }
            _ => {}
        }
    }
    out.into_iter().collect::<Vec<_>>()
}

fn recon_index_for_target(
    normalized_target: &str,
    route: Option<&Route>,
    target_class: &str,
    ts_iso: &str,
) -> (
    Value,
    Vec<String>,
    Vec<Value>,
    Value,
    Vec<Value>,
    Vec<Value>,
    Value,
    Value,
) {
    let route_obj = route.map(|v| {
        json!({
            "domain": v.domain,
            "args": v.args
        })
    });
    let target_root = infer_target_root(normalized_target);
    let root = target_root
        .as_ref()
        .cloned()
        .unwrap_or_else(repo_root_from_env_or_cwd);
    let root_exists = target_root.as_ref().is_some();
    let manifest_inventory = if root_exists {
        parse_manifest_inventory(&root)
    } else {
        Vec::new()
    };
    let (dependency_hints, dependency_closure, dependency_graph_summary) =
        build_dependency_closure(&manifest_inventory);
    let license_surface = if root_exists {
        parse_license_surface(&root)
    } else {
        Vec::new()
    };
    let test_surface = if root_exists {
        parse_test_surface(&root)
    } else {
        json!({"directory_hints":[],"test_file_count":0,"sample_files":[],"scanned_entries":0})
    };
    let api_surface = if root_exists {
        parse_api_surface(&root)
    } else {
        Vec::new()
    };
    let structure_surface = if root_exists {
        parse_structure_surface(&root)
    } else {
        json!({"total_files":0,"scanned_entries":0,"top_extensions":[]})
    };
    let recon_index = json!({
        "recon_id": build_receipt_hash(&format!("recon:{normalized_target}"), ts_iso),
        "route": route_obj,
        "probe_set": [
            "shape_scan",
            "dependency_scan",
            "integration_scan",
            "license_surface_scan",
            "test_surface_scan",
            "api_surface_scan",
            "structure_surface_scan"
        ],
        "target_root": if root_exists { Value::String(normalized_path_text(&root)) } else { Value::Null },
        "target_class": target_class,
        "manifest_inventory": manifest_inventory,
        "dependency_graph_summary": dependency_graph_summary,
        "license_surface": license_surface,
        "test_surface": test_surface,
        "api_surface": api_surface,
        "structure_surface": structure_surface
    });
    (
        recon_index,
        dependency_hints,
        dependency_closure,
        test_surface,
        license_surface,
        api_surface,
        structure_surface,
        dependency_graph_summary,
    )
}

pub fn canonical_assimilation_plan(
    target: &str,
    route: Option<&Route>,
    ts_iso: &str,
    requested_admission_verdict: &str,
    hard_selector: &str,
    selector_bypass: bool,
) -> Value {
    let normalized_target = normalize_target(target);
    let normalized_selector = normalize_target(hard_selector);
    let hard_selector_present = !normalized_selector.is_empty();
    let target_class =
        if normalized_target.starts_with("http://") || normalized_target.starts_with("https://") {
            "url"
        } else if normalized_target.contains("://") {
            "named_target"
        } else if normalized_target.contains('/') || normalized_target.contains('\\') {
            "path"
        } else {
            "named_target"
        };
    let route_domain = route
        .map(|v| normalize_target(&v.domain))
        .unwrap_or_default();
    let selector_matches_target = !hard_selector_present
        || normalized_selector == normalized_target
        || (!route_domain.is_empty() && normalized_selector == route_domain);
    let closure_controls_satisfied = route.is_some() && selector_matches_target && !selector_bypass;
    let intent_spec = json!({
        "intent_id": build_receipt_hash(&format!("intent:{normalized_target}"), ts_iso),
        "target": normalized_target.clone(),
        "target_class": target_class,
        "requested_at": ts_iso
    });
    let (
        recon_index,
        dependency_hints,
        dependency_closure,
        test_surface,
        license_surface,
        api_surface,
        structure_surface,
        dependency_graph_summary,
    ) = recon_index_for_target(&normalized_target, route, target_class, ts_iso);
    let framework_candidates =
        framework_targets_from_surfaces(&dependency_hints, &api_surface, &structure_surface);
    let mut candidate_target_set = BTreeSet::<String>::new();
    candidate_target_set.insert(normalized_target.clone());
    if !route_domain.is_empty() {
        candidate_target_set.insert(route_domain.clone());
    }
    for candidate in framework_candidates {
        candidate_target_set.insert(candidate);
    }
    let candidate_targets = candidate_target_set.into_iter().collect::<Vec<_>>();
    let manifest_count = recon_index
        .get("manifest_inventory")
        .and_then(Value::as_array)
        .map(|rows| rows.len())
        .unwrap_or(0);
    let test_file_count = test_surface
        .get("test_file_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let license_count = license_surface.len();
    let api_surface_count = api_surface.len();
    let structure_file_count = structure_surface
        .get("total_files")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let dependency_edge_count = dependency_graph_summary
        .get("edge_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let recon_surface_complete = if target_class == "path" {
        manifest_count > 0 && structure_file_count > 0 && dependency_edge_count > 0
    } else {
        true
    };
    let closure_complete =
        closure_controls_satisfied && !candidate_targets.is_empty() && recon_surface_complete;
    let candidate_set = json!({
        "candidate_set_id": build_receipt_hash(&format!("cset:{normalized_target}"), ts_iso),
        "targets": candidate_targets,
        "dependency_hints": dependency_hints,
        "selector_mode": if hard_selector_present { "hard" } else { "auto" },
        "hard_selector": if hard_selector_present {
            Value::String(normalized_selector.clone())
        } else {
            Value::Null
        },
        "admissible_count": if closure_complete { 1 } else { 0 }
    });
    let candidate_closure = json!({
        "closure_id": build_receipt_hash(&format!("closure:{normalized_target}"), ts_iso),
        "resolved_targets": [normalized_target.clone()],
        "dependencies": dependency_closure,
        "closure_complete": closure_complete,
        "closure_stats": {
            "manifest_count": manifest_count,
            "test_file_count": test_file_count,
            "license_count": license_count,
            "api_surface_count": api_surface_count,
            "structure_file_count": structure_file_count,
            "dependency_graph_summary": dependency_graph_summary
        },
        "selected_candidate": if closure_complete {
            json!({
                "target": normalized_target.clone(),
                "route_domain": route_domain,
            })
        } else {
            Value::Null
        }
    });
    let mut gaps = Vec::<Value>::new();
    if selector_bypass {
        gaps.push(json!({
            "gap_id": "assimilation_selector_bypass_rejected",
            "severity": "blocker",
            "detail": "selector bypass is prohibited in the canonical assimilation protocol"
        }));
    }
    if hard_selector_present && !selector_matches_target {
        gaps.push(json!({
            "gap_id": "assimilation_hard_selector_closure_reject",
            "severity": "blocker",
            "detail": format!("hard selector `{}` did not resolve to the target or routed domain", normalized_selector)
        }));
    }
    if !closure_complete {
        gaps.push(json!({
            "gap_id": "assimilation_candidate_closure_incomplete",
            "severity": "blocker",
            "detail": "candidate closure is incomplete; no admissible closure candidate is available"
        }));
    }
    if target_class == "path" && manifest_count == 0 {
        gaps.push(json!({
            "gap_id": "assimilation_manifest_surface_missing",
            "severity": "blocker",
            "detail": "recon scan found no dependency manifests for a path target; assimilation cannot derive dependency closure safely"
        }));
    }
    if target_class == "path" && structure_file_count == 0 {
        gaps.push(json!({
            "gap_id": "assimilation_structure_surface_empty",
            "severity": "blocker",
            "detail": "recon scan found no source files for target path; target may be invalid or inaccessible"
        }));
    }
    if target_class == "path" && license_count == 0 {
        gaps.push(json!({
            "gap_id": "assimilation_license_surface_missing",
            "severity": "warning",
            "detail": "no license/security artifacts were discovered; legal/compliance review may be required"
        }));
    }
    if target_class == "path" && api_surface_count == 0 {
        gaps.push(json!({
            "gap_id": "assimilation_api_surface_missing",
            "severity": "warning",
            "detail": "no API/protocol surface discovered; integration blast radius may be under-modeled"
        }));
    }
    if target_class == "path" && test_file_count == 0 {
        gaps.push(json!({
            "gap_id": "assimilation_test_surface_missing",
            "severity": "warning",
            "detail": "no test surface discovered; integration confidence may be reduced"
        }));
    }
    let has_blocker_gap = gaps
        .iter()
        .any(|gap| gap.get("severity").and_then(Value::as_str) == Some("blocker"));
    let admitted =
        requested_admission_verdict == "admitted" && closure_complete && !has_blocker_gap;
    let blocker_count = gaps
        .iter()
        .filter(|gap| gap.get("severity").and_then(Value::as_str) == Some("blocker"))
        .count();
    let warning_count = gaps
        .iter()
        .filter(|gap| gap.get("severity").and_then(Value::as_str) == Some("warning"))
        .count();
    let mut denial_codes = gaps
        .iter()
        .filter(|row| row.get("severity").and_then(Value::as_str) == Some("blocker"))
        .filter_map(|row| row.get("gap_id").and_then(Value::as_str))
        .map(ToString::to_string)
        .collect::<Vec<_>>();
    let mut seen_codes = BTreeSet::<String>::new();
    denial_codes.retain(|code| seen_codes.insert(code.clone()));
    denial_codes.sort_by(|a, b| {
        assimilation_denial_priority(a)
            .cmp(&assimilation_denial_priority(b))
            .then_with(|| a.cmp(b))
    });
    let provisional_gap_report = json!({
        "gap_report_id": build_receipt_hash(&format!("gap:{normalized_target}"), ts_iso),
        "gaps": gaps,
        "risk_level": if admitted { "normal" } else { "elevated" },
        "blocker_count": blocker_count,
        "warning_count": warning_count,
        "denial_codes": denial_codes
    });
    let admission = json!({
        "admission_id": build_receipt_hash(&format!("admission:{normalized_target}"), ts_iso),
        "verdict": if admitted { "admitted" } else { "unadmitted" },
        "policy_gate": "assimilate_admission_v2",
        "requested_verdict": requested_admission_verdict,
        "required_controls": [
            "intent_spec",
            "recon_index",
            "candidate_set",
            "candidate_closure",
            "provisional_gap_report",
            "admission_verdict",
            "protocol_step_receipt"
        ],
        "denial_codes": denial_codes
    });
    let admitted_plan = json!({
        "plan_id": build_receipt_hash(&format!("plan:{normalized_target}"), ts_iso),
        "steps": [
            "intent_spec",
            "recon_index",
            "candidate_set",
            "candidate_closure",
            "gap_analysis",
            "bridge_execution",
            "receipt_commit"
        ],
        "target_root": recon_index.get("target_root").cloned().unwrap_or(Value::Null),
        "rollback": {
            "strategy": "append_only_receipt_reversal",
            "enabled": true
        },
        "status": if admitted { "ready" } else { "blocked" }
    });
    let protocol_step_receipt = json!({
        "receipt_id": build_receipt_hash(&format!("protocol:{normalized_target}"), ts_iso),
        "status": if admitted { "ready" } else { "blocked" },
        "ts": ts_iso
    });
    json!({
        "protocol_version": ASSIMILATION_PROTOCOL_VERSION,
        "intent_spec": intent_spec,
        "recon_index": recon_index,
        "candidate_set": candidate_set,
        "candidate_closure": candidate_closure,
        "provisional_gap_report": provisional_gap_report,
        "admission_verdict": admission,
        "admitted_assimilation_plan": admitted_plan,
        "protocol_step_receipt": protocol_step_receipt
    })
}

fn parse_last_json_object(raw: &str) -> Option<Value> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
        return Some(value);
    }
    for line in trimmed.lines().rev() {
        let row = line.trim();
        if row.is_empty() {
            continue;
        }
        if let Ok(value) = serde_json::from_str::<Value>(row) {
            return Some(value);
        }
    }
    None
}

fn ensure_state_dir(root: &Path) {
    let _ = fs::create_dir_all(root.join(STATE_DIR_REL));
}

