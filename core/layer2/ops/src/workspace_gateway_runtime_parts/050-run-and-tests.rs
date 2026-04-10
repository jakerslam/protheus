fn print_task_usage() {
    for line in TASK_USAGE {
        println!("{line}");
    }
}

fn strip_head(parsed: &ParsedCli) -> ParsedCli {
    ParsedCli {
        positional: parsed.positional.iter().skip(1).cloned().collect(),
        flags: parsed.flags.clone(),
    }
}

fn task_command_error(command: &str) -> i32 {
    eprintln!(
        "{}",
        json!({
            "ok": false,
            "type": "task_command_error",
            "error": "unknown_task_command",
            "command": command
        })
    );
    1
}

fn run_task_command(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_cli(argv);
    let Some(command_raw) = parsed.positional.first().cloned() else {
        print_task_usage();
        return 0;
    };
    let command = command_raw.trim().to_ascii_lowercase();
    let tail = strip_head(&parsed);
    match command.as_str() {
        "submit" | "enqueue" => submit_task(root, &tail),
        "status" => status_task(root, &tail),
        "list" => list_tasks(root, &tail),
        "cancel" => cancel_task(root, &tail),
        "worker" | "work" => run_worker(root, &tail),
        "slow-test" | "test-flow" => run_slow_test(root, &tail),
        "help" | "--help" | "-h" => {
            print_task_usage();
            0
        }
        _ => task_command_error(&command),
    }
}

