fn is_ts_bootstrap_wrapper(source: &str) -> bool {
    let mut normalized = source.replace("\r\n", "\n");
    if normalized.starts_with("#!") {
        if let Some((_, rest)) = normalized.split_once('\n') {
            normalized = rest.to_string();
        }
    }
    let trimmed = normalized.trim();
    let without_use_strict = trimmed
        .strip_prefix("\"use strict\";")
        .or_else(|| trimmed.strip_prefix("'use strict';"))
        .unwrap_or(trimmed)
        .trim();
    without_use_strict.contains("ts_bootstrap")
        && without_use_strict.contains(".bootstrap(__filename, module)")
}

fn effective_runtime_mode(root: &Path) -> String {
    let env_mode = std::env::var("PROTHEUS_RUNTIME_MODE")
        .unwrap_or_default()
        .trim()
        .to_ascii_lowercase();
    if env_mode == "dist" || env_mode == "source" {
        return env_mode;
    }

    let state_path = std::env::var("PROTHEUS_RUNTIME_MODE_STATE_PATH")
        .map(|v| root.join(v))
        .unwrap_or_else(|_| root.join(RUNTIME_MODE_STATE_REL));
    let Ok(raw) = fs::read_to_string(&state_path) else {
        return "source".to_string();
    };
    let Ok(parsed) = serde_json::from_str::<Value>(&raw) else {
        return "source".to_string();
    };
    let mode = parsed
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("source")
        .trim()
        .to_ascii_lowercase();
    if mode == "dist" || mode == "source" {
        return mode;
    }
    "source".to_string()
}

fn check_dist_runtime_guardrails(root: &Path) -> Result<Value, String> {
    let mode = effective_runtime_mode(root);
    if mode != "dist" {
        return Ok(json!({
            "id": "dist_runtime_guardrails",
            "ok": true,
            "mode": mode,
            "strict_wrapper_check": false,
            "wrappers_checked": 0,
        }));
    }

    if std::env::var("PROTHEUS_RUNTIME_DIST_REQUIRED").unwrap_or_default() != "1" {
        return Err(
            "dist_mode_requires_PROTHEUS_RUNTIME_DIST_REQUIRED=1_to_prevent_source_fallback"
                .to_string(),
        );
    }

    if !env_flag("CONTRACT_CHECK_DIST_WRAPPER_STRICT", false) {
        return Ok(json!({
            "id": "dist_runtime_guardrails",
            "ok": true,
            "mode": mode,
            "strict_wrapper_check": false,
            "wrappers_checked": 0,
        }));
    }

    let mut wrappers_checked = 0usize;
    let mut missing = Vec::<String>::new();
    for root_dir in ["systems", "lib"] {
        let walk_root = root.join(root_dir);
        if !walk_root.exists() {
            continue;
        }
        for entry in WalkDir::new(&walk_root)
            .into_iter()
            .filter_entry(|entry| {
                let name = entry.file_name().to_string_lossy();
                name != "node_modules" && name != ".git" && name != "dist"
            })
            .flatten()
        {
            if !entry.file_type().is_file() {
                continue;
            }
            if entry.path().extension().and_then(|ext| ext.to_str()) != Some("js") {
                continue;
            }
            let Ok(source) = fs::read_to_string(entry.path()) else {
                continue;
            };
            if !is_ts_bootstrap_wrapper(&source) {
                continue;
            }
            wrappers_checked += 1;
            let Ok(rel) = entry.path().strip_prefix(root) else {
                continue;
            };
            let dist_path = root.join("dist").join(rel);
            if !dist_path.exists() {
                missing.push(rel.to_string_lossy().to_string());
            }
        }
    }

    if !missing.is_empty() {
        missing.sort();
        return Err(format!(
            "missing_dist_wrappers:{}:{}",
            missing.len(),
            missing
                .iter()
                .take(10)
                .cloned()
                .collect::<Vec<_>>()
                .join(",")
        ));
    }

    Ok(json!({
        "id": "dist_runtime_guardrails",
        "ok": true,
        "mode": mode,
        "strict_wrapper_check": true,
        "wrappers_checked": wrappers_checked,
    }))
}

