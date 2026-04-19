const DASHBOARD_TROUBLESHOOTING_RECENT_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/troubleshooting/recent_workflows.json";
const DASHBOARD_TROUBLESHOOTING_SNAPSHOT_LATEST_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/troubleshooting/latest_snapshot.json";
const DASHBOARD_TROUBLESHOOTING_SNAPSHOT_HISTORY_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/troubleshooting/snapshot_history.jsonl";
const DASHBOARD_TROUBLESHOOTING_EVAL_QUEUE_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/troubleshooting/eval_queue.json";
const DASHBOARD_TROUBLESHOOTING_EVAL_LATEST_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/troubleshooting/latest_eval_report.json";
const DASHBOARD_TROUBLESHOOTING_EVAL_HISTORY_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/troubleshooting/eval_reports.jsonl";
const DASHBOARD_TROUBLESHOOTING_ISSUE_OUTBOX_REL: &str =
    "client/runtime/local/state/ui/infring_dashboard/troubleshooting/issue_outbox.json";
const DASHBOARD_TROUBLESHOOTING_MAX_RECENT: usize = 10;
const DASHBOARD_TROUBLESHOOTING_MAX_QUEUE: usize = 500;
const DASHBOARD_TROUBLESHOOTING_MAX_OUTBOX: usize = 300;
const DASHBOARD_TROUBLESHOOTING_DEFAULT_EVAL_MODEL: &str = "gpt-5.4";

include!("065-troubleshooting-and-eval_parts/001-segment.rs");
include!("065-troubleshooting-and-eval_parts/002-segment.rs");
include!("065-troubleshooting-and-eval_parts/003-segment.rs");
include!("065-troubleshooting-and-eval_parts/004-segment.rs");
include!("065-troubleshooting-and-eval_parts/005-segment.rs");
