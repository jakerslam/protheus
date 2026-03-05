use serde_json::json;
use std::env;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

pub fn node_binary() -> String {
    env::var("PROTHEUS_NODE_BINARY").unwrap_or_else(|_| "node".to_string())
}

pub fn resolve_script_path(root: &Path, env_key: &str, default_rel: &str) -> PathBuf {
    let from_env = env::var(env_key)
        .ok()
        .map(|v| v.trim().to_string())
        .filter(|v| !v.is_empty());

    let script = from_env.unwrap_or_else(|| default_rel.to_string());
    let path = PathBuf::from(script);
    if path.is_absolute() {
        path
    } else {
        root.join(path)
    }
}

pub fn run_legacy_script(
    root: &Path,
    domain: &str,
    script_path: &Path,
    args: &[String],
    forward_stdin: bool,
) -> i32 {
    let mut cmd = Command::new(node_binary());
    cmd.arg(script_path)
        .args(args)
        .current_dir(root)
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit());

    if forward_stdin {
        cmd.stdin(Stdio::inherit());
    } else {
        cmd.stdin(Stdio::null());
    }

    match cmd.status() {
        Ok(status) => status.code().unwrap_or(1),
        Err(err) => {
            eprintln!(
                "{}",
                json!({
                    "ok": false,
                    "type": "rust_bridge_dispatch",
                    "domain": domain,
                    "error": format!("spawn_failed:{err}")
                })
            );
            1
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn resolve_script_path_uses_default_relative() {
        let root = tempdir().expect("tempdir");
        let path = resolve_script_path(root.path(), "NO_SUCH_ENV_KEY", "systems/foo.js");
        assert_eq!(path, root.path().join("systems/foo.js"));
    }

    #[test]
    fn resolve_script_path_honors_absolute_env_path() {
        let root = tempdir().expect("tempdir");
        let abs = root.path().join("x/y/z.js");
        env::set_var("BRIDGE_TEST_ABS", abs.to_string_lossy().to_string());
        let path = resolve_script_path(root.path(), "BRIDGE_TEST_ABS", "fallback.js");
        env::remove_var("BRIDGE_TEST_ABS");
        assert_eq!(path, abs);
    }

    #[test]
    fn resolve_script_path_honors_relative_env_path() {
        let root = tempdir().expect("tempdir");
        env::set_var("BRIDGE_TEST_REL", "systems/legacy.js");
        let path = resolve_script_path(root.path(), "BRIDGE_TEST_REL", "fallback.js");
        env::remove_var("BRIDGE_TEST_REL");
        assert_eq!(path, root.path().join("systems/legacy.js"));
    }
}
