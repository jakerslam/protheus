// Layer ownership: orchestration (run observation lifecycle only).
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;
use std::fs;
use std::io::{self, Write};
use std::path::Path;

pub const DEFAULT_POLICY_PATH: &str = "orchestration/config/run_observation_lifecycle_policy.json";
pub const DEFAULT_LEDGER_PATH: &str = "local/state/ops/run_observation_lifecycle/events.jsonl";
pub const DEFAULT_HOT_WINDOW_PATH: &str =
    "local/state/ops/run_observation_lifecycle/hot_window.json";
pub const DEFAULT_ARCHIVE_PATH: &str = "local/state/ops/run_observation_lifecycle/archive.json";
pub const DEFAULT_SUMMARY_PATH: &str =
    "core/local/artifacts/run_observation_lifecycle_current.json";

#[derive(Debug, Clone)]
pub struct ObservationLifecyclePaths {
    pub ledger_path: String,
    pub hot_window_path: String,
    pub archive_path: String,
    pub summary_path: String,
}

impl Default for ObservationLifecyclePaths {
    fn default() -> Self {
        Self {
            ledger_path: DEFAULT_LEDGER_PATH.to_string(),
            hot_window_path: DEFAULT_HOT_WINDOW_PATH.to_string(),
            archive_path: DEFAULT_ARCHIVE_PATH.to_string(),
            summary_path: DEFAULT_SUMMARY_PATH.to_string(),
        }
    }
}

pub fn load_policy_or_default(path: &str) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(default_policy)
}

pub fn default_policy() -> Value {
    json!({
        "schema_version": 1,
        "canonical_owner": "orchestration",
        "policy_id": "run_observation_lifecycle_v1",
        "purpose": "standard compact observation records plus bounded lifecycle views for workflows, tools, evals, and agent runs",
        "paths": {
            "compact_ledger": DEFAULT_LEDGER_PATH,
            "hot_ring_buffer": DEFAULT_HOT_WINDOW_PATH,
            "failure_lifecycle_archive": DEFAULT_ARCHIVE_PATH,
            "current_summary": DEFAULT_SUMMARY_PATH
        },
        "retention": {
            "hot_ring_max_records": 500,
            "subject_history_tail_max": 12,
            "archive_after_consecutive_passes": 2
        },
        "data_minimization": {
            "store_raw_user_content": false,
            "store_raw_tool_payloads": false,
            "store_full_chat_transcripts": false,
            "store_artifact_refs_hashes_and_short_previews_only": true
        }
    })
}

pub fn policy_path_string(policy: &Value, path: &[&str], default: &str) -> String {
    let mut cursor = policy;
    for segment in path {
        let Some(next) = cursor.get(*segment) else {
            return default.to_string();
        };
        cursor = next;
    }
    cursor
        .as_str()
        .map(clean_text)
        .filter(|raw| !raw.is_empty())
        .unwrap_or_else(|| default.to_string())
}

pub fn policy_u64(policy: &Value, path: &[&str], default: u64) -> u64 {
    let mut cursor = policy;
    for segment in path {
        let Some(next) = cursor.get(*segment) else {
            return default;
        };
        cursor = next;
    }
    cursor.as_u64().unwrap_or(default)
}

