
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
