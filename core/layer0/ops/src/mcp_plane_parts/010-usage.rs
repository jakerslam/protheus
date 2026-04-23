// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::mcp_plane (authoritative)

use crate::v8_kernel::{
    append_jsonl, attach_conduit, build_plane_conduit_enforcement, canonical_json_string,
    conduit_bypass_requested, emit_plane_receipt, load_json_or, parse_bool, parse_csv_flag,
    parse_csv_or_file_unique, parse_u64, plane_status, print_json, read_json, scoped_state_root,
    sha256_hex_str, write_json,
};
use crate::{clean, now_iso, parse_args};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

const STATE_ENV: &str = "MCP_PLANE_STATE_ROOT";
const STATE_SCOPE: &str = "mcp_plane";

const CAPABILITY_MATRIX_CONTRACT_PATH: &str =
    "planes/contracts/mcp/capability_matrix_contract_v1.json";
const DURABLE_WORKFLOW_CONTRACT_PATH: &str =
    "planes/contracts/mcp/durable_workflow_contract_v1.json";
const EXPOSURE_CONTRACT_PATH: &str = "planes/contracts/mcp/exposure_contract_v1.json";
const PATTERN_PACK_CONTRACT_PATH: &str = "planes/contracts/mcp/pattern_pack_contract_v1.json";
const TEMPLATE_GOVERNANCE_CONTRACT_PATH: &str =
    "planes/contracts/mcp/template_governance_contract_v1.json";
const TEMPLATE_MANIFEST_PATH: &str = "planes/contracts/mcp/template_pack_manifest_v1.json";

fn usage() {
    println!("Usage:");
    println!("  infring-ops mcp-plane status");
    println!("  infring-ops mcp-plane capability-matrix [--server-capabilities=a,b] [--server-capabilities-file=<path>] [--strict=1|0]");
    println!("  infring-ops mcp-plane client [--server-capabilities=a,b] [--server-capabilities-file=<path>] [--strict=1|0]");
    println!(
        "  infring-ops mcp-plane server --agent=<id> [--tools=a,b] [--max-rps=<n>] [--strict=1|0]"
    );
    println!("  infring-ops mcp-plane workflow --op=<start|pause|resume|retry|status> [--workflow-id=<id>] [--checkpoint-json=<json>|--checkpoint-path=<path>] [--reason=<text>] [--strict=1|0]");
    println!(
        "  infring-ops mcp-plane expose --agent=<id> [--tools=a,b] [--max-rps=<n>] [--strict=1|0]"
    );
    println!("  infring-ops mcp-plane pattern-pack [--pattern=router|map-reduce|orchestrator|evaluator|swarm|fanout|sequential] [--tasks=a,b] [--tasks-file=<path>] [--steps-json=<json>|--steps-path=<path>] [--strict=1|0]");
    println!("  infring-ops mcp-plane template-governance [--manifest=<path>] [--templates-root=<path>] [--strict=1|0]");
    println!("  infring-ops mcp-plane template-suite [--template=<id>] [--strict=1|0]");
    println!("  infring-ops mcp-plane interop-status [--server-capabilities=a,b] [--server-capabilities-file=<path>] [--agent=<id>] [--tools=a,b] [--max-rps=<n>] [--strict=1|0]");
}

fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn emit(root: &Path, payload: Value) -> i32 {
    emit_plane_receipt(root, STATE_ENV, STATE_SCOPE, "mcp_plane_error", payload)
}

fn conduit_enforcement(
    root: &Path,
    parsed: &crate::ParsedArgs,
    strict: bool,
    action: &str,
) -> Value {
    let bypass_requested = conduit_bypass_requested(&parsed.flags);
    build_plane_conduit_enforcement(
        root,
        STATE_ENV,
        STATE_SCOPE,
        strict,
        action,
        "mcp_conduit_enforcement",
        "core/layer0/ops/mcp_plane",
        bypass_requested,
        "all_mcp_client_server_actions_are_conduit_only_with_fail_closed_bypass_rejection",
        &["V6-MCP-001.6"],
    )
}

fn status(root: &Path) -> Value {
    plane_status(root, STATE_ENV, STATE_SCOPE, "mcp_plane_status")
}