fn check_guard_registry_contracts(root: &Path) -> Result<Value, String> {
    let path = root.join(GUARD_REGISTRY_REL);
    let raw = fs::read_to_string(&path)
        .map_err(|err| format!("read_guard_registry_failed:{}:{err}", path.display()))?;
    let parsed = serde_json::from_str::<Value>(&raw)
        .map_err(|err| format!("parse_guard_registry_failed:{}:{err}", path.display()))?;
    let checks = parsed
        .pointer("/merge_guard/checks")
        .and_then(Value::as_array)
        .ok_or_else(|| "guard_registry_missing_merge_guard_checks".to_string())?;

    let mut merge_guard_ids = HashSet::<String>::new();
    let mut node_script_count = 0usize;
    for check in checks {
        if let Some(id) = check.get("id").and_then(Value::as_str) {
            let id = id.trim();
            if !id.is_empty() {
                merge_guard_ids.insert(id.to_string());
            }
        }
        if check.get("command").and_then(Value::as_str) != Some("node") {
            continue;
        }
        let rel_script = check
            .get("args")
            .and_then(Value::as_array)
            .and_then(|args| args.first())
            .and_then(Value::as_str)
            .unwrap_or("")
            .trim()
            .to_string();
        if rel_script.is_empty() {
            return Err("guard_registry_node_check_missing_script_path".to_string());
        }
        let script_path = root.join(&rel_script);
        if !script_path.exists() {
            return Err(format!(
                "guard_registry_missing_script:{}:{}",
                check.get("id").and_then(Value::as_str).unwrap_or("unknown"),
                rel_script
            ));
        }
        node_script_count += 1;
    }

    let required_ids = parsed
        .pointer("/contract_check/required_merge_guard_ids")
        .and_then(Value::as_array)
        .ok_or_else(|| "guard_registry_missing_contract_check_required_ids".to_string())?
        .iter()
        .filter_map(Value::as_str)
        .map(|id| id.trim().to_string())
        .filter(|id| !id.is_empty())
        .collect::<Vec<_>>();

    let mut missing_ids = required_ids
        .iter()
        .filter(|id| !merge_guard_ids.contains((*id).as_str()))
        .cloned()
        .collect::<Vec<_>>();
    if !missing_ids.is_empty() {
        missing_ids.sort();
        return Err(format!(
            "required_merge_guard_ids_missing:{}",
            missing_ids.join(",")
        ));
    }

    Ok(json!({
        "id": "guard_registry_contracts",
        "ok": true,
        "required_merge_guard_ids": required_ids,
        "node_script_checks": node_script_count,
    }))
}

fn check_script_help_tokens(
    root: &Path,
    rel_path: &str,
    required_tokens: &[&str],
) -> Result<Value, String> {
    let script_path = root.join(rel_path);
    if !script_path.exists() {
        return Err(format!("missing_probe_script:{rel_path}"));
    }
    let node_bin = std::env::var("PROTHEUS_NODE_BINARY").unwrap_or_else(|_| "node".to_string());
    let output = Command::new(&node_bin)
        .arg(&script_path)
        .arg("--help")
        .current_dir(root)
        .output()
        .map_err(|err| format!("probe_spawn_failed:{rel_path}:{err}"))?;

    let mut text = String::new();
    text.push_str(&String::from_utf8_lossy(&output.stdout));
    text.push(' ');
    text.push_str(&String::from_utf8_lossy(&output.stderr));
    let tokens = required_tokens
        .iter()
        .map(|token| token.to_string())
        .collect::<Vec<_>>();
    let missing = missing_tokens(&text, &tokens);
    if !output.status.success() || !missing.is_empty() {
        return Err(format!(
            "probe_failed:{}:exit={}:missing={}",
            rel_path,
            output.status.code().unwrap_or(1),
            missing.join(",")
        ));
    }

    Ok(json!({
        "id": format!("probe:{rel_path}"),
        "ok": true,
        "path": rel_path,
        "required_tokens": required_tokens.len(),
    }))
}

fn compact_json_spacing(token: &str) -> String {
    let mut out = String::with_capacity(token.len());
    let mut chars = token.chars().peekable();
    while let Some(ch) = chars.next() {
        out.push(ch);
        if ch == ':' && out.ends_with("\":") {
            while let Some(next) = chars.peek() {
                if next.is_whitespace() {
                    chars.next();
                } else {
                    break;
                }
            }
        }
    }
    out
}

