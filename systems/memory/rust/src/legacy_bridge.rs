use std::env;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Clone, Copy, Debug)]
pub struct LegacyBridgeSpec {
    pub lane_id: &'static str,
    pub legacy_script_rel: &'static str,
}

pub fn detect_repo_root(explicit_root: Option<&str>, cwd: &Path) -> PathBuf {
    if let Some(raw) = explicit_root {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            let candidate = PathBuf::from(trimmed);
            if candidate.is_absolute() {
                return candidate;
            }
            return cwd.join(candidate);
        }
    }

    for candidate in cwd.ancestors() {
        if looks_like_repo_root(candidate) {
            return candidate.to_path_buf();
        }
    }

    cwd.to_path_buf()
}

fn looks_like_repo_root(candidate: &Path) -> bool {
    candidate.join("AGENT-CONSTITUTION.md").is_file()
        && candidate.join("systems").join("memory").is_dir()
}

pub fn resolve_legacy_script(root: &Path, relative_script: &str) -> PathBuf {
    root.join(relative_script)
}

fn resolve_node_bin() -> String {
    let explicit = env::var("PROTHEUS_NODE_BIN").unwrap_or_default();
    let trimmed = explicit.trim();
    if trimmed.is_empty() {
        "node".to_string()
    } else {
        trimmed.to_string()
    }
}

pub fn run_legacy_bridge(spec: LegacyBridgeSpec, args: &[String]) -> i32 {
    let cwd = env::current_dir().unwrap_or_else(|_| PathBuf::from("."));
    let explicit_root = env::var("PROTHEUS_ROOT").ok();
    let repo_root = detect_repo_root(explicit_root.as_deref(), &cwd);
    let legacy_script = resolve_legacy_script(&repo_root, spec.legacy_script_rel);

    if !legacy_script.is_file() {
        eprintln!(
            "{}: legacy script missing at {}",
            spec.lane_id,
            legacy_script.to_string_lossy()
        );
        return 1;
    }

    let command = resolve_node_bin();
    let output = Command::new(&command)
        .arg(&legacy_script)
        .args(args)
        .current_dir(&repo_root)
        .env("PROTHEUS_RUST_LEGACY_BRIDGE", "1")
        .env("PROTHEUS_RUST_BRIDGE_LANE", spec.lane_id)
        .output();

    let Ok(out) = output else {
        eprintln!(
            "{}: failed to spawn legacy bridge command ({})",
            spec.lane_id, command
        );
        return 1;
    };

    if io::stdout().write_all(&out.stdout).is_err() {
        return 1;
    }
    if io::stderr().write_all(&out.stderr).is_err() {
        return 1;
    }

    out.status.code().unwrap_or(1)
}

#[cfg(test)]
mod tests {
    use super::{detect_repo_root, resolve_legacy_script, run_legacy_bridge, LegacyBridgeSpec};
    use std::env;
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::{Mutex, OnceLock};
    use std::time::{SystemTime, UNIX_EPOCH};

