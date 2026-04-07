// SPDX-License-Identifier: Apache-2.0
// File: core/layer0/ops/src/security_plane_v6_sec_011.rs
// Purpose: Auto-Remediate Command (V6-SEC-011)
// Layer ownership: core/layer1/security (authoritative)

use crate::clean;
use crate::contract_lane_utils as lane_utils;
use crate::{deterministic_receipt_hash, now_iso};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::path::Path;

// ========================================================
// POLICY PATCH STRUCTURE
// ========================================================

/// Remediation policy patch that blocks dangerous patterns
#[derive(Debug, Clone)]
pub struct PolicyPatch {
    pub scan_id: String,
    pub generated_at: String,
    pub blocked_patterns: Vec<Value>,
    pub rules: RemediationRules,
    pub next_action: NextAction,
    pub metadata: HashMap<String, String>,
}

/// Remediation rules to enforce
#[derive(Debug, Clone)]
pub struct RemediationRules {
    pub deny_tool_poisoning: bool,
    pub deny_prompt_override: bool,
    pub require_index_first: bool,
    pub conduit_only_execution: bool,
}

impl RemediationRules {
    pub fn hardened() -> Self {
        Self {
            deny_tool_poisoning: true,
            deny_prompt_override: true,
            require_index_first: true,
            conduit_only_execution: true,
        }
    }

    pub fn permissive() -> Self {
        Self {
            deny_tool_poisoning: false,
            deny_prompt_override: false,
            require_index_first: false,
            conduit_only_execution: false,
        }
    }
}

#[derive(Debug, Clone)]
pub enum NextAction {
    RescanRequired,
    PromotionAllowed,
    ManualReviewRequired,
}

impl NextAction {
    fn as_str(&self) -> &'static str {
        match self {
            NextAction::RescanRequired => "rescan_required",
            NextAction::PromotionAllowed => "promotion_allowed",
            NextAction::ManualReviewRequired => "manual_review_required",
        }
    }
}

// ========================================================
// REMEDIATION TEMPLATES
// ========================================================

/// Map hit patterns to specific policy rules
fn pattern_to_rules(hit: &str) -> Vec<&'static str> {
    let mut rules = Vec::new();
    
    match hit {
        h if h.contains("tool poisoning") || h.contains("mcp://") => {
            rules.push("deny_tool_poisoning");
            rules.push("conduit_only_execution");
        }
        h if h.contains("ignore") || h.contains("override") || h.contains("reveal") => {
            rules.push("deny_prompt_override");
            rules.push("require_index_first");
        }
        h if h.contains("execute without approval") => {
            rules.push("conduit_only_execution");
            rules.push("deny_tool_poisoning");
        }
        h if h.contains("export") || h.contains("siphon") => {
            rules.push("conduit_only_execution");
        }
        _ => {
            rules.push("require_index_first");
        }
    }
    
    rules
}

// ========================================================
// CORE REMEDIATE FUNCTION
// ========================================================

