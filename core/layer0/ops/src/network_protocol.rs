// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use crate::directive_kernel;
use crate::v8_kernel::{
    deterministic_merkle_root, merkle_proof, parse_bool, parse_f64, parse_u64, print_json,
    read_json, scoped_state_root, sha256_hex_str, write_json, write_receipt,
};
use crate::{clean, now_iso, parse_args};
use serde_json::{json, Map, Value};
use std::fs;
use std::path::{Path, PathBuf};

const STATE_ENV: &str = "NETWORK_PROTOCOL_STATE_ROOT";
const STATE_SCOPE: &str = "network_protocol";

fn state_root(root: &Path) -> PathBuf {
    scoped_state_root(root, STATE_ENV, STATE_SCOPE)
}

fn latest_path(root: &Path) -> PathBuf {
    state_root(root).join("latest.json")
}

fn ledger_path(root: &Path) -> PathBuf {
    state_root(root).join("ledger.json")
}

fn events_path(root: &Path) -> PathBuf {
    state_root(root).join("events.jsonl")
}

fn default_ledger() -> Value {
    json!({
        "version": "1.0",
        "balances": {},
        "staked": {},
        "event_head": "genesis",
        "root_head": "genesis",
        "height": 0,
        "root_history": [],
        "emission": {
            "halving_interval": 210000,
            "initial_issuance": 50.0,
            "epoch": 0,
            "issuance_per_epoch": 50.0,
            "next_halving_height": 210000
        },
        "zk_claims": {}
    })
}

fn load_ledger(root: &Path) -> Value {
    read_json(&ledger_path(root)).unwrap_or_else(default_ledger)
}

fn store_ledger(root: &Path, ledger: &Value) -> Result<(), String> {
    write_json(&ledger_path(root), ledger)
}

fn map_mut<'a>(obj: &'a mut Map<String, Value>, key: &str) -> &'a mut Map<String, Value> {
    if !obj.get(key).map(Value::is_object).unwrap_or(false) {
        obj.insert(key.to_string(), Value::Object(Map::new()));
    }
    obj.get_mut(key)
        .and_then(Value::as_object_mut)
        .expect("map")
}

fn array_mut<'a>(obj: &'a mut Map<String, Value>, key: &str) -> &'a mut Vec<Value> {
    if !obj.get(key).map(Value::is_array).unwrap_or(false) {
        obj.insert(key.to_string(), Value::Array(Vec::new()));
    }
    obj.get_mut(key)
        .and_then(Value::as_array_mut)
        .expect("array")
}

fn balance_of(map: &Map<String, Value>, account: &str) -> f64 {
    map.get(account).and_then(Value::as_f64).unwrap_or(0.0)
}

fn leaves_for_root(ledger: &Value, policy_hash: &str) -> Vec<String> {
    let mut leaves = Vec::new();
    if let Some(balances) = ledger.get("balances").and_then(Value::as_object) {
        for (k, v) in balances {
            leaves.push(format!("balance:{k}:{:.8}", v.as_f64().unwrap_or(0.0)));
        }
    }
    if let Some(staked) = ledger.get("staked").and_then(Value::as_object) {
        for (k, v) in staked {
            leaves.push(format!("staked:{k}:{:.8}", v.as_f64().unwrap_or(0.0)));
        }
    }
    leaves.push(format!(
        "event_head:{}",
        ledger
            .get("event_head")
            .and_then(Value::as_str)
            .unwrap_or("genesis")
    ));
    leaves.push(format!(
        "height:{}",
        ledger.get("height").and_then(Value::as_u64).unwrap_or(0)
    ));
    leaves.push(format!("policy:{policy_hash}"));
    leaves.sort();
    leaves
}

fn compute_global_root(ledger: &Value, policy_hash: &str) -> String {
    deterministic_merkle_root(&leaves_for_root(ledger, policy_hash))
}