pub fn stable_hash_hex(raw: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(raw.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub fn research_golden_observation_events(report: &Value, meta: &Value) -> Vec<Value> {
    let generated_at = str_at(report, &["generated_at"], "");
    let run_id = str_at(meta, &["run_id"], "");
    let commit_sha = str_at(meta, &["commit_sha"], "unknown");
    let model_ref = str_at(meta, &["model_ref"], "selected_model");
    let cache_mode = str_at(meta, &["cache_mode"], "unknown");
    let artifact_refs = string_array_at(meta, &["artifact_refs"]);
    report
        .get("cases")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .map(|row| {
            let case_id = str_at(row, &["case_id"], "unknown_case");
            let workflow_id = "research_synthesize_verify";
            let response_preview = str_at(row, &["response_preview"], "");
            let response_hash = stable_hash_hex(response_preview.as_str());
            let first_failed_checkpoint = str_at(
                row,
                &["gate_transition_diagnostics", "first_failed_checkpoint"],
                "",
            );
            let failure_boundary = str_at(
                row,
                &["gate_transition_diagnostics", "inferred_failure_boundary"],
                "",
            );
            let synthesis_failure_class = str_at(
                row,
                &["gate_transition_diagnostics", "synthesis_failure_class"],
                "",
            );
            let failure_signatures = failure_signatures_for_row(row);
            let pass = bool_at(row, &["pass"], false);
            let subject_key = format!("workflow:{workflow_id}/case:{case_id}");
            let event_seed = json!({
                "source_kind": "research_golden_eval",
                "run_id": run_id,
                "subject_key": subject_key,
                "pass": pass,
                "score": u64_at(row, &["score"], 0),
                "failure_signatures": failure_signatures,
                "response_hash": response_hash
            });
            json!({
                "type": "run_observation_event",
                "schema_version": 1,
                "event_id": stable_hash_hex(&event_seed.to_string()),
                "observed_at": generated_at,
                "source_kind": "research_golden_eval",
                "workflow_id": workflow_id,
                "case_id": case_id,
                "category": str_at(row, &["category"], "unknown"),
                "subject_key": subject_key,
                "run": {
                    "run_id": run_id,
                    "mode": str_at(report, &["mode"], ""),
                    "commit_sha": commit_sha,
                    "model_ref": model_ref,
                    "cache_mode": cache_mode,
                    "live": str_at(report, &["mode"], "") == "live_dashboard"
                },
                "outcome": {
                    "pass": pass,
                    "excellent": bool_at(row, &["excellent"], false),
                    "score": u64_at(row, &["score"], 0),
                    "failure_classification": str_at(row, &["failure_classification"], "none"),
                    "first_failed_checkpoint": first_failed_checkpoint,
                    "failure_boundary": failure_boundary,
                    "synthesis_failure_class": synthesis_failure_class
                },
                "failure_signatures": failure_signatures,
                "gates": row.get("gates").cloned().unwrap_or_else(|| json!({})),
                "transition_summary": transition_summary(row),
                "artifact_refs": artifact_refs,
                "payload_refs": {
                    "response_hash": response_hash,
                    "response_preview": clean_text_len(response_preview.as_str(), 360)
                }
            })
        })
        .collect()
}

pub fn persist_lifecycle_observations(
    policy: &Value,
    paths: &ObservationLifecyclePaths,
    observations: &[Value],
    updated_at: &str,
) -> Result<Value, String> {
    append_jsonl(&paths.ledger_path, observations).map_err(|err| format!("ledger:{err}"))?;
    let hot_window = update_hot_window(policy, &paths.hot_window_path, observations, updated_at)
        .map_err(|err| format!("hot_window:{err}"))?;
    let previous_archive = read_json_or_empty(&paths.archive_path);
    let archive = update_archive(policy, &previous_archive, observations, updated_at);
    write_json(&paths.archive_path, &archive).map_err(|err| format!("archive:{err}"))?;
    let summary = json!({
        "type": "run_observation_lifecycle_summary",
        "schema_version": 1,
        "updated_at": updated_at,
        "policy_id": str_at(policy, &["policy_id"], "run_observation_lifecycle_v1"),
        "events_recorded": observations.len(),
        "hot_window": hot_window.get("summary").cloned().unwrap_or_else(|| json!({})),
        "archive": archive.get("summary").cloned().unwrap_or_else(|| json!({})),
        "paths": {
            "compact_ledger": paths.ledger_path,
            "hot_ring_buffer": paths.hot_window_path,
            "failure_lifecycle_archive": paths.archive_path,
            "current_summary": paths.summary_path
        },
        "data_minimization": policy.get("data_minimization").cloned().unwrap_or_else(|| json!({}))
    });
    write_json(&paths.summary_path, &summary).map_err(|err| format!("summary:{err}"))?;
    Ok(summary)
}

fn update_hot_window(
    policy: &Value,
    hot_window_path: &str,
    observations: &[Value],
    updated_at: &str,
) -> io::Result<Value> {
    let max_records = policy_u64(policy, &["retention", "hot_ring_max_records"], 500) as usize;
    let previous = read_json_or_empty(hot_window_path);
    let mut events = previous
        .get("events")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    events.extend(observations.iter().cloned());
    if events.len() > max_records {
        let start = events.len().saturating_sub(max_records);
        events = events[start..].to_vec();
    }
    let hot_window = json!({
        "type": "run_observation_hot_ring_buffer",
        "schema_version": 1,
        "updated_at": updated_at,
        "summary": {
            "max_records": max_records,
            "retained_records": events.len(),
            "dropped_oldest_records": previous
                .get("events")
                .and_then(Value::as_array)
                .map(|rows| rows.len().saturating_add(observations.len()).saturating_sub(events.len()))
                .unwrap_or(0)
        },
        "events": events
    });
    write_json(hot_window_path, &hot_window)?;
    Ok(hot_window)
}

fn update_archive(
    policy: &Value,
    previous: &Value,
    observations: &[Value],
    updated_at: &str,
) -> Value {
    let close_after = policy_u64(
        policy,
        &["retention", "archive_after_consecutive_passes"],
        2,
    );
    let history_tail_max =
        policy_u64(policy, &["retention", "subject_history_tail_max"], 12) as usize;
    let mut subjects = subjects_from_archive(previous);
    for observation in observations {
        let subject_key = str_at(observation, &["subject_key"], "unknown_subject");
        let mut subject = subjects
            .remove(subject_key.as_str())
            .unwrap_or_else(|| empty_subject(subject_key.as_str()));
        subject = update_subject(subject, observation, close_after, history_tail_max);
        subjects.insert(subject_key, subject);
    }
    let open = subjects
        .values()
        .filter(|row| {
            matches!(
                str_at(row, &["status"], "").as_str(),
                "open" | "reemerged" | "targeted"
            )
        })
        .count();
    let archived = subjects
        .values()
        .filter(|row| str_at(row, &["status"], "") == "archived")
        .count();
    let reemerged = subjects
        .values()
        .filter(|row| str_at(row, &["status"], "") == "reemerged")
        .count();
    json!({
        "type": "run_observation_lifecycle_archive",
        "schema_version": 1,
        "updated_at": updated_at,
        "summary": {
            "subjects": subjects.len(),
            "open_subjects": open,
            "archived_subjects": archived,
            "reemerged_subjects": reemerged,
            "close_after_consecutive_passes": close_after,
            "subject_history_tail_max": history_tail_max
        },
        "subjects": subjects.into_iter().map(|(_, value)| value).collect::<Vec<_>>()
    })
}

fn update_subject(
    subject: Value,
    observation: &Value,
    close_after: u64,
    history_tail_max: usize,
) -> Value {
    let pass = bool_at(observation, &["outcome", "pass"], false);
    let previous_status = str_at(&subject, &["status"], "new");
    let previous_signature = str_at(&subject, &["active_failure_signature"], "");
    let primary_signature = string_array_at(observation, &["failure_signatures"])
        .into_iter()
        .next()
        .unwrap_or_default();
    let mut consecutive_passes = u64_at(&subject, &["consecutive_passes"], 0);
    let mut consecutive_failures = u64_at(&subject, &["consecutive_failures"], 0);
    let mut reemergence_count = u64_at(&subject, &["reemergence_count"], 0);
    let mut active_failure_signature = previous_signature.clone();
    let status = if pass {
        consecutive_passes = consecutive_passes.saturating_add(1);
        consecutive_failures = 0;
        if matches!(
            previous_status.as_str(),
            "open" | "reemerged" | "targeted" | "mitigated"
        ) && consecutive_passes < close_after
        {
            "mitigated"
        } else if matches!(
            previous_status.as_str(),
            "open" | "reemerged" | "targeted" | "mitigated"
        ) && consecutive_passes >= close_after
        {
            "archived"
        } else {
            "stable_pass"
        }
    } else {
        consecutive_passes = 0;
        consecutive_failures = consecutive_failures.saturating_add(1);
        active_failure_signature = primary_signature.clone();
        if previous_status == "archived"
            && !previous_signature.is_empty()
            && previous_signature == primary_signature
        {
            reemergence_count = reemergence_count.saturating_add(1);
            "reemerged"
        } else {
            "open"
        }
    };
    let mut history_tail = subject
        .get("history_tail")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    history_tail.push(history_entry(observation, status));
    if history_tail.len() > history_tail_max {
        let start = history_tail.len().saturating_sub(history_tail_max);
        history_tail = history_tail[start..].to_vec();
    }
    let seen_count = u64_at(&subject, &["seen_count"], 0).saturating_add(1);
    let pass_count = u64_at(&subject, &["pass_count"], 0).saturating_add(if pass { 1 } else { 0 });
    let failure_count =
        u64_at(&subject, &["failure_count"], 0).saturating_add(if pass { 0 } else { 1 });
    let subject_key = str_at(&subject, &["subject_key"], "unknown_subject");
    json!({
        "subject_key": subject_key,
        "status": status,
        "active_failure_signature": active_failure_signature,
        "last_observed_at": str_at(observation, &["observed_at"], ""),
        "last_event_id": str_at(observation, &["event_id"], ""),
        "last_score": u64_at(observation, &["outcome", "score"], 0),
        "last_failure_classification": str_at(observation, &["outcome", "failure_classification"], "none"),
        "consecutive_passes": consecutive_passes,
        "consecutive_failures": consecutive_failures,
        "seen_count": seen_count,
        "pass_count": pass_count,
        "failure_count": failure_count,
        "reemergence_count": reemergence_count,
        "history_tail": history_tail
    })
}

fn history_entry(observation: &Value, status: &str) -> Value {
    json!({
        "observed_at": str_at(observation, &["observed_at"], ""),
        "event_id": str_at(observation, &["event_id"], ""),
        "status": status,
        "run_id": str_at(observation, &["run", "run_id"], ""),
        "commit_sha": str_at(observation, &["run", "commit_sha"], "unknown"),
        "pass": bool_at(observation, &["outcome", "pass"], false),
        "score": u64_at(observation, &["outcome", "score"], 0),
        "failure_classification": str_at(observation, &["outcome", "failure_classification"], "none"),
        "failure_signatures": observation.get("failure_signatures").cloned().unwrap_or_else(|| json!([]))
    })
}

fn subjects_from_archive(previous: &Value) -> BTreeMap<String, Value> {
    let rows = previous
        .get("subjects")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    rows.into_iter()
        .filter_map(|row| {
            let key = str_at(&row, &["subject_key"], "");
            if key.is_empty() {
                None
            } else {
                Some((key, row))
            }
        })
        .collect()
}

fn empty_subject(subject_key: &str) -> Value {
    json!({
        "subject_key": clean_text(subject_key),
        "status": "new",
        "active_failure_signature": "",
        "consecutive_passes": 0,
        "consecutive_failures": 0,
        "seen_count": 0,
        "pass_count": 0,
        "failure_count": 0,
        "reemergence_count": 0,
        "history_tail": []
    })
}

fn failure_signatures_for_row(row: &Value) -> Vec<String> {
    if bool_at(row, &["pass"], false) {
        return Vec::new();
    }
    let classification = str_at(row, &["failure_classification"], "soft");
    let boundary = str_at(
        row,
        &["gate_transition_diagnostics", "inferred_failure_boundary"],
        "unknown_boundary",
    );
    let mut signatures = string_array_at(row, &["failures"])
        .into_iter()
        .map(|failure| {
            format!(
                "{}:{}:{}",
                normalize_signature_part(classification.as_str()),
                normalize_signature_part(boundary.as_str()),
                normalize_signature_part(failure.as_str())
            )
        })
        .collect::<Vec<_>>();
    let checkpoint = str_at(
        row,
        &["gate_transition_diagnostics", "first_failed_checkpoint"],
        "",
    );
    if !checkpoint.is_empty() {
        signatures.push(format!(
            "{}:{}:checkpoint_{}",
            normalize_signature_part(classification.as_str()),
            normalize_signature_part(boundary.as_str()),
            normalize_signature_part(checkpoint.as_str())
        ));
    }
    if signatures.is_empty() {
        signatures.push(format!(
            "{}:{}:unspecified_failure",
            normalize_signature_part(classification.as_str()),
            normalize_signature_part(boundary.as_str())
        ));
    }
    signatures.sort();
    signatures.dedup();
    signatures
}

fn transition_summary(row: &Value) -> Value {
    let checkpoints = row
        .get("gate_transition_diagnostics")
        .and_then(|value| value.get("checkpoints"))
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|checkpoint| {
            let name = str_at(&checkpoint, &["checkpoint"], "");
            if name.is_empty() {
                return None;
            }
            Some(json!({
                "checkpoint": name,
                "status": str_at(&checkpoint, &["status"], "unknown")
            }))
        })
        .collect::<Vec<_>>();
    json!({
        "first_failed_checkpoint": str_at(row, &["gate_transition_diagnostics", "first_failed_checkpoint"], ""),
        "failure_boundary": str_at(row, &["gate_transition_diagnostics", "inferred_failure_boundary"], ""),
        "checkpoints": checkpoints
    })
}

fn normalize_signature_part(raw: &str) -> String {
    let mut out = String::new();
    let mut last_was_sep = false;
    for ch in raw.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_was_sep = false;
        } else if !last_was_sep {
            out.push('_');
            last_was_sep = true;
        }
        if out.len() >= 120 {
            break;
        }
    }
    out.trim_matches('_').to_string()
}