/// Run auto-remediation based on prior scan results
/// 
/// # Arguments
/// * `root` - Workspace root directory
/// * `argv` - Command line arguments
/// * `strict` - Fail-closed mode
/// 
/// # Returns
/// * `(Value, i32)` - JSON output and exit code
pub fn run_remediation_command(root: &Path, argv: &[String], strict: bool) -> (Value, i32) {
    // Read previous scan artifact
    let latest = match read_json(&scanner_latest_path(root)) {
        Some(doc) => doc,
        None => {
            let out = json!({
                "ok": false,
                "type": "security_plane_auto_remediation",
                "lane": "core/layer1/security",
                "mode": "remediate",
                "strict": strict,
                "error": "scan_missing",
                "claim_evidence": [{
                    "id": "V6-SEC-011",
                    "claim": "auto_remediation_lane_requires_scan_artifacts_before_policy_patch_proposal",
                    "evidence": {"scan_present": false}
                }]
            });
            return (out, if strict { 2 } else { 0 });
        }
    };

    // Extract scan data
    let scan = latest.get("scan").cloned().unwrap_or_else(|| json!({}));
    let critical_hits = scan
        .get("critical_hits")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let hit_rows = scan
        .get("hits")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let scan_id = latest
        .get("scan_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown_scan")
        .to_string();
    let success_rate = scan
        .get("success_rate")
        .and_then(Value::as_f64)
        .unwrap_or(1.0);
    let score = scan
        .get("score")
        .and_then(Value::as_u64)
        .unwrap_or(100);

    // Determine promotion status
    let promotion_blocked = critical_hits > 0;
    let next_action = if promotion_blocked {
        NextAction::RescanRequired
    } else if success_rate < 1.0 {
        NextAction::PromotionAllowed
    } else {
        NextAction::PromotionAllowed
    };

    // Build remediation rules based on actual hits
    let mut rules = HashMap::new();
    for hit in &hit_rows {
        if let Some(hit_str) = hit.as_str() {
            for rule in pattern_to_rules(hit_str) {
                rules.insert(rule, true);
            }
        } else if let Some(hit_obj) = hit.as_object() {
            if let Some(pat) = hit_obj.get("pattern").and_then(Value::as_str) {
                for rule in pattern_to_rules(pat) {
                    rules.insert(rule, true);
                }
            }
        }
    }

    // Generate policy patch
    let patch = PolicyPatch {
        scan_id: scan_id.clone(),
        generated_at: now_iso(),
        blocked_patterns: hit_rows.clone(),
        rules: RemediationRules {
            deny_tool_poisoning: rules.contains_key("deny_tool_poisoning"),
            deny_prompt_override: rules.contains_key("deny_prompt_override"),
            require_index_first: rules.contains_key("require_index_first"),
            conduit_only_execution: rules.contains_key("conduit_only_execution"),
        },
        next_action: next_action.clone(),
        metadata: {
            let mut m = HashMap::new();
            m.insert("remediation_version".to_string(), "1.0".to_string());
            m.insert("score".to_string(), score.to_string());
            m
        },
    };

    // Serialize patch to JSON
    let patch_json = json!({
        "scan_id": patch.scan_id,
        "generated_at": patch.generated_at,
        "blocked_patterns": patch.blocked_patterns,
        "rules": {
            "deny_tool_poisoning": patch.rules.deny_tool_poisoning,
            "deny_prompt_override": patch.rules.deny_prompt_override,
            "require_index_first": patch.rules.require_index_first,
            "conduit_only_execution": patch.rules.conduit_only_execution,
        },
        "next_action": patch.next_action.as_str(),
        "metadata": patch.metadata,
    });

    // Determine patch path
    let patch_filename = format!("prompt_policy_patch_{}.json", &scan_id[..16.min(scan_id.len())]);
    let patch_path = remediation_state_dir(root).join(&patch_filename);

    // Ensure directory exists and write patch
    if let Some(parent) = patch_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    write_json(&patch_path, &patch_json);

    // Update promotion gate
    let gate = json!({
        "updated_at": now_iso(),
        "scan_id": &scan_id,
        "promotion_blocked": promotion_blocked,
        "patch_path": patch_path.display().to_string(),
        "critical_hits": critical_hits,
        "score": score,
        "success_rate": success_rate,
    });
    let gate_path = remediation_gate_path(root);
    if let Some(parent) = gate_path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    write_json(&gate_path, &gate);

    // Build output
    let out = json!({
        "ok": !promotion_blocked,
        "type": "security_plane_auto_remediation",
        "lane": "core/layer1/security",
        "mode": "remediate",
        "strict": strict,
        "scan_id": scan_id,
        "critical_hits": critical_hits,
        "score": score,
        "success_rate": success_rate,
        "promotion_blocked": promotion_blocked,
        "patch_path": patch_path.display().to_string(),
        "next_action": patch.next_action.as_str(),
        "rules_applied": {
            "deny_tool_poisoning": patch.rules.deny_tool_poisoning,
            "deny_prompt_override": patch.rules.deny_prompt_override,
            "require_index_first": patch.rules.require_index_first,
            "conduit_only_execution": patch.rules.conduit_only_execution,
        },
        "claim_evidence": [{
            "id": "V6-SEC-011",
            "claim": "auto_remediation_generates_policy_patch_and_blocks_promotion_until_rescan_passes",
            "evidence": {
                "scan_id": scan_id,
                "critical_hits": critical_hits,
                "promotion_blocked": promotion_blocked,
                "patch_path": patch_path.display().to_string()
            }
        }]
    });

    let exit_code = if strict && promotion_blocked { 2 } else { 0 };
    (out, exit_code)
}

// ========================================================
// PROMOTION GATE OPERATIONS
// ========================================================