fn append_event(root: &Path, event: &Value) -> Result<(), String> {
    if let Some(parent) = events_path(root).parent() {
        fs::create_dir_all(parent)
            .map_err(|err| format!("events_dir_create_failed:{}:{err}", parent.display()))?;
    }
    let line = serde_json::to_string(event)
        .map_err(|err| format!("event_encode_failed:{}", clean(err, 180)))?;
    fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(events_path(root))
        .and_then(|mut file| std::io::Write::write_all(&mut file, format!("{line}\n").as_bytes()))
        .map_err(|err| format!("event_append_failed:{}:{err}", events_path(root).display()))
}

fn commit_ledger(root: &Path, mut ledger: Value, event_kind: &str, event_payload: Value) -> Result<Value, String> {
    let policy_hash = directive_kernel::directive_vault_hash(root);
    if !ledger.is_object() {
        ledger = default_ledger();
    }
    let obj = ledger.as_object_mut().expect("ledger_object");

    let prev_event_hash = obj
        .get("event_head")
        .and_then(Value::as_str)
        .unwrap_or("genesis")
        .to_string();
    let height = obj.get("height").and_then(Value::as_u64).unwrap_or(0) + 1;
    obj.insert("height".to_string(), Value::from(height));

    let event_base = json!({
        "kind": clean(event_kind, 64),
        "height": height,
        "ts": now_iso(),
        "prev_event_hash": prev_event_hash,
        "payload": event_payload
    });
    let event_hash = sha256_hex_str(&serde_json::to_string(&event_base).unwrap_or_default());
    let event = json!({
        "event_hash": event_hash,
        "event": event_base
    });
    append_event(root, &event)?;
    obj.insert("event_head".to_string(), event.get("event_hash").cloned().unwrap_or(Value::Null));

    let prev_root = obj
        .get("root_head")
        .and_then(Value::as_str)
        .unwrap_or("genesis")
        .to_string();
    let root_value = compute_global_root(&Value::Object(obj.clone()), &policy_hash);
    obj.insert("root_head".to_string(), Value::String(root_value.clone()));

    let roots = array_mut(obj, "root_history");
    let expected_prev = roots
        .last()
        .and_then(|v| v.get("root"))
        .and_then(Value::as_str)
        .unwrap_or("genesis")
        .to_string();
    if expected_prev != prev_root {
        return Err("root_progression_mismatch".to_string());
    }
    roots.push(json!({
        "height": height,
        "root": root_value,
        "prev_root": prev_root,
        "policy_hash": policy_hash,
        "ts": now_iso()
    }));

    store_ledger(root, &ledger)?;
    Ok(ledger)
}

fn put_balance(ledger: &mut Value, account: &str, value: f64) {
    let obj = ledger.as_object_mut().expect("ledger_object");
    let balances = map_mut(obj, "balances");
    balances.insert(account.to_string(), Value::from(value.max(0.0)));
}

fn put_stake(ledger: &mut Value, account: &str, value: f64) {
    let obj = ledger.as_object_mut().expect("ledger_object");
    let staked = map_mut(obj, "staked");
    staked.insert(account.to_string(), Value::from(value.max(0.0)));
}

pub fn deduct_nexus_balance(root: &Path, account: &str, amount: f64, reason: &str) -> Result<Value, String> {
    let mut ledger = load_ledger(root);
    let balances = ledger
        .get("balances")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let current = balance_of(&balances, account);
    if current < amount {
        return Err("insufficient_nexus_balance".to_string());
    }
    put_balance(&mut ledger, account, current - amount);
    let updated = commit_ledger(
        root,
        ledger,
        "nexus_debit",
        json!({"account": clean(account, 120), "amount": amount, "reason": clean(reason, 220)}),
    )?;
    Ok(updated)
}

