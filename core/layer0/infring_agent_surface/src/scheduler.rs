use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::BTreeMap;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SchedulePlan {
    pub interval_seconds: u64,
    pub jitter_seconds: u64,
    pub max_runs: Option<u32>,
}

impl Default for SchedulePlan {
    fn default() -> Self {
        Self {
            interval_seconds: 300,
            jitter_seconds: 0,
            max_runs: None,
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ScheduleEntry {
    pub entry_id: String,
    pub agent_name: String,
    pub capability_pack: String,
    pub plan: SchedulePlan,
    pub next_due_unix: u64,
    pub last_run_unix: Option<u64>,
    pub run_count: u32,
    pub paused: bool,
    pub pause_reason: Option<String>,
    pub last_status: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct Scheduler {
    entries: BTreeMap<String, ScheduleEntry>,
}

impl Scheduler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn upsert(&mut self, entry: ScheduleEntry) {
        self.entries.insert(entry.entry_id.clone(), entry);
    }

    pub fn pause(&mut self, entry_id: &str, paused: bool) {
        if let Some(entry) = self.entries.get_mut(entry_id) {
            entry.paused = paused;
            if !paused {
                entry.pause_reason = None;
            }
        }
    }

    pub fn pause_with_reason(&mut self, entry_id: &str, reason: impl Into<String>) {
        if let Some(entry) = self.entries.get_mut(entry_id) {
            entry.paused = true;
            let normalized = reason.into().trim().to_string();
            if !normalized.is_empty() {
                entry.pause_reason = Some(normalized);
            }
        }
    }

    pub fn resume(&mut self, entry_id: &str) {
        self.pause(entry_id, false);
    }

    pub fn due_entries(&self, now_unix: u64) -> Vec<ScheduleEntry> {
        self.entries
            .values()
            .filter(|entry| !entry.paused)
            .filter(|entry| now_unix >= entry.next_due_unix)
            .filter(|entry| {
                entry
                    .plan
                    .max_runs
                    .map(|max| entry.run_count < max)
                    .unwrap_or(true)
            })
            .cloned()
            .collect()
    }

    pub fn mark_executed(&mut self, entry_id: &str, now_unix: u64) {
        if let Some(entry) = self.entries.get_mut(entry_id) {
            entry.last_run_unix = Some(now_unix);
            entry.run_count = entry.run_count.saturating_add(1);
            entry.last_status = Some("ok".to_string());
            let jitter = if entry.plan.jitter_seconds == 0 {
                0
            } else {
                (entry.run_count as u64) % entry.plan.jitter_seconds
            };
            entry.next_due_unix = now_unix
                .saturating_add(entry.plan.interval_seconds)
                .saturating_add(jitter);
        }
    }

    pub fn mark_failed(&mut self, entry_id: &str, now_unix: u64, error_code: &str) {
        if let Some(entry) = self.entries.get_mut(entry_id) {
            entry.last_run_unix = Some(now_unix);
            entry.last_status = Some(format!("error:{error_code}"));
            let retry_delay = entry.plan.interval_seconds.min(60);
            entry.next_due_unix = now_unix.saturating_add(retry_delay);
        }
    }

    pub fn snapshot(&self) -> Value {
        json!({
            "entries": self.entries.values().collect::<Vec<_>>(),
            "active": self.entries.values().filter(|entry| !entry.paused).count(),
            "paused": self.entries.values().filter(|entry| entry.paused).count(),
            "bounded": self.entries.values().all(|entry| entry.plan.max_runs.is_some() || entry.plan.interval_seconds >= 60),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_entry() -> ScheduleEntry {
        ScheduleEntry {
            entry_id: "entry-1".to_string(),
            agent_name: "agent".to_string(),
            capability_pack: "issue-ops".to_string(),
            plan: SchedulePlan {
                interval_seconds: 120,
                jitter_seconds: 15,
                max_runs: Some(3),
            },
            next_due_unix: 10,
            last_run_unix: None,
            run_count: 0,
            paused: false,
            pause_reason: None,
            last_status: None,
        }
    }

    #[test]
    fn scheduler_pause_with_reason_and_resume() {
        let mut scheduler = Scheduler::new();
        scheduler.upsert(sample_entry());
        scheduler.pause_with_reason("entry-1", "manual_hold");
        let snapshot = scheduler.snapshot();
        let entries = snapshot
            .get("entries")
            .and_then(Value::as_array)
            .expect("entries");
        let reason = entries[0]
            .get("pause_reason")
            .and_then(Value::as_str)
            .expect("pause reason");
        assert_eq!(reason, "manual_hold");
        scheduler.resume("entry-1");
        let due = scheduler.due_entries(15);
        assert_eq!(due.len(), 1);
    }

    #[test]
    fn scheduler_mark_failed_uses_short_retry_backoff() {
        let mut scheduler = Scheduler::new();
        scheduler.upsert(sample_entry());
        scheduler.mark_failed("entry-1", 100, "provider_timeout");
        let due = scheduler.due_entries(159);
        assert!(due.is_empty());
        let due = scheduler.due_entries(160);
        assert_eq!(due.len(), 1);
    }
}
