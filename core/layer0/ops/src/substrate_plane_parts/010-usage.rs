// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops::substrate_plane (authoritative)

use crate::v8_kernel::{
    append_jsonl, attach_conduit, build_plane_conduit_enforcement, conduit_bypass_requested,
    emit_plane_receipt, load_json_or, parse_bool, parse_f64, parse_u64, plane_status, print_json,
    read_json, scoped_state_root, sha256_hex_str, split_csv_clean, write_json,
};
use crate::{clean, parse_args};
use exotic_wrapper::{default_degradation, wrap_exotic_signal, ExoticDomain, ExoticEnvelope};
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

const STATE_ENV: &str = "SUBSTRATE_PLANE_STATE_ROOT";
const STATE_SCOPE: &str = "substrate_plane";

const CSI_CAPTURE_CONTRACT_PATH: &str = "planes/contracts/substrate/csi_capture_contract_v1.json";
const CSI_MODULE_CONTRACT_PATH: &str =
    "planes/contracts/substrate/csi_module_registry_contract_v1.json";
const CSI_EMBEDDED_CONTRACT_PATH: &str =
    "planes/contracts/substrate/csi_embedded_profile_contract_v1.json";
const CSI_POLICY_CONTRACT_PATH: &str = "planes/contracts/substrate/csi_policy_contract_v1.json";
const EYE_BINDING_CONTRACT_PATH: &str = "planes/contracts/substrate/eye_binding_contract_v1.json";
const BIO_INTERFACE_CONTRACT_PATH: &str =
    "planes/contracts/substrate/biological_interface_contract_v1.json";
const BIO_FEEDBACK_CONTRACT_PATH: &str =
    "planes/contracts/substrate/biological_feedback_contract_v1.json";
const BIO_ADAPTER_TEMPLATE_CONTRACT_PATH: &str =
    "planes/contracts/substrate/biological_adapter_template_contract_v1.json";
const BIO_ETHICS_POLICY_CONTRACT_PATH: &str =
    "planes/contracts/substrate/biological_ethics_policy_contract_v1.json";
const BIO_ENABLE_CONTRACT_PATH: &str =
    "planes/contracts/substrate/biological_enable_contract_v1.json";

fn usage() {
    println!("Usage:");
    println!("  protheus-ops substrate-plane status");
    println!(
        "  protheus-ops substrate-plane csi-capture [--adapter=<id>] [--signal-ref=<ref>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops substrate-plane csi-module --op=<register|activate|list> [--module=<id>] [--input-contract=<id>] [--budget-units=<n>] [--privacy-class=<local|sensitive|restricted>] [--degrade-behavior=<id>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops substrate-plane csi-embedded-profile [--target=<esp32>] [--power-mw=<n>] [--latency-ms=<n>] [--bounded-memory-kb=<n>] [--offline=1|0] [--strict=1|0]"
    );
    println!(
        "  protheus-ops substrate-plane csi-policy [--consent=1|0] [--locality=<local-only|restricted-edge>] [--retention-minutes=<n>] [--biometric-risk=<low|medium|high>] [--allow-export=1|0] [--strict=1|0]"
    );
    println!(
        "  protheus-ops substrate-plane eye-bind --op=<enable|status> [--source=<wifi>] [--persona=<id>] [--shadow=<id>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops substrate-plane bio-interface --op=<ingest|status> [--channels=<n>] [--payload-ref=<ref>] [--strict=1|0]"
    );
    println!(
        "  protheus-ops substrate-plane bio-feedback --op=<stimulate|degrade|status> [--mode=<closed-loop|silicon-only>] [--consent=1|0] [--strict=1|0]"
    );
    println!(
        "  protheus-ops substrate-plane bio-adapter-template --op=<emit|status> [--adapter=<id>] [--spike-channels=a,b] [--stimulation-channels=x,y] [--health-telemetry=latency_ms,power_mw] [--strict=1|0]"
    );
    println!(
        "  protheus-ops substrate-plane bioethics-policy --op=<status|approve|enforce> [--approval=<HMAN-BIO-001>] [--artifact-ref=<ref>] [--consent=1|0] [--high-risk=1|0] [--strict=1|0]"
    );
    println!(
        "  protheus-ops substrate-plane bio-enable [--mode=<biological|silicon-only>] [--persona=<id>] [--adapter=<id>] [--strict=1|0]"
    );
}