/// Check promotion gate status
pub fn check_promotion_gate(root: &Path) -> Option<Value> {
    let path = remediation_gate_path(root);
    read_json(&path)
}

/// Clear promotion gate (for testing or emergency override)
pub fn clear_promotion_gate(root: &Path) -> bool {
    let path = remediation_gate_path(root);
    std::fs::remove_file(path).is_ok()
}

/// Get list of applied patches
pub fn list_patches(root: &Path) -> Vec<std::path::PathBuf> {
    let dir = remediation_state_dir(root);
    let mut patches = Vec::new();
    
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().map(|e| e == "json").unwrap_or(false) {
                if path.file_stem()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .starts_with("prompt_policy_patch_")
                {
                    patches.push(path);
                }
            }
        }
    }
    
    patches.sort();
    patches
}

/// Get active remediation rules
pub fn get_active_rules(root: &Path) -> Option<RemediationRules> {
    let gate = check_promotion_gate(root)?;
    let patch_path = gate.get("patch_path").and_then(Value::as_str)?;
    let patch = read_json(Path::new(patch_path))?;
    
    Some(RemediationRules {
        deny_tool_poisoning: patch
            .pointer("/rules/deny_tool_poisoning")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        deny_prompt_override: patch
            .pointer("/rules/deny_prompt_override")
            .and_then(Value::as_bool)
            .unwrap_or(true),
        require_index_first: patch
            .pointer("/rules/require_index_first")
            .and_then(Value::as_bool)
            .unwrap_or(false),
        conduit_only_execution: patch
            .pointer("/rules/conduit_only_execution")
            .and_then(Value::as_bool)
            .unwrap_or(false),
    })
}

// ========================================================
// HELPER FUNCTIONS
// ========================================================

pub fn remediation_state_dir(root: &Path) -> std::path::PathBuf {
    root.join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("security_plane")
        .join("remediation")
}

pub fn remediation_gate_path(root: &Path) -> std::path::PathBuf {
    remediation_state_dir(root).join("promotion_gate.json")
}

pub fn scanner_latest_path(root: &Path) -> std::path::PathBuf {
    root.join("core")
        .join("local")
        .join("state")
        .join("ops")
        .join("security_plane")
        .join("scanner")
        .join("latest.json")
}

fn write_json(path: &std::path::Path, payload: &Value) {
    let _ = lane_utils::write_json(path, payload);
}

fn read_json(path: &std::path::Path) -> Option<Value> {
    lane_utils::read_json(path)
}

// ========================================================
// MODULE EXPORTS
// ========================================================

pub use {
    run_remediation_command,
    check_promotion_gate,
    clear_promotion_gate,
    list_patches,
    get_active_rules,
    PolicyPatch,
    RemediationRules,
};

