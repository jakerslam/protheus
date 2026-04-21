// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/autonomy (authoritative).

use crate::{
    append_jsonl, clamp_num, clean_text, normalize_token, now_iso, read_json, resolve_runtime_path,
    round_to, write_json_atomic,
};
use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
struct EthicalPolicy {
    version: String,
    enabled: bool,
    shadow_only: bool,
    monoculture_warn_share: f64,
    high_impact_share: f64,
    maturity_min_for_prior_updates: f64,
    mirror_pressure_warn: f64,
    value_priors: BTreeMap<String, f64>,
    max_prior_delta_per_run: f64,
    weaver_latest_path: PathBuf,
    mirror_latest_path: PathBuf,
}

#[derive(Clone, Debug)]
struct RuntimePaths {
    latest_path: PathBuf,
    history_path: PathBuf,
    receipts_path: PathBuf,
    priors_state_path: PathBuf,
}

fn hash10(seed: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(seed.as_bytes());
    hex::encode(hasher.finalize())[..10].to_string()
}

fn default_policy(root: &Path) -> EthicalPolicy {
    let mut priors = BTreeMap::new();
    priors.insert("adaptive_value".to_string(), 0.2);
    priors.insert("user_value".to_string(), 0.2);
    priors.insert("quality".to_string(), 0.2);
    priors.insert("learning".to_string(), 0.2);
    priors.insert("delivery".to_string(), 0.2);

    EthicalPolicy {
        version: "1.0".to_string(),
        enabled: true,
        shadow_only: true,
        monoculture_warn_share: 0.68,
        high_impact_share: 0.72,
        maturity_min_for_prior_updates: 0.65,
        mirror_pressure_warn: 0.55,
        value_priors: priors,
        max_prior_delta_per_run: 0.03,
        weaver_latest_path: resolve_runtime_path(
            root,
            Some("local/state/autonomy/weaver/latest.json"),
            "local/state/autonomy/weaver/latest.json",
        ),
        mirror_latest_path: resolve_runtime_path(
            root,
            Some("local/state/autonomy/mirror_organ/latest.json"),
            "local/state/autonomy/mirror_organ/latest.json",
        ),
    }
}

fn policy_path(root: &Path, explicit: Option<&Path>) -> PathBuf {
    explicit
        .map(|p| p.to_path_buf())
        .or_else(|| {
            std::env::var("ETHICAL_REASONING_POLICY_PATH")
                .ok()
                .map(PathBuf::from)
        })
        .unwrap_or_else(|| {
            resolve_runtime_path(
                root,
                Some("config/ethical_reasoning_policy.json"),
                "config/ethical_reasoning_policy.json",
            )
        })
}

fn load_policy(root: &Path, explicit: Option<&Path>) -> EthicalPolicy {
    let mut policy = default_policy(root);
    let p = policy_path(root, explicit);
    let raw = read_json(&p);
    let obj = raw.as_object();

    if let Some(v) = obj
        .and_then(|m| m.get("version"))
        .and_then(Value::as_str)
        .map(|s| clean_text(s, 40))
    {
        if !v.is_empty() {
            policy.version = v;
        }
    }
    if let Some(v) = obj.and_then(|m| m.get("enabled")).and_then(Value::as_bool) {
        policy.enabled = v;
    }
    if let Some(v) = obj
        .and_then(|m| m.get("shadow_only"))
        .and_then(Value::as_bool)
    {
        policy.shadow_only = v;
    }

    if let Some(th) = obj
        .and_then(|m| m.get("thresholds"))
        .and_then(Value::as_object)
    {
        policy.monoculture_warn_share = clamp_num(
            th.get("monoculture_warn_share")
                .and_then(Value::as_f64)
                .unwrap_or(policy.monoculture_warn_share),
            0.3,
            0.99,
            policy.monoculture_warn_share,
        );
        policy.high_impact_share = clamp_num(
            th.get("high_impact_share")
                .and_then(Value::as_f64)
                .unwrap_or(policy.high_impact_share),
            0.3,
            0.99,
            policy.high_impact_share,
        );
        policy.maturity_min_for_prior_updates = clamp_num(
            th.get("maturity_min_for_prior_updates")
                .and_then(Value::as_f64)
                .unwrap_or(policy.maturity_min_for_prior_updates),
            0.0,
            1.0,
            policy.maturity_min_for_prior_updates,
        );
        policy.mirror_pressure_warn = clamp_num(
            th.get("mirror_pressure_warn")
                .and_then(Value::as_f64)
                .unwrap_or(policy.mirror_pressure_warn),
            0.0,
            1.0,
            policy.mirror_pressure_warn,
        );
    }

    if let Some(priors) = obj
        .and_then(|m| m.get("value_priors"))
        .and_then(Value::as_object)
    {
        let mut next = BTreeMap::new();
        for (k, v) in priors {
            let key = normalize_token(k, 80);
            if key.is_empty() {
                continue;
            }
            next.insert(key, clamp_num(v.as_f64().unwrap_or(0.0), 0.0, 1.0, 0.0));
        }
        if !next.is_empty() {
            policy.value_priors = next;
        }
    }

    policy.max_prior_delta_per_run = clamp_num(
        obj.and_then(|m| m.get("max_prior_delta_per_run"))
            .and_then(Value::as_f64)
            .unwrap_or(policy.max_prior_delta_per_run),
        0.001,
        0.2,
        policy.max_prior_delta_per_run,
    );

    if let Some(integration) = obj
        .and_then(|m| m.get("integration"))
        .and_then(Value::as_object)
    {
        policy.weaver_latest_path = resolve_runtime_path(
            root,
            integration
                .get("weaver_latest_path")
                .and_then(Value::as_str),
            "local/state/autonomy/weaver/latest.json",
        );
        policy.mirror_latest_path = resolve_runtime_path(
            root,
            integration
                .get("mirror_latest_path")
                .and_then(Value::as_str),
            "local/state/autonomy/mirror_organ/latest.json",
        );
    }

    policy
}

fn resolve_runtime_paths(root: &Path, state_dir: Option<&Path>) -> RuntimePaths {
    let dir = state_dir
        .map(|p| p.to_path_buf())
        .or_else(|| {
            std::env::var("ETHICAL_REASONING_STATE_DIR")
                .ok()
                .map(PathBuf::from)
        })
        .unwrap_or_else(|| {
            resolve_runtime_path(
                root,
                Some("local/state/autonomy/ethical_reasoning"),
                "local/state/autonomy/ethical_reasoning",
            )
        });

    RuntimePaths {
        latest_path: dir.join("latest.json"),
        history_path: dir.join("history.jsonl"),
        receipts_path: dir.join("tradeoff_receipts.jsonl"),
        priors_state_path: dir.join("value_priors.json"),
    }
}