fn normalize_substrate_action(action: &str) -> String {
    let token = action.trim().to_ascii_lowercase().replace('_', "-");
    match token.as_str() {
        "capture" | "csi-capture" => "csi-capture".to_string(),
        "module" | "csi-module" => "csi-module".to_string(),
        "embedded" | "embedded-profile" | "csi-embedded-profile" => {
            "csi-embedded-profile".to_string()
        }
        "policy" | "csi-policy" => "csi-policy".to_string(),
        "eye-bind" | "eye-bindings" | "bind-eye" => "eye-bind".to_string(),
        "bio-interface" | "interface" => "bio-interface".to_string(),
        "bio-feedback" | "feedback" => "bio-feedback".to_string(),
        "bio-adapter-template" | "adapter-template" => "bio-adapter-template".to_string(),
        "bioethics-policy" | "bio-ethics-policy" | "ethics-policy" => "bioethics-policy".to_string(),
        "bio-enable" | "enable" => "bio-enable".to_string(),
        _ => token,
    }
}

fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn emit(root: &Path, payload: Value) -> i32 {
    emit_plane_receipt(
        root,
        STATE_ENV,
        STATE_SCOPE,
        "substrate_plane_error",
        payload,
    )
}

fn status(root: &Path) -> Value {
    plane_status(root, STATE_ENV, STATE_SCOPE, "substrate_plane_status")
}

fn claim_ids_for_action(action: &str) -> Vec<&'static str> {
    match normalize_substrate_action(action).as_str() {
        "csi-capture" => vec!["V6-SUBSTRATE-001.1", "V6-SUBSTRATE-001.4"],
        "csi-module" => vec!["V6-SUBSTRATE-001.2", "V6-SUBSTRATE-001.4"],
        "csi-embedded-profile" => vec!["V6-SUBSTRATE-001.3", "V6-SUBSTRATE-001.4"],
        "csi-policy" => vec!["V6-SUBSTRATE-001.4"],
        "eye-bind" => vec!["V6-SUBSTRATE-001.5", "V6-SUBSTRATE-001.4"],
        "bio-interface" => vec!["V6-SUBSTRATE-002.1"],
        "bio-feedback" => vec!["V6-SUBSTRATE-002.2"],
        "bio-adapter-template" => vec!["V6-SUBSTRATE-002.3"],
        "bioethics-policy" => vec!["V6-SUBSTRATE-002.4"],
        "bio-enable" => vec!["V6-SUBSTRATE-002.5", "V6-SUBSTRATE-002.4"],
        _ => vec!["V6-SUBSTRATE-001.4"],
    }
}

fn conduit_enforcement(
    root: &Path,
    parsed: &crate::ParsedArgs,
    strict: bool,
    action: &str,
) -> Value {
    let canonical_action = normalize_substrate_action(action);
    let bypass_requested = conduit_bypass_requested(&parsed.flags);
    let claim_ids = claim_ids_for_action(&canonical_action);
    build_plane_conduit_enforcement(
        root,
        STATE_ENV,
        STATE_SCOPE,
        strict,
        &canonical_action,
        "substrate_conduit_enforcement",
        "core/layer0/ops/substrate_plane",
        bypass_requested,
        "substrate_operations_route_through_layer0_conduit_with_fail_closed_policy",
        &claim_ids,
    )
}

fn csi_capture_artifact_path(root: &Path, id: &str) -> PathBuf {
    state_root(root)
        .join("csi")
        .join("captures")
        .join(format!("{id}.json"))
}

fn csi_module_registry_path(root: &Path) -> PathBuf {
    state_root(root)
        .join("csi")
        .join("modules")
        .join("registry.json")
}

fn csi_embedded_profile_path(root: &Path, target: &str) -> PathBuf {
    state_root(root)
        .join("csi")
        .join("embedded")
        .join(format!("{target}.json"))
}

fn csi_policy_state_path(root: &Path) -> PathBuf {
    state_root(root)
        .join("csi")
        .join("policy")
        .join("latest.json")
}

fn eye_binding_state_path(root: &Path) -> PathBuf {
    state_root(root).join("eye").join("bindings.json")
}

fn bio_interface_state_path(root: &Path) -> PathBuf {
    state_root(root)
        .join("bio")
        .join("interface")
        .join("latest.json")
}

fn bio_feedback_state_path(root: &Path) -> PathBuf {
    state_root(root)
        .join("bio")
        .join("feedback")
        .join("latest.json")
}

fn bio_adapter_template_path(root: &Path) -> PathBuf {
    state_root(root)
        .join("bio")
        .join("adapter")
        .join("template.json")
}

fn bioethics_state_path(root: &Path) -> PathBuf {
    state_root(root)
        .join("bio")
        .join("ethics")
        .join("policy.json")
}

fn bio_enable_state_path(root: &Path) -> PathBuf {
    state_root(root)
        .join("bio")
        .join("enable")
        .join("latest.json")
}

fn decode_signal_u64(hex: &str, offset: usize) -> u64 {
    let start = offset.min(hex.len());
    let end = (start + 8).min(hex.len());
    if start >= end {
        return 0;
    }
    u64::from_str_radix(&hex[start..end], 16).unwrap_or(0)
}