// ========================================================
// UNIT TESTS
// ========================================================

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    
    fn temp_root() -> (tempfile::TempDir, std::path::PathBuf) {
        let tmp = tempfile::tempdir().expect("tempdir");
        let root = tmp.path().to_path_buf();
        (tmp, root)
    }
    
    fn create_scan_artifact(root: &Path, critical_hits: u64) {
        let content = json!({
            "scan_id": "test_scan_12345",
            "scan": {
                "critical_hits": critical_hits,
                "hits": if critical_hits > 0 {
                    vec![json!("tool poisoning"), json!("ignore previous")]
                } else {
                    vec![]
                },
                "success_rate": if critical_hits == 0 { 1.0 } else { 0.5 },
                "score": if critical_hits == 0 { 100 } else { 50 },
            }
        });
        
        let path = scanner_latest_path(root);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).expect("create dir");
        }
        write_json(&path, &content);
    }
    
    fn parse_flag(argv: &[String], key: &str) -> Option<String> {
        argv.iter()
            .find(|arg| arg.starts_with(&format!("--{}=", key)))
            .map(|arg| {
                arg.splitn(2, '=')
                    .nth(1)
                    .map(|v| v.to_string())
                    .unwrap_or_default()
            })
    }
    
    #[test]
    fn pattern_to_rules_maps_correctly() {
        assert!(pattern_to_rules("tool poisoning").contains(&"deny_tool_poisoning"));
        assert!(pattern_to_rules("ignore previous instructions").contains(&"deny_prompt_override"));
        assert!(pattern_to_rules("mcp://override-policy").contains(&"conduit_only_execution"));
    }
    
    #[test]
    fn remediation_requires_scan_artifact() {
        let (_tmp, root) = temp_root();
        
        let args: Vec<String> = vec![
            "--strict=1".to_string(),
        ];
        
        let (out, code) = run_remediation_command(&root,
            &args,
            true
        );
        
        assert_eq!(code, 2);
        assert_eq!(out.get("error").and_then(Value::as_str), Some("scan_missing"));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
    }
    
    #[test]
    fn remediation_generates_policy_patch() {
        let (_tmp, root) = temp_root();
        
        // Create scan artifact with hits
        create_scan_artifact(&root, 2);
        
        let args: Vec<String> = vec![];
        let (out, code) = run_remediation_command(&root,
            &args,
            false
        );
        
        assert_eq!(code, 0);
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false)); // Blocked due to hits
        
        // Verify patch was created
        let patch_path = out
            .get("patch_path")
            .and_then(Value::as_str)
            .expect("patch_path");
        assert!(std::path::Path::new(patch_path).exists());
        
        // Load and verify patch content
        let patch = read_json(Path::new(patch_path)).expect("read patch");
        assert!(patch.pointer("/rules/deny_tool_poisoning").and_then(Value::as_bool).unwrap_or(false));
    }
    
    #[test]
    fn remediation_blocks_promotion_with_critical_hits() {
        let (_tmp, root) = temp_root();
        
        create_scan_artifact(&root, 2);
        
        let (out, code) = run_remediation_command(&root,
            &["--strict=1".to_string()],
            true
        );
        
        assert_eq!(code, 2);
        assert_eq!(out.get("promotion_blocked").and_then(Value::as_bool), Some(true));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(false));
        assert_eq!(out.pointer("/rules_applied/deny_tool_poisoning").and_then(Value::as_bool), Some(true));
    }
    
    #[test]
    fn remediation_allows_promotion_after_clean_scan() {
        let (_tmp, root) = temp_root();
        
        // First dirty scan
        create_scan_artifact(&root, 2);
        run_remediation_command(&root,
            &[],
            false
        );
        
        // Clean scan
        create_scan_artifact(&root, 0);
        
        let (out, code) = run_remediation_command(&root,
            &["--strict=1".to_string()],
            true
        );
        
        assert_eq!(code, 0);
        assert_eq!(out.get("promotion_blocked").and_then(Value::as_bool), Some(false));
        assert_eq!(out.get("ok").and_then(Value::as_bool), Some(true));
    }
    
    #[test]
    fn check_promotion_gate_returns_correct_status() {
        let (_tmp, root) = temp_root();
        
        // No gate initially
        assert!(check_promotion_gate(&root).is_none());
        
        // Create gate
        create_scan_artifact(&root, 1);
        run_remediation_command(&root, &[], false);
        
        let gate = check_promotion_gate(&root).expect("gate exists");
        assert_eq!(gate.get("promotion_blocked").and_then(Value::as_bool), Some(true));
        assert!(gate.get("scan_id").is_some());
    }
    
    #[test]
    fn list_patches_returns_created_patches() {
        let (_tmp, root) = temp_root();
        
        // Create scan and remediate multiple times
        for i in 0..3 {
            create_scan_artifact(&root, i);
            run_remediation_command(&root, &[], false);
        }
        
        let patches = list_patches(&root);
        assert_eq!(patches.len(), 3);
    }
    
    #[test]
    fn get_active_rules_returns_rules() {
        let (_tmp, root) = temp_root();
        
        create_scan_artifact(&root, 1);
        run_remediation_command(&root, &[], false);
        
        let rules = get_active_rules(&root);
        assert!(rules.is_some());
        let rules = rules.unwrap();
        assert!(rules.deny_tool_poisoning || rules.deny_prompt_override);
    }
    
    #[test]
    fn remediation_rules_struct_works() {
        let hardened = RemediationRules::hardened();
        assert!(hardened.deny_tool_poisoning);
        assert!(hardened.deny_prompt_override);
        assert!(hardened.require_index_first);
        assert!(hardened.conduit_only_execution);
        
        let permissive = RemediationRules::permissive();
        assert!(!permissive.deny_tool_poisoning);
        assert!(!permissive.deny_prompt_override);
        assert!(!permissive.require_index_first);
        assert!(!permissive.conduit_only_execution);
    }
}
