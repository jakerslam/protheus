fn print_task_usage() {
    for line in TASK_USAGE {
        println!("{line}");
    }
}

fn run_task_command(root: &Path, argv: &[String]) -> i32 {
    let parsed = parse_cli(argv);
    let Some(command_raw) = parsed.positional.first().cloned() else {
        print_task_usage();
        return 0;
    };
    let command = command_raw.trim().to_ascii_lowercase();
    let tail = ParsedCli {
        positional: parsed.positional.iter().skip(1).cloned().collect(),
        flags: parsed.flags.clone(),
    };
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
        _ => {
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
        let receipts = fs::read_to_string(paths.receipts_jsonl).expect("receipts readable");
        assert!(receipts.contains("\"type\":\"task_verity_receipt\""));
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
}