pub fn run(root: &Path, argv: &[String]) -> i32 {
    if let Some(first) = argv.first() {
        if first.trim().eq_ignore_ascii_case("task") {
            let task_args = argv.iter().skip(1).cloned().collect::<Vec<_>>();
            return run_task_command(root, &task_args);
        }
    }
    run_legacy_lane(root, argv)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};
    use tempfile::tempdir;

    fn state_root_for(temp_root: &Path) -> String {
        temp_root.join("state").display().to_string()
    }

    fn set_test_state_env(path: &str) {
        std::env::set_var(TASK_STATE_ROOT_ENV, path);
        std::env::set_var(TASK_BUS_ENV, "local");
    }

    fn clear_test_state_env() {
        std::env::remove_var(TASK_STATE_ROOT_ENV);
        std::env::remove_var(TASK_BUS_ENV);
    }

    fn test_guard() -> std::sync::MutexGuard<'static, ()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
            .lock()
            .expect("test lock")
    }

    fn list_registry_ids(path: &Path) -> Vec<String> {
        let raw = fs::read_to_string(path).expect("registry readable");
        let parsed: TaskRegistry = serde_json::from_str(&raw).expect("registry parse");
        parsed.tasks.into_iter().map(|row| row.id).collect()
    }

    fn read_jsonl(path: &Path) -> Vec<Value> {
        fs::read_to_string(path)
            .expect("jsonl readable")
            .lines()
            .filter_map(|line| serde_json::from_str::<Value>(line.trim()).ok())
            .collect::<Vec<_>>()
    }

    #[test]
    fn task_submit_and_worker_complete_generates_verity_receipt() {
        let _guard = test_guard();
        let temp = tempdir().expect("tempdir");
        let state_root = state_root_for(temp.path());
        set_test_state_env(&state_root);

        let root = temp.path();
        let submit = vec![
            "task".to_string(),
            "submit".to_string(),
            "--estimated-seconds=1".to_string(),
            "--steps=1".to_string(),
            "--kind=test".to_string(),
        ];
        assert_eq!(run(root, &submit), 0);

        let worker = vec![
            "task".to_string(),
            "worker".to_string(),
            "--max-tasks=1".to_string(),
            "--wait-ms=10".to_string(),
        ];
        assert_eq!(run(root, &worker), 0);

        let paths = task_paths(root);
        let ids = list_registry_ids(&paths.registry_json);
        assert_eq!(ids.len(), 1);
        let receipts = read_jsonl(&paths.receipts_jsonl);
        assert!(receipts.iter().any(|row| {
            row.get("type").and_then(Value::as_str) == Some("task_verity_receipt")
                && row
                    .get("timestamp_drift_ms")
                    .and_then(Value::as_u64)
                    .is_some()
                && row
                    .get("parent_receipt_hash")
                    .and_then(Value::as_str)
                    .is_some()
        }));
        clear_test_state_env();
    }

    #[test]
    fn task_cancel_marks_cancelled_state() {
        let _guard = test_guard();
        let temp = tempdir().expect("tempdir");
        let state_root = state_root_for(temp.path());
        set_test_state_env(&state_root);

        let root = temp.path();
        let submit = vec![
            "task".to_string(),
            "submit".to_string(),
            "--estimated-seconds=2".to_string(),
            "--steps=2".to_string(),
            "--kind=cancel-test".to_string(),
        ];
        assert_eq!(run(root, &submit), 0);
        let paths = task_paths(root);
        let raw = fs::read_to_string(&paths.registry_json).expect("registry");
        let parsed: TaskRegistry = serde_json::from_str(&raw).expect("registry parse");
        let id = parsed.tasks.first().expect("task exists").id.clone();
        let cancel = vec![
            "task".to_string(),
            "cancel".to_string(),
            format!("--ticket={id}"),
        ];
        assert_eq!(run(root, &cancel), 0);
        let raw_after = fs::read_to_string(&paths.registry_json).expect("registry after");
        let parsed_after: TaskRegistry = serde_json::from_str(&raw_after).expect("registry parse");
        let record = parsed_after.tasks.first().expect("record exists");
        assert!(record.cancelled);
        assert_eq!(record.status, "cancelled");
        clear_test_state_env();
    }

    #[test]
    fn worker_applies_cancel_sync_from_bus_before_processing() {
        struct CancelOnlyBus {
            ids: Vec<String>,
        }
        impl TaskBus for CancelOnlyBus {
            fn mode(&self) -> &'static str {
                "test_bus"
            }
            fn enqueue(&self, _payload: &TaskPayload) -> Result<(), String> {
                Ok(())
            }
            fn dequeue(
                &self,
                _max_messages: usize,
                _wait_ms: u64,
            ) -> Result<Vec<TaskPayload>, String> {
                Ok(Vec::new())
            }
            fn publish_cancel(&self, _task_id: &str) -> Result<(), String> {
                Ok(())
            }
            fn pull_cancelled(
                &self,
                _max_messages: usize,
                _wait_ms: u64,
            ) -> Result<Vec<String>, String> {
                Ok(self.ids.clone())
            }
        }

        let _guard = test_guard();
        let temp = tempdir().expect("tempdir");
        let state_root = state_root_for(temp.path());
        set_test_state_env(&state_root);
        let root = temp.path();
        let submit = vec![
            "task".to_string(),
            "submit".to_string(),
            "--estimated-seconds=2".to_string(),
            "--steps=2".to_string(),
            "--kind=sync-cancel-test".to_string(),
        ];
        assert_eq!(run(root, &submit), 0);
        let paths = task_paths(root);
        let raw = fs::read_to_string(&paths.registry_json).expect("registry");
        let parsed: TaskRegistry = serde_json::from_str(&raw).expect("registry parse");
        let id = parsed.tasks.first().expect("task exists").id.clone();
        let bus = CancelOnlyBus {
            ids: vec![id.clone()],
        };
        let synced = apply_bus_cancellations(&paths, &bus, 10).expect("cancel sync");
        assert_eq!(synced, 1);
        let registry_after = load_registry(&paths).expect("registry after");
        let record = registry_after
            .tasks
            .into_iter()
            .find(|row| row.id == id)
            .expect("record exists");
        assert!(record.cancelled);
        assert_eq!(record.status, "cancelled");
        clear_test_state_env();
    }
}