fn append_jsonl(path: &str, rows: &[Value]) -> io::Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)?;
    for row in rows {
        writeln!(file, "{}", serde_json::to_string(row)?)?;
    }
    Ok(())
}

fn write_json(path: &str, value: &Value) -> io::Result<()> {
    if let Some(parent) = Path::new(path).parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, format!("{}\n", serde_json::to_string_pretty(value)?))
}

fn read_json_or_empty(path: &str) -> Value {
    fs::read_to_string(path)
        .ok()
        .and_then(|raw| serde_json::from_str::<Value>(&raw).ok())
        .unwrap_or_else(|| json!({}))
}

fn clean_text(raw: &str) -> String {
    clean_text_len(raw, 240)
}

fn clean_text_len(raw: &str, limit: usize) -> String {
    raw.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(limit)
        .collect()
}

fn str_at(value: &Value, path: &[&str], default: &str) -> String {
    let mut cursor = value;
    for segment in path {
        let Some(next) = cursor.get(*segment) else {
            return default.to_string();
        };
        cursor = next;
    }
    cursor
        .as_str()
        .map(clean_text)
        .filter(|raw| !raw.is_empty())
        .unwrap_or_else(|| default.to_string())
}

fn bool_at(value: &Value, path: &[&str], default: bool) -> bool {
    let mut cursor = value;
    for segment in path {
        let Some(next) = cursor.get(*segment) else {
            return default;
        };
        cursor = next;
    }
    cursor.as_bool().unwrap_or(default)
}