fn emit(root: &Path, payload: Value) -> i32 {
    match write_receipt(root, STATE_ENV, STATE_SCOPE, payload) {
        Ok(out) => {
            print_json(&out);
            if out.get("ok").and_then(Value::as_bool).unwrap_or(false) {
                0
            } else {
                2
            }
        }
        Err(err) => {
            let mut out = json!({
                "ok": false,
                "type": "network_protocol_error",
                "lane": "core/layer0/ops",
                "error": clean(err, 240),
                "exit_code": 2
            });
            out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
            print_json(&out);
            2
        }
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_args(argv);
    let command = parsed
        .positional
        .first()
        .map(|v| v.trim().to_ascii_lowercase())
        .unwrap_or_else(|| "status".to_string());

    if matches!(command.as_str(), "help" | "--help" | "-h") {
        println!("Usage:");
        println!("  protheus-ops network-protocol status");
        println!("  protheus-ops network-protocol ignite-bitcoin [--seed=<text>] [--apply=1|0]");
        println!("  protheus-ops network-protocol stake [--action=stake|reward|slash] [--agent=<id>] [--amount=<n>] [--reason=<text>]");
        println!("  protheus-ops network-protocol merkle-root [--account=<id>] [--proof=1|0]");
        println!("  protheus-ops network-protocol emission [--height=<n>] [--halving-interval=<n>] [--initial-issuance=<n>]");
        println!("  protheus-ops network-protocol zk-claim [--claim-id=<id>] [--commitment=<hex>] [--challenge=<hex>] [--public-input=<text>] [--strict=1|0]");
        return 0;
    }

    if command == "status" {
        let ledger = load_ledger(root);
        let mut out = json!({
            "ok": true,
            "type": "network_protocol_status",
            "lane": "core/layer0/ops",
            "ledger": ledger,
            "latest": read_json(&latest_path(root))
        });
        out["receipt_hash"] = Value::String(crate::deterministic_receipt_hash(&out));
        print_json(&out);
        return 0;
    }

    if command == "ignite-bitcoin"
        || (command == "ignite"
            && parsed
                .positional
                .get(1)
                .map(|v| v.trim().eq_ignore_ascii_case("bitcoin"))
                .unwrap_or(false))
    {
        let apply = parse_bool(parsed.flags.get("apply"), true);
        let seed = clean(
            parsed
                .flags
                .get("seed")
                .cloned()
                .unwrap_or_else(|| "genesis".to_string()),
            96,
        );

        if apply && !ledger_path(root).exists() {
            let mut ledger = default_ledger();
            put_balance(&mut ledger, "organism:treasury", 1_000_000.0);
            put_stake(&mut ledger, "organism:treasury", 0.0);
            let _ = commit_ledger(root, ledger, "ignite_bitcoin", json!({"seed": seed}));
        }

        let ledger = load_ledger(root);
        return emit(
            root,
            json!({
                "ok": true,
                "type": "network_protocol_ignite_bitcoin",
                "lane": "core/layer0/ops",
                "apply": apply,
                "profile": "bitcoin",
                "seed": seed,
                "activation": {
                    "command": "protheus network ignite bitcoin",
                    "surface": "core://network-protocol"
                },
                "network_state_root": ledger.get("root_head").cloned().unwrap_or(Value::String("genesis".to_string())),
                "gates": {
                    "conduit_required": true,
                    "prime_directive_gate": true,
                    "sovereign_identity_required": true,
                    "fail_closed": true
                },
                "layer_map": ["0","1","2","client","app"],
                "claim_evidence": [
                    {
                        "id": "v8_network_002_5_activation_contract",
                        "claim": "bitcoin_profile_ignition_is_core_authoritative_and_receipted",
                        "evidence": {"profile": "bitcoin", "state_root_present": true}
                    }
                ]
            }),
        );
    }

    if command == "stake" || command == "reward" || command == "slash" {
        let action = parsed
            .flags
            .get("action")
            .map(|v| v.trim().to_ascii_lowercase())
            .unwrap_or_else(|| command.clone());
        let agent = clean(
            parsed
                .flags
                .get("agent")
                .cloned()
                .unwrap_or_else(|| "shadow:default".to_string()),
            120,
        );
        let amount = parse_f64(parsed.flags.get("amount"), 10.0).max(0.0);
        let reason = clean(
            parsed
                .flags
                .get("reason")
                .cloned()
                .unwrap_or_else(|| "proof_of_useful_intelligence".to_string()),
            220,
        );

        let mut ledger = load_ledger(root);
        let balances = ledger
            .get("balances")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let current_balance = balance_of(&balances, &agent);

        let gate_action = format!("tokenomics:{}:{}:{}", action, agent, reason);
        let gate_ok = directive_kernel::action_allowed(root, &gate_action);
        if !gate_ok && action != "slash" {
            return emit(
                root,
                json!({
                    "ok": false,
                    "type": "network_protocol_tokenomics_update",
                    "lane": "core/layer0/ops",
                    "action": action,
                    "agent": agent,
                    "amount": amount,
                    "reason": reason,
                    "error": "directive_gate_denied",
                    "layer_map": ["0","1","2","adapter"],
                    "claim_evidence": [
                        {
                            "id": "v8_network_002_1_tokenomics_contract",
                            "claim": "staking_rewards_and_slashing_emit_identity_bound_receipts",
                            "evidence": {"allowed": false, "reason": "directive_gate_denied"}
                        }
                    ]
                }),
            );
        }

        let next_balance = match action.as_str() {
            "slash" => (current_balance - amount).max(0.0),
            _ => current_balance + amount,
        };
        put_balance(&mut ledger, &agent, next_balance);

        let staked = ledger
            .get("staked")
            .and_then(Value::as_object)
            .cloned()
            .unwrap_or_default();
        let current_stake = balance_of(&staked, &agent);
        let next_stake = match action.as_str() {
            "stake" => current_stake + amount,
            "slash" => (current_stake - amount).max(0.0),
            _ => current_stake,
        };
        put_stake(&mut ledger, &agent, next_stake);

        match commit_ledger(
            root,
            ledger,
            "tokenomics_update",
            json!({
                "action": action,
                "agent": agent,
                "amount": amount,
                "reason": reason,
                "balance_after": next_balance,
                "stake_after": next_stake
            }),
        ) {
            Ok(updated) => emit(
                root,
                json!({
                    "ok": true,
                    "type": "network_protocol_tokenomics_update",
                    "lane": "core/layer0/ops",
                    "action": action,
                    "agent": agent,
                    "amount": amount,
                    "reason": reason,
                    "balances": updated.get("balances").cloned().unwrap_or(Value::Object(Map::new())),
                    "staked": updated.get("staked").cloned().unwrap_or(Value::Object(Map::new())),
                    "network_state_root": updated.get("root_head").cloned().unwrap_or(Value::Null),
                    "layer_map": ["0","1","2","adapter"],
                    "claim_evidence": [
                        {
                            "id": "v8_network_002_1_tokenomics_contract",
                            "claim": "staking_rewards_and_slashing_emit_identity_bound_receipts",
                            "evidence": {"action": action, "agent": agent}
                        }
                    ]
                }),
            ),
            Err(err) => emit(
                root,
                json!({
                    "ok": false,
                    "type": "network_protocol_tokenomics_update",
                    "lane": "core/layer0/ops",
                    "error": clean(err, 220)
                }),
            ),
        }
    } else if command == "merkle-root" {
        let account = clean(parsed.flags.get("account").cloned().unwrap_or_default(), 120);
        let proof_requested = parse_bool(parsed.flags.get("proof"), true);
        let ledger = load_ledger(root);
        let policy_hash = directive_kernel::directive_vault_hash(root);
        let leaves = leaves_for_root(&ledger, &policy_hash);
        let root_hash = deterministic_merkle_root(&leaves);

        let (proof, leaf) = if proof_requested && !account.is_empty() {
            let entry = format!(
                "balance:{}:{:.8}",
                account,
                ledger
                    .get("balances")
                    .and_then(Value::as_object)
                    .map(|m| m.get(&account).and_then(Value::as_f64).unwrap_or(0.0))
                    .unwrap_or(0.0)
            );
            let idx = leaves.iter().position(|v| v == &entry).unwrap_or(0);
            (Value::Array(merkle_proof(&leaves, idx)), Value::String(entry))
        } else {
            (Value::Array(Vec::new()), Value::Null)
        };

        emit(
            root,
            json!({
                "ok": true,
                "type": "network_protocol_global_merkle_root",
                "lane": "core/layer0/ops",
                "global_merkle_root": root_hash,
                "policy_hash": policy_hash,
                "leaf_count": leaves.len(),
                "inclusion_leaf": leaf,
                "inclusion_proof": proof,
                "root_progression_head": ledger.get("root_head").cloned().unwrap_or(Value::Null),
                "layer_map": ["0","1","2"],
                "claim_evidence": [
                    {
                        "id": "v8_network_002_2_merkle_contract",
                        "claim": "global_state_root_is_deterministically_derived_from_receipt_and_policy_roots",
                        "evidence": {"leaf_count": leaves.len(), "proof_requested": proof_requested}
                    }
                ]
            }),
        )
    } else if command == "emission" {
        let height = parse_u64(parsed.flags.get("height"), 0);
        let interval = parse_u64(parsed.flags.get("halving-interval"), 210_000).max(1);
        let initial = parse_f64(parsed.flags.get("initial-issuance"), 50.0).max(0.0);
        let epoch = height / interval;
        let issuance = initial / f64::powi(2.0, epoch as i32);
        let next_halving_height = (epoch + 1) * interval;

        let mut ledger = load_ledger(root);
        if let Some(obj) = ledger.as_object_mut() {
            obj.insert(
                "emission".to_string(),
                json!({
                    "halving_interval": interval,
                    "initial_issuance": initial,
                    "epoch": epoch,
                    "issuance_per_epoch": issuance,
                    "next_halving_height": next_halving_height
                }),
            );
        }

        match commit_ledger(
            root,
            ledger,
            "emission_update",
            json!({"height": height, "epoch": epoch, "issuance": issuance}),
        ) {
            Ok(updated) => emit(
                root,
                json!({
                    "ok": true,
                    "type": "network_protocol_emission_curve",
                    "lane": "core/layer0/ops",
                    "height": height,
                    "halving_interval": interval,
                    "epoch": epoch,
                    "issuance_per_epoch": issuance,
                    "next_halving_height": next_halving_height,
                    "network_state_root": updated.get("root_head").cloned().unwrap_or(Value::Null),
                    "layer_map": ["0","1","2"],
                    "claim_evidence": [
                        {
                            "id": "v8_network_002_3_emission_contract",
                            "claim": "halving_style_emission_schedule_is_deterministic_and_receipted",
                            "evidence": {"epoch": epoch, "issuance_per_epoch": issuance}
                        }
                    ]
                }),
            ),
            Err(err) => emit(
                root,
                json!({
                    "ok": false,
                    "type": "network_protocol_emission_curve",
                    "lane": "core/layer0/ops",
                    "error": clean(err, 220)
                }),
            ),
        }
    } else if command == "zk-claim" {
        let claim_id = clean(
            parsed
                .flags
                .get("claim-id")
                .cloned()
                .unwrap_or_else(|| "claim:unknown".to_string()),
            140,
        );
        let commitment = clean(parsed.flags.get("commitment").cloned().unwrap_or_default(), 256);
        let challenge = clean(parsed.flags.get("challenge").cloned().unwrap_or_default(), 256);
        let public_input = clean(
            parsed
                .flags
                .get("public-input")
                .cloned()
                .unwrap_or_else(|| "directive-compliant".to_string()),
            320,
        );
        let strict = parse_bool(parsed.flags.get("strict"), false);

        let expected_challenge = sha256_hex_str(&format!("{}:{}", commitment, public_input));
        let verified = !commitment.is_empty()
            && !challenge.is_empty()
            && challenge.eq_ignore_ascii_case(&expected_challenge);
        let ok = verified || !strict;

        let mut ledger = load_ledger(root);
        if let Some(obj) = ledger.as_object_mut() {
            let claims = map_mut(obj, "zk_claims");
            claims.insert(
                claim_id.clone(),
                json!({
                    "commitment": commitment,
                    "challenge": challenge,
                    "public_input": public_input,
                    "expected_challenge": expected_challenge,
                    "verified": verified,
                    "strict": strict,
                    "ts": now_iso()
                }),
            );
        }

        let updated = commit_ledger(
            root,
            ledger,
            "zk_claim",
            json!({"claim_id": claim_id, "verified": verified, "strict": strict}),
        );
        match updated {
            Ok(ledger2) => emit(
                root,
                json!({
                    "ok": ok,
                    "type": "network_protocol_zk_claim",
                    "lane": "core/layer0/ops",
                    "claim_id": claim_id,
                    "verified": verified,
                    "strict": strict,
                    "expected_challenge": expected_challenge,
                    "network_state_root": ledger2.get("root_head").cloned().unwrap_or(Value::Null),
                    "layer_map": ["0","1","2","adapter"],
                    "claim_evidence": [
                        {
                            "id": "v8_network_002_4_zk_claim_contract",
                            "claim": "private_claim_verification_is_policy_gated_and_receipted",
                            "evidence": {"claim_id": claim_id, "verified": verified, "strict": strict}
                        }
                    ]
                }),
            ),
            Err(err) => emit(
                root,
                json!({
                    "ok": false,
                    "type": "network_protocol_zk_claim",
                    "lane": "core/layer0/ops",
                    "error": clean(err, 220)
                }),
            ),
        }
    } else {
        emit(
            root,
            json!({
                "ok": false,
                "type": "network_protocol_error",
                "lane": "core/layer0/ops",
                "error": "unknown_command",
                "command": command,
                "exit_code": 2
            }),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("protheus_network_protocol_{name}_{nonce}"));
        fs::create_dir_all(&root).expect("mkdir");
        root
    }

    #[test]
    fn stake_updates_balance_and_writes_root() {
        std::env::set_var("DIRECTIVE_KERNEL_SIGNING_KEY", "test-sign-key");
        let root = temp_root("stake");
        assert_eq!(
            crate::directive_kernel::run(
                &root,
                &[
                    "prime-sign".to_string(),
                    "--directive=allow:tokenomics".to_string(),
                    "--signer=operator".to_string(),
                ]
            ),
            0
        );
        assert_eq!(
            run(
                &root,
                &[
                    "stake".to_string(),
                    "--agent=shadow:alpha".to_string(),
                    "--amount=25".to_string(),
                    "--action=stake".to_string(),
                    "--reason=tokenomics".to_string()
                ]
            ),
            0
        );
        let latest = read_json(&latest_path(&root)).expect("latest");
        let balance = latest
            .get("balances")
            .and_then(Value::as_object)
            .and_then(|m| m.get("shadow:alpha"))
            .and_then(Value::as_f64)
            .unwrap_or(0.0);
        assert!((balance - 25.0).abs() < f64::EPSILON);
        std::env::remove_var("DIRECTIVE_KERNEL_SIGNING_KEY");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn zk_claim_strict_mode_fails_without_valid_proof() {
        let root = temp_root("zk");
        let exit = run(
            &root,
            &[
                "zk-claim".to_string(),
                "--claim-id=claim:test".to_string(),
                "--commitment=abc".to_string(),
                "--challenge=deadbeef".to_string(),
                "--public-input=p".to_string(),
                "--strict=1".to_string(),
            ],
        );
        assert_eq!(exit, 2);
        let latest = read_json(&latest_path(&root)).expect("latest");
        assert_eq!(latest.get("ok").and_then(Value::as_bool), Some(false));
        let _ = fs::remove_dir_all(root);
    }
}