pub fn missing_tokens(text: &str, tokens: &[String]) -> Vec<String> {
    let mut missing = Vec::new();
    for token in tokens {
        if text.contains(token) {
            continue;
        }
        let compact_json_token = compact_json_spacing(token);
        if compact_json_token != *token && text.contains(&compact_json_token) {
            continue;
        }
        missing.push(token.clone());
    }
    missing
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn compact_json_spacing_only_compacts_key_colon_whitespace() {
        let token = r#""schema":   {"id": "x"} value:  keep"#;
        let compacted = compact_json_spacing(token);
        assert_eq!(compacted, r#""schema":{"id":"x"} value:  keep"#);
    }

    #[test]
    fn injects_contract_check_ids_when_missing() {
        let args = vec!["run".to_string()];
        let resolved = with_contract_check_ids(&args);
        assert_eq!(resolved.len(), 2);
        assert!(resolved[1].starts_with(CHECK_IDS_FLAG_PREFIX));
        assert!(resolved[1].contains("burn_oracle_budget_gate"));
        assert!(resolved[1].contains("persona_dispatch_security_gate"));
    }

    #[test]
    fn respects_existing_contract_check_id_flag() {
        let args = vec![
            "run".to_string(),
            format!("{CHECK_IDS_FLAG_PREFIX}already-set"),
        ];
        let resolved = with_contract_check_ids(&args);
        assert_eq!(resolved, args);
    }

    #[test]
    fn missing_tokens_accepts_compact_json_variant() {
        let text = r#"{"schema":{"id":"x"}}"#;
        let tokens = vec!["\"schema\": {".to_string()];
        let missing = missing_tokens(text, &tokens);
        assert!(missing.is_empty());
    }

    #[test]
    fn missing_tokens_reports_absent_tokens() {
        let text = "usage run --help";
        let tokens = vec!["status".to_string(), "run".to_string()];
        let missing = missing_tokens(text, &tokens);
        assert_eq!(missing, vec!["status".to_string()]);
    }

    #[test]
    fn missing_tokens_preserves_missing_order() {
        let text = "run --help";
        let tokens = vec![
            "status".to_string(),
            "run".to_string(),
            "contract".to_string(),
        ];
        let missing = missing_tokens(text, &tokens);
        assert_eq!(missing, vec!["status".to_string(), "contract".to_string()]);
    }

    #[test]
    fn missing_tokens_does_not_loosen_non_json_colon_spacing() {
        let text = "value: keep";
        let tokens = vec!["value:  keep".to_string()];
        let missing = missing_tokens(text, &tokens);
        assert_eq!(missing, vec!["value:  keep".to_string()]);
    }

    #[test]
    fn missing_tokens_accepts_multiple_compacted_json_tokens() {
        let text = r#"{"schema":{"id":"x","checks":[1,2]}}"#;
        let tokens = vec![
            "\"schema\": {".to_string(),
            "\"id\":   \"x\"".to_string(),
            "\"checks\":   [1,2]".to_string(),
        ];
        let missing = missing_tokens(text, &tokens);
        assert!(missing.is_empty());
    }

    #[test]
    fn compact_json_spacing_removes_all_whitespace_after_json_key_colon() {
        let token = "\"schema\":\n\t  {\"id\":\n \"x\"}";
        let compacted = compact_json_spacing(token);
        assert_eq!(compacted, "\"schema\":{\"id\":\"x\"}");
    }

    #[test]
    fn missing_tokens_treats_empty_token_as_present_like_str_contains() {
        let text = "anything";
        let tokens = vec!["".to_string(), "absent".to_string()];
        let missing = missing_tokens(text, &tokens);
        assert_eq!(missing, vec!["absent".to_string()]);
    }

    #[test]
    fn compact_json_spacing_leaves_non_json_colon_patterns_untouched() {
        let token = "url:http://example.com key: value";
        let compacted = compact_json_spacing(token);
        assert_eq!(compacted, token);
    }

    #[test]
    fn missing_tokens_preserves_duplicate_missing_entries() {
        let text = "run";
        let tokens = vec![
            "missing".to_string(),
            "run".to_string(),
            "missing".to_string(),
        ];
        let missing = missing_tokens(text, &tokens);
        assert_eq!(missing, vec!["missing".to_string(), "missing".to_string()]);
    }

    #[test]
    fn guard_registry_contract_receipt_matches_expected_tokens() {
        let source = "guard_check_registry required_merge_guard_ids";
        let receipt = guard_registry_contract_receipt(source);
        assert!(receipt.ok);
        assert!(!receipt.fail_closed);
        assert!(receipt.missing_hooks.is_empty());
    }

    #[test]
    fn foundation_hook_coverage_receipt_detects_missing_tokens() {
        let source = "foundation_contract_gate.js";
        let receipt = foundation_hook_coverage_receipt(source);
        assert!(!receipt.ok);
        assert!(!receipt.fail_closed);
        assert!(!receipt.missing_hooks.is_empty());
        assert!(receipt
            .missing_hooks
            .contains(&"scale_envelope_baseline.js".to_string()));
    }

    #[test]
    fn foundation_hook_coverage_receipt_succeeds_when_all_hooks_are_present() {
        let source = FOUNDATION_HOOK_REQUIRED_TOKENS.join(" ");
        let receipt = foundation_hook_coverage_receipt(&source);
        assert!(receipt.ok);
        assert_eq!(
            receipt.observed_hooks.len(),
            FOUNDATION_HOOK_REQUIRED_TOKENS.len()
        );
    }
}