fn run_capability_matrix(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        CAPABILITY_MATRIX_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "mcp_capability_matrix_contract",
            "required_capabilities": [
                "tools.call",
                "resources.read",
                "prompts.get",
                "notifications.emit",
                "auth.session",
                "sampling.request",
                "elicitation.request",
                "roots.enumerate"
            ],
            "required_domains": [
                "tools",
                "resources",
                "prompts",
                "notifications",
                "auth",
                "sampling",
                "elicitation",
                "roots"
            ],
            "optional_capabilities": [
                "workflow.pause_resume_retry",
                "server.expose",
                "pattern.pack",
                "template.governance"
            ]
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("mcp_capability_matrix_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "mcp_capability_matrix_contract"
    {
        errors.push("mcp_capability_matrix_contract_kind_invalid".to_string());
    }

    let server_caps = parse_csv_or_file_unique(
        root,
        &parsed.flags,
        "server-capabilities",
        "server-capabilities-file",
        120,
    );
    if server_caps.is_empty() {
        errors.push("server_capabilities_required".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "mcp_plane_capability_matrix",
            "errors": errors
        });
    }

    let required = contract
        .get("required_capabilities")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|v| clean(v, 120))
        .collect::<Vec<_>>();
    let optional = contract
        .get("optional_capabilities")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|v| clean(v, 120))
        .collect::<Vec<_>>();
    let required_domains = contract
        .get("required_domains")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .iter()
        .filter_map(Value::as_str)
        .map(|v| clean(v, 40).to_ascii_lowercase())
        .collect::<Vec<_>>();

    let missing_required = required
        .iter()
        .filter(|cap| !server_caps.iter().any(|row| row == *cap))
        .cloned()
        .collect::<Vec<_>>();
    let mut domain_map = Map::<String, Value>::new();
    for cap in &server_caps {
        let domain = cap
            .split('.')
            .next()
            .unwrap_or_default()
            .to_ascii_lowercase();
        if domain.is_empty() {
            continue;
        }
        domain_map.insert(domain, Value::Bool(true));
    }
    let missing_domains = required_domains
        .iter()
        .filter(|domain| !domain_map.contains_key(domain.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let coverage = required
        .iter()
        .chain(optional.iter())
        .map(|cap| {
            json!({
                "capability": cap,
                "required": required.iter().any(|row| row == cap),
                "present": server_caps.iter().any(|row| row == cap)
            })
        })
        .collect::<Vec<_>>();
    let pass = missing_required.is_empty() && missing_domains.is_empty();
    let ok = if strict { pass } else { true };

    let result = json!({
        "required_capabilities": required,
        "required_domains": required_domains,
        "optional_capabilities": optional,
        "server_capabilities": server_caps,
        "missing_required": missing_required,
        "missing_domains": missing_domains,
        "coverage": coverage,
        "pass": pass
    });
    let artifact_path = state_root(root)
        .join("capability_matrix")
        .join("latest.json");
    let _ = write_json(&artifact_path, &result);

    let mut out = json!({
        "ok": ok,
        "strict": strict,
        "type": "mcp_plane_capability_matrix",
        "lane": "core/layer0/ops",
        "artifact": {
            "path": artifact_path.display().to_string(),
            "sha256": sha256_hex_str(&result.to_string())
        },
        "result": result,
        "errors": if pass { Value::Array(Vec::new()) } else {
            json!(["required_capabilities_or_domains_missing"])
        },
        "claim_evidence": [
            {
                "id": "V6-MCP-001.1",
                "claim": "versioned_mcp_capability_matrix_conformance_harness_produces_deterministic_pass_fail_receipts",
                "evidence": {
                    "missing_required_count": missing_required.len(),
                    "missing_domain_count": missing_domains.len(),
                    "required_dimension_count": 8,
                    "pass": pass
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}

fn load_checkpoint(root: &Path, parsed: &crate::ParsedArgs) -> Option<Value> {
    if let Some(raw) = parsed.flags.get("checkpoint-json") {
        if let Ok(value) = serde_json::from_str::<Value>(raw) {
            return Some(value);
        }
    }
    if let Some(rel_or_abs) = parsed.flags.get("checkpoint-path") {
        let path = if Path::new(rel_or_abs).is_absolute() {
            PathBuf::from(rel_or_abs)
        } else {
            root.join(rel_or_abs)
        };
        if let Some(value) = read_json(&path) {
            return Some(value);
        }
    }
    None
}
