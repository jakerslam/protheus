
#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::{Mutex, OnceLock};

    fn test_env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    fn supports_rg() -> bool {
        Command::new("rg").arg("--version").output().is_ok()
    }

    fn reset_dir(path: &Path) {
        let _ = fs::remove_dir_all(path);
        fs::create_dir_all(path).expect("root dir");
    }

    fn temp_case_root(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "workspace_file_search_{}_{}_{}",
            label,
            std::process::id(),
            chrono::Utc::now().timestamp_nanos_opt().unwrap_or_default()
        ))
    }

    fn search_args_for(root: &Path, query: &str) -> crate::ParsedArgs {
        crate::parse_args(&[
            format!("--workspace={}", root.display()),
            format!("--q={query}"),
            "--limit=5".to_string(),
        ])
    }

    #[test]
    fn fuzzy_score_prefers_tighter_match() {
        let tight = subsequence_gap_score("abc", "abc file").expect("tight");
        let loose = subsequence_gap_score("abc", "a xx b yy c").expect("loose");
        assert!(tight.0 < loose.0);
    }

    #[test]
    fn workspace_outside_root_is_blocked_by_default() {
        let root =
            std::env::temp_dir().join(format!("workspace_file_search_root_{}", std::process::id()));
        let inside = root.join("inside");
        let outside = std::env::temp_dir().join(format!(
            "workspace_file_search_outside_{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&outside);
        fs::create_dir_all(&inside).expect("inside dir");
        fs::create_dir_all(&outside).expect("outside dir");
        let args = crate::parse_args(&[format!("--workspace={}", outside.display())]);
        let result = resolve_workspace_specs(&inside, &args);
        assert!(result.is_err());
        let _ = fs::remove_dir_all(&root);
        let _ = fs::remove_dir_all(&outside);
    }

    #[test]
    fn run_search_returns_match_for_workspace_file() {
        let _guard = test_env_lock()
            .lock()
            .expect("workspace_file_search test lock");
        if !supports_rg() {
            return;
        }
        let root = temp_case_root("run");
        reset_dir(&root);
        fs::write(root.join("context-stacks-proof.txt"), "proof").expect("fixture");
        let payload = run_search(&root, &search_args_for(&root, "context"), "");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        let results = payload
            .get("results")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(!results.is_empty());
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn run_search_reports_ripgrep_install_hint_when_missing() {
        let _guard = test_env_lock()
            .lock()
            .expect("workspace_file_search test lock");
        let root = temp_case_root("missing-rg");
        reset_dir(&root);
        fs::write(root.join("file.txt"), "fixture").expect("fixture");
        let previous_rg = std::env::var("INFRING_RG_BINARY").ok();
        std::env::set_var(
            "INFRING_RG_BINARY",
            "__missing_rg_binary_for_workspace_file_search__",
        );
        let payload = run_search(&root, &search_args_for(&root, "file"), "");
        let warnings = payload
            .get("warnings")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        assert!(
            warnings
                .iter()
                .any(|row| row.as_str().unwrap_or("").contains("rg_not_found")),
            "expected rg_not_found warning with install hint"
        );
        if let Some(prev) = previous_rg {
            std::env::set_var("INFRING_RG_BINARY", prev);
        } else {
            std::env::remove_var("INFRING_RG_BINARY");
        }
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn run_mention_returns_insertable_path() {
        let _guard = test_env_lock()
            .lock()
            .expect("workspace_file_search test lock");
        if !supports_rg() {
            return;
        }
        let root = temp_case_root("mention");
        reset_dir(&root.join("src"));
        fs::write(root.join("src").join("main.rs"), "fn main() {}").expect("fixture");
        let payload = run_mention(&root, &search_args_for(&root, "main"), "");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(payload.get("status").and_then(Value::as_str), Some("ok"));
        let mention = payload
            .get("mention")
            .and_then(Value::as_str)
            .unwrap_or_default()
            .to_string();
        assert!(mention.starts_with('@'));
        assert!(mention.contains("main.rs"));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn run_mention_reports_no_results_state() {
        let _guard = test_env_lock()
            .lock()
            .expect("workspace_file_search test lock");
        if !supports_rg() {
            return;
        }
        let root = temp_case_root("mention-no-results");
        reset_dir(&root);
        fs::write(root.join("alpha.txt"), "fixture").expect("fixture");
        let payload = run_mention(&root, &search_args_for(&root, "zzzzzz"), "");
        assert_eq!(payload.get("ok").and_then(Value::as_bool), Some(true));
        assert_eq!(
            payload.get("status").and_then(Value::as_str),
            Some("no_results")
        );
        assert!(payload.get("mention").map(Value::is_null).unwrap_or(false));
        let _ = fs::remove_dir_all(&root);
    }
}
