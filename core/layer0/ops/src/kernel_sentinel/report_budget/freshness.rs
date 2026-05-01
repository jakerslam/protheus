// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use serde_json::{json, Value};
use std::time::{SystemTime, UNIX_EPOCH};

pub(super) const RELEASE_FINDING_STALE_AFTER_SECONDS: u64 = 1_800;

#[derive(Clone, Debug)]
pub(super) struct FindingFreshness {
    pub state: &'static str,
    pub signal: &'static str,
    pub current_truth: bool,
    pub stale_do_not_use: bool,
    pub age_seconds: Option<u64>,
}

impl FindingFreshness {
    pub(super) fn to_json(&self) -> Value {
        json!({
            "state": self.state,
            "signal": self.signal,
            "current_truth": self.current_truth,
            "stale_do_not_use": self.stale_do_not_use,
            "age_seconds": self.age_seconds,
            "stale_after_seconds": RELEASE_FINDING_STALE_AFTER_SECONDS,
        })
    }
}

pub(super) fn classify_finding_freshness(finding: &Value) -> FindingFreshness {
    if bool_field(finding, "stale").unwrap_or(false) {
        return freshness("stale_do_not_use", "explicit_stale_flag", false, true, age_seconds(finding));
    }
    if let Some(age) = age_seconds(finding) {
        let stale = age > RELEASE_FINDING_STALE_AFTER_SECONDS;
        return freshness(
            if stale { "stale_do_not_use" } else { "current_live_truth" },
            "bounded_age_seconds",
            !stale,
            stale,
            Some(age),
        );
    }
    if let Some(age) = evidence_freshness_age_seconds(finding) {
        let stale = age > RELEASE_FINDING_STALE_AFTER_SECONDS;
        return freshness(
            if stale { "stale_do_not_use" } else { "current_live_truth" },
            "evidence_ref_age_seconds",
            !stale,
            stale,
            Some(age),
        );
    }
    if let Some(generated_at) = u64_field(finding, "generated_at_epoch_seconds") {
        let age = unix_now().saturating_sub(generated_at);
        let stale = age > RELEASE_FINDING_STALE_AFTER_SECONDS;
        return freshness(
            if stale { "stale_do_not_use" } else { "current_live_truth" },
            "generated_at_epoch_seconds",
            !stale,
            stale,
            Some(age),
        );
    }
    if has_text_field(finding, "generated_at") || has_text_field(finding, "observed_at") {
        return freshness("freshness_unverified", "timestamp_without_age", false, false, None);
    }
    freshness("missing_freshness", "missing_freshness_metadata", false, false, None)
}

fn freshness(
    state: &'static str,
    signal: &'static str,
    current_truth: bool,
    stale_do_not_use: bool,
    age_seconds: Option<u64>,
) -> FindingFreshness {
    FindingFreshness {
        state,
        signal,
        current_truth,
        stale_do_not_use,
        age_seconds,
    }
}

fn age_seconds(finding: &Value) -> Option<u64> {
    ["freshness_age_seconds", "age_seconds", "source_artifact_age_seconds"]
        .iter()
        .find_map(|key| u64_field(finding, key))
}

fn evidence_freshness_age_seconds(finding: &Value) -> Option<u64> {
    finding["evidence"].as_array()?.iter().filter_map(Value::as_str).find_map(|reference| {
        reference
            .strip_prefix("freshness://age_seconds/")
            .and_then(|raw| raw.trim().parse::<u64>().ok())
    })
}

fn u64_field(finding: &Value, key: &str) -> Option<u64> {
    let details = finding.get("details").unwrap_or(&Value::Null);
    details
        .get(key)
        .or_else(|| finding.get(key))
        .and_then(|raw| {
            raw.as_u64()
                .or_else(|| raw.as_i64().and_then(|value| u64::try_from(value).ok()))
                .or_else(|| raw.as_str().and_then(|text| text.trim().parse::<u64>().ok()))
        })
}

fn bool_field(finding: &Value, key: &str) -> Option<bool> {
    let details = finding.get("details").unwrap_or(&Value::Null);
    details.get(key).or_else(|| finding.get(key)).and_then(|raw| {
        raw.as_bool().or_else(|| {
            raw.as_str().and_then(|text| match text.trim().to_ascii_lowercase().as_str() {
                "true" | "1" | "yes" => Some(true),
                "false" | "0" | "no" => Some(false),
                _ => None,
            })
        })
    })
}

fn has_text_field(finding: &Value, key: &str) -> bool {
    let details = finding.get("details").unwrap_or(&Value::Null);
    details
        .get(key)
        .or_else(|| finding.get(key))
        .and_then(Value::as_str)
        .map(|raw| !raw.trim().is_empty())
        .unwrap_or(false)
}

fn unix_now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0)
}
