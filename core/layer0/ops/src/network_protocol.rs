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
const WEB_SEARCH_AUTH_ENV_KEYS: &[&str] = &[
    "WEB_SEARCH_API_KEY",
    "TAVILY_API_KEY",
    "EXA_API_KEY",
    "PERPLEXITY_API_KEY",
    "BRAVE_API_KEY",
    "FIRECRAWL_API_KEY",
    "GOOGLE_SEARCH_API_KEY",
    "MOONSHOT_API_KEY",
    "XAI_API_KEY",
];
const WEB_FETCH_AUTH_ENV_KEYS: &[&str] = &["WEB_FETCH_API_KEY", "FIRECRAWL_API_KEY"];
#[path = "network_protocol_run.rs"]
mod network_protocol_run;

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

fn web_tooling_runtime_path(root: &Path) -> PathBuf {
    state_root(root).join("web_tooling_runtime.json")
}

fn first_present_env_key(keys: &[&str]) -> Option<String> {
    keys.iter().find_map(|key| {
        std::env::var(key)
            .ok()
            .map(|value| value.trim().to_string())
            .filter(|value| !value.is_empty())
            .map(|_| (*key).to_string())
    })
}

fn detect_web_tooling_auth_presence() -> Value {
    let search_key = first_present_env_key(WEB_SEARCH_AUTH_ENV_KEYS);
    let fetch_key = first_present_env_key(WEB_FETCH_AUTH_ENV_KEYS);
    json!({
        "search": {
            "present": search_key.is_some(),
            "source_env": search_key.unwrap_or_default()
        },
        "fetch": {
            "present": fetch_key.is_some(),
            "source_env": fetch_key.unwrap_or_default()
        }
    })
}

fn normalize_web_provider_token(raw: Option<&str>, fallback: &str) -> String {
    let candidate = raw
        .map(|value| clean(value.to_string(), 64))
        .unwrap_or_else(|| fallback.to_string())
        .trim()
        .to_ascii_lowercase()
        .chars()
        .filter(|c| c.is_ascii_alphanumeric() || matches!(c, '_' | '-' | '.'))
        .collect::<String>();
    if candidate.is_empty() {
        fallback.to_string()
    } else {
        candidate
    }
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

fn commit_ledger(
    root: &Path,
    mut ledger: Value,
    event_kind: &str,
    event_payload: Value,
) -> Result<Value, String> {
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
    obj.insert(
        "event_head".to_string(),
        event.get("event_hash").cloned().unwrap_or(Value::Null),
    );

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

pub fn deduct_nexus_balance(
    root: &Path,
    account: &str,
    amount: f64,
    reason: &str,
) -> Result<Value, String> {
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

fn gate_action(root: &Path, action: &str) -> bool {
    directive_kernel::action_allowed(root, action)
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    network_protocol_run::run(root, argv)
}

pub fn web_tooling_health_report(root: &Path, strict: bool) -> Value {
    network_protocol_run::web_tooling_health_report(root, strict)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn env_guard() -> std::sync::MutexGuard<'static, ()> {
        crate::test_env_guard()
    }

    fn temp_root(name: &str) -> PathBuf {
        let nonce = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!("infring_network_protocol_{name}_{nonce}"));
        fs::create_dir_all(&root).expect("mkdir");
        root
    }

    fn allow_tokenomics(root: &Path) {
        std::env::set_var("DIRECTIVE_KERNEL_SIGNING_KEY", "test-sign-key");
        assert_eq!(
            crate::directive_kernel::run(
                root,
                &[
                    "prime-sign".to_string(),
                    "--directive=allow:tokenomics".to_string(),
                    "--signer=operator".to_string(),
                ]
            ),
            0
        );
    }

    #[test]
    fn stake_updates_balance_and_writes_root() {
        let _guard = env_guard();
        let root = temp_root("stake");
        allow_tokenomics(&root);
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
    fn ignite_bitcoin_requires_directive_gate_when_apply_is_true() {
        let _guard = env_guard();
        let root = temp_root("ignite_gate");
        let denied = run(
            &root,
            &[
                "ignite-bitcoin".to_string(),
                "--seed=test".to_string(),
                "--apply=1".to_string(),
            ],
        );
        assert_eq!(denied, 2);
        let latest = read_json(&latest_path(&root)).expect("latest");
        assert_eq!(
            latest.get("error").and_then(Value::as_str),
            Some("directive_gate_denied")
        );

        allow_tokenomics(&root);
        let allowed = run(
            &root,
            &[
                "ignite-bitcoin".to_string(),
                "--seed=test".to_string(),
                "--apply=1".to_string(),
            ],
        );
        assert_eq!(allowed, 0);
        std::env::remove_var("DIRECTIVE_KERNEL_SIGNING_KEY");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn emission_requires_directive_gate() {
        let _guard = env_guard();
        let root = temp_root("emission_gate");
        let denied = run(
            &root,
            &[
                "emission".to_string(),
                "--height=210000".to_string(),
                "--halving-interval=210000".to_string(),
                "--initial-issuance=50".to_string(),
            ],
        );
        assert_eq!(denied, 2);
        let latest = read_json(&latest_path(&root)).expect("latest");
        assert_eq!(
            latest.get("error").and_then(Value::as_str),
            Some("directive_gate_denied")
        );

        allow_tokenomics(&root);
        let allowed = run(
            &root,
            &[
                "emission".to_string(),
                "--height=210000".to_string(),
                "--halving-interval=210000".to_string(),
                "--initial-issuance=50".to_string(),
            ],
        );
        assert_eq!(allowed, 0);
        std::env::remove_var("DIRECTIVE_KERNEL_SIGNING_KEY");
        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn zk_claim_strict_mode_fails_without_valid_proof() {
        let _guard = env_guard();
        let root = temp_root("zk");
        let denied = run(
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
        assert_eq!(denied, 2);
        let latest = read_json(&latest_path(&root)).expect("latest");
        assert_eq!(
            latest.get("error").and_then(Value::as_str),
            Some("directive_gate_denied")
        );

        allow_tokenomics(&root);
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
        std::env::remove_var("DIRECTIVE_KERNEL_SIGNING_KEY");
        let _ = fs::remove_dir_all(root);
    }
}
