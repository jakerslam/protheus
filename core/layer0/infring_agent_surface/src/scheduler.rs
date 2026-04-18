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
        }
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

    pub fn snapshot(&self) -> Value {
        json!({
            "entries": self.entries.values().collect::<Vec<_>>(),
            "active": self.entries.values().filter(|entry| !entry.paused).count(),
            "paused": self.entries.values().filter(|entry| entry.paused).count(),
        })
    }
}