fn u64_at(value: &Value, path: &[&str], default: u64) -> u64 {
    let mut cursor = value;
    for segment in path {
        let Some(next) = cursor.get(*segment) else {
            return default;
        };
        cursor = next;
    }
    cursor.as_u64().unwrap_or(default)
}

fn string_array_at(value: &Value, path: &[&str]) -> Vec<String> {
    let mut cursor = value;
    for segment in path {
        let Some(next) = cursor.get(*segment) else {
            return Vec::new();
        };
        cursor = next;
    }
    cursor
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(clean_text)
        .filter(|raw| !raw.is_empty())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archive_tracks_archived_and_reemerged_failure() {
        let policy = default_policy();
        let failed = json!({
            "event_id": "event-1",
            "observed_at": "unix_ms:1",
            "subject_key": "workflow:test/case:a",
            "run": {"run_id": "r1", "commit_sha": "abc"},
            "outcome": {"pass": false, "score": 70, "failure_classification": "soft"},
            "failure_signatures": ["soft:synthesis:entity_coverage_low"]
        });
        let pass_one = json!({
            "event_id": "event-2",
            "observed_at": "unix_ms:2",
            "subject_key": "workflow:test/case:a",
            "run": {"run_id": "r2", "commit_sha": "def"},
            "outcome": {"pass": true, "score": 96, "failure_classification": "none"},
            "failure_signatures": []
        });
        let pass_two = json!({
            "event_id": "event-3",
            "observed_at": "unix_ms:3",
            "subject_key": "workflow:test/case:a",
            "run": {"run_id": "r3", "commit_sha": "ghi"},
            "outcome": {"pass": true, "score": 97, "failure_classification": "none"},
            "failure_signatures": []
        });
        let archive = update_archive(&policy, &json!({}), &[failed.clone()], "unix_ms:1");
        assert_eq!(archive.pointer("/subjects/0/status"), Some(&json!("open")));
        let archive = update_archive(&policy, &archive, &[pass_one], "unix_ms:2");
        assert_eq!(
            archive.pointer("/subjects/0/status"),
            Some(&json!("mitigated"))
        );
        let archive = update_archive(&policy, &archive, &[pass_two], "unix_ms:3");
        assert_eq!(
            archive.pointer("/subjects/0/status"),
            Some(&json!("archived"))
        );
        let archive = update_archive(&policy, &archive, &[failed], "unix_ms:4");
        assert_eq!(
            archive.pointer("/subjects/0/status"),
            Some(&json!("reemerged"))
        );
        assert_eq!(
            archive.pointer("/subjects/0/reemergence_count"),
            Some(&json!(1))
        );
    }

    #[test]
    fn hot_window_keeps_recent_records_only() {
        let temp = std::env::temp_dir().join(format!(
            "observation_hot_window_{}.json",
            stable_hash_hex("hot-window-test")
        ));
        let policy = json!({"retention": {"hot_ring_max_records": 2}});
        let rows = vec![
            json!({"event_id": "a"}),
            json!({"event_id": "b"}),
            json!({"event_id": "c"}),
        ];
        let hot = update_hot_window(&policy, temp.to_str().unwrap(), &rows, "unix_ms:1")
            .expect("hot window");
        let retained = hot.get("events").and_then(Value::as_array).unwrap();
        assert_eq!(retained.len(), 2);
        assert_eq!(retained[0].get("event_id"), Some(&json!("b")));
        assert_eq!(retained[1].get("event_id"), Some(&json!("c")));
        let _ = fs::remove_file(temp);
    }
}