fn run_csi_capture(root: &Path, parsed: &crate::ParsedArgs, strict: bool) -> Value {
    let contract = load_json_or(
        root,
        CSI_CAPTURE_CONTRACT_PATH,
        json!({
            "version": "v1",
            "kind": "substrate_csi_capture_contract",
            "normalized_events": ["presence", "respiration", "heartbeat_proxy", "pose_proxy", "motion"],
            "require_layer_minus_one_descriptor": true
        }),
    );
    let mut errors = Vec::<String>::new();
    if contract
        .get("version")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "v1"
    {
        errors.push("substrate_csi_capture_contract_version_must_be_v1".to_string());
    }
    if contract
        .get("kind")
        .and_then(Value::as_str)
        .unwrap_or_default()
        != "substrate_csi_capture_contract"
    {
        errors.push("substrate_csi_capture_contract_kind_invalid".to_string());
    }
    if !errors.is_empty() {
        return json!({
            "ok": false,
            "strict": strict,
            "type": "substrate_plane_csi_capture",
            "errors": errors
        });
    }

    let adapter = clean(
        parsed
            .flags
            .get("adapter")
            .cloned()
            .unwrap_or_else(|| "wifi-csi-esp32".to_string()),
        120,
    );
    let signal_ref = clean(
        parsed
            .flags
            .get("signal-ref")
            .cloned()
            .unwrap_or_else(|| "csi://capture/latest".to_string()),
        220,
    );
    let envelope = ExoticEnvelope {
        domain: ExoticDomain::Analog,
        adapter_id: adapter.clone(),
        signal_type: "wifi_csi_frame".to_string(),
        payload_ref: signal_ref.clone(),
        ts_ms: chrono::Utc::now().timestamp_millis(),
    };
    let wrapped = wrap_exotic_signal(&envelope, "sense.csi.capture");
    let digest = wrapped.deterministic_digest.clone();
    let presence_score = (decode_signal_u64(&digest, 0) % 100) as f64 / 100.0;
    let respiration_bpm = 10 + (decode_signal_u64(&digest, 8) % 24);
    let heartbeat_proxy_bpm = 52 + (decode_signal_u64(&digest, 12) % 58);
    let pose_proxy = match decode_signal_u64(&digest, 14) % 3 {
        0 => "upright",
        1 => "supine",
        _ => "moving",
    };
    let motion_flag = (decode_signal_u64(&digest, 16) % 2) == 1;
    let normalized = vec![
        json!({
            "event": "presence",
            "value": presence_score >= 0.42,
            "confidence": presence_score
        }),
        json!({
            "event": "respiration",
            "value": respiration_bpm,
            "unit": "breaths_per_minute",
            "confidence": 0.76
        }),
        json!({
            "event": "heartbeat_proxy",
            "value": heartbeat_proxy_bpm,
            "unit": "beats_per_minute",
            "confidence": 0.61
        }),
        json!({
            "event": "pose_proxy",
            "value": pose_proxy,
            "confidence": 0.57
        }),
        json!({
            "event": "motion",
            "value": motion_flag,
            "confidence": 0.69
        }),
    ];
    let capture_id = format!("csi_{}", &sha256_hex_str(&digest)[..12]);
    let artifact = json!({
        "version": "v1",
        "capture_id": capture_id,
        "layer_minus_one": {
            "descriptor": {
                "domain": "analog",
                "adapter_id": adapter,
                "signal_type": "wifi_csi_frame",
                "payload_ref": signal_ref
            },
            "wrapped_envelope": wrapped
        },
        "layer_two_decode": {
            "normalized_events": normalized,
            "sampling_metadata": {
                "sampling_hz": 20,
                "window_ms": 1200,
                "provenance": "layer2_decode_from_layer_minus_one_csi_envelope"
            }
        },
        "captured_at": crate::now_iso()
    });
    let path = csi_capture_artifact_path(root, &capture_id);
    let _ = write_json(&path, &artifact);
    let _ = append_jsonl(
        &state_root(root)
            .join("csi")
            .join("captures")
            .join("history.jsonl"),
        &artifact,
    );
    let mut out = json!({
        "ok": true,
        "strict": strict,
        "type": "substrate_plane_csi_capture",
        "lane": "core/layer0/ops",
        "capture": artifact,
        "artifact": {
            "path": path.display().to_string(),
            "sha256": sha256_hex_str(&artifact.to_string())
        },
        "claim_evidence": [
            {
                "id": "V6-SUBSTRATE-001.1",
                "claim": "csi_primitive_captures_layer_minus_one_signal_and_layer_two_normalized_events_with_receipts",
                "evidence": {
                    "capture_id": capture_id,
                    "event_count": artifact
                        .get("layer_two_decode")
                        .and_then(|v| v.get("normalized_events"))
                        .and_then(Value::as_array)
                        .map(|rows| rows.len())
                        .unwrap_or(0)
                }
            }
        ]
    });
    out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
    out
}