    fn env_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }

    struct EnvRestore {
        vars: Vec<(String, Option<String>)>,
    }

    impl EnvRestore {
        fn set(pairs: &[(&str, Option<&str>)]) -> Self {
            let mut vars = Vec::with_capacity(pairs.len());
            for (key, value) in pairs {
                let previous = env::var(key).ok();
                match value {
                    Some(v) => env::set_var(key, v),
                    None => env::remove_var(key),
                }
                vars.push(((*key).to_string(), previous));
            }
            Self { vars }
        }
    }

    impl Drop for EnvRestore {
        fn drop(&mut self) {
            for (key, previous) in self.vars.drain(..).rev() {
                if let Some(value) = previous {
                    env::set_var(key, value);
                } else {
                    env::remove_var(key);
                }
            }
        }
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map(|dur| dur.as_nanos())
            .unwrap_or(0);
        let dir = std::env::temp_dir().join(format!("{prefix}-{now}"));
        let _ = fs::create_dir_all(&dir);
        dir
    }

    fn setup_repo(root: &Path) {
        let _ = fs::create_dir_all(root.join("systems").join("memory"));
        let _ = fs::write(root.join("AGENT-CONSTITUTION.md"), "# test\n");
    }

    #[cfg(unix)]
    fn make_executable(path: &Path) {
        use std::os::unix::fs::PermissionsExt;

        let mut perms = fs::metadata(path).expect("metadata").permissions();
        perms.set_mode(0o755);
        fs::set_permissions(path, perms).expect("set permissions");
    }

    #[test]
    fn detect_repo_root_prefers_explicit_path() {
        let cwd = unique_temp_dir("legacy-bridge-explicit-cwd");
        let explicit = unique_temp_dir("legacy-bridge-explicit-root");
        let root = detect_repo_root(Some(&explicit.to_string_lossy()), &cwd);
        assert_eq!(root, explicit);
    }

    #[test]
    fn detect_repo_root_supports_relative_explicit_path() {
        let cwd = unique_temp_dir("legacy-bridge-relative-cwd");
        let root = detect_repo_root(Some("repo-root"), &cwd);
        assert_eq!(root, cwd.join("repo-root"));
    }

    #[test]
    fn detect_repo_root_finds_ancestor_with_markers() {
        let repo = unique_temp_dir("legacy-bridge-ancestor-root");
        setup_repo(&repo);
        let nested = repo
            .join("systems")
            .join("memory")
            .join("nested")
            .join("child");
        let _ = fs::create_dir_all(&nested);

        let root = detect_repo_root(None, &nested);
        assert_eq!(root, repo);
    }

    #[test]
    fn detect_repo_root_falls_back_to_cwd_when_markers_missing() {
        let cwd = unique_temp_dir("legacy-bridge-fallback-cwd");
        let root = detect_repo_root(None, &cwd);
        assert_eq!(root, cwd);
    }

    #[test]
    fn resolve_legacy_script_joins_relative_path() {
        let root = PathBuf::from("/tmp/protheus-test");
        let script = resolve_legacy_script(&root, "systems/memory/lane_legacy.js");
        assert_eq!(
            script,
            PathBuf::from("/tmp/protheus-test/systems/memory/lane_legacy.js")
        );
    }

    #[test]
    fn run_legacy_bridge_returns_error_when_script_missing() {
        let _env_guard = env_lock().lock().expect("env lock");
        let repo = unique_temp_dir("legacy-bridge-missing-script");
        setup_repo(&repo);
        let _restore = EnvRestore::set(&[
            ("PROTHEUS_ROOT", Some(repo.to_string_lossy().as_ref())),
            ("PROTHEUS_NODE_BIN", Some("node")),
        ]);

        let code = run_legacy_bridge(
            LegacyBridgeSpec {
                lane_id: "test_lane",
                legacy_script_rel: "systems/memory/missing_legacy.js",
            },
            &[],
        );

        assert_eq!(code, 1);
    }

    #[cfg(unix)]
    #[test]
    fn run_legacy_bridge_propagates_args_env_and_exit_code() {
        let _env_guard = env_lock().lock().expect("env lock");
        let repo = unique_temp_dir("legacy-bridge-runner");
        setup_repo(&repo);
        let legacy_rel = "systems/memory/test_lane_legacy.js";
        let legacy_abs = repo.join(legacy_rel);
        fs::write(&legacy_abs, "console.log('ok')\n").expect("write legacy script");

        let args_capture = repo.join("args.txt");
        let env_capture = repo.join("env.txt");
        let fake_node = repo.join("fake_node.sh");
        let fake_node_src = r#"#!/bin/sh
printf '%s\n' "$@" > "$LEGACY_BRIDGE_ARGS_PATH"
printf '%s:%s\n' "$PROTHEUS_RUST_LEGACY_BRIDGE" "$PROTHEUS_RUST_BRIDGE_LANE" > "$LEGACY_BRIDGE_ENV_PATH"
exit "${LEGACY_BRIDGE_EXIT_CODE:-0}"
"#;
        fs::write(&fake_node, fake_node_src).expect("write fake node");
        make_executable(&fake_node);

        let _restore = EnvRestore::set(&[
            ("PROTHEUS_ROOT", Some(repo.to_string_lossy().as_ref())),
            (
                "PROTHEUS_NODE_BIN",
                Some(fake_node.to_string_lossy().as_ref()),
            ),
            (
                "LEGACY_BRIDGE_ARGS_PATH",
                Some(args_capture.to_string_lossy().as_ref()),
            ),
            (
                "LEGACY_BRIDGE_ENV_PATH",
                Some(env_capture.to_string_lossy().as_ref()),
            ),
            ("LEGACY_BRIDGE_EXIT_CODE", Some("17")),
        ]);

        let args = vec!["pilot".to_string(), "--backend=rust".to_string()];
        let code = run_legacy_bridge(
            LegacyBridgeSpec {
                lane_id: "rust_memory_transition_lane",
                legacy_script_rel: legacy_rel,
            },
            &args,
        );
        assert_eq!(code, 17);

        let args_raw = fs::read_to_string(&args_capture).expect("read args capture");
        let lines = args_raw
            .lines()
            .map(|line| line.to_string())
            .collect::<Vec<_>>();
        assert_eq!(lines.len(), 3);
        assert_eq!(lines[0], legacy_abs.to_string_lossy());
        assert_eq!(lines[1], "pilot");
        assert_eq!(lines[2], "--backend=rust");

        let env_raw = fs::read_to_string(&env_capture).expect("read env capture");
        assert_eq!(env_raw.trim(), "1:rust_memory_transition_lane");
    }
}
