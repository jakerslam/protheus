// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer0/ops (authoritative)

use std::path::{Path, PathBuf};

pub(crate) fn bool_flag(args: &[String], name: &str) -> bool {
    args.iter()
        .any(|arg| arg == name || arg == &format!("{name}=1") || arg == &format!("{name}=true"))
}

pub(crate) fn option_path(args: &[String], name: &str, fallback: PathBuf) -> PathBuf {
    let prefix = format!("{name}=");
    args.iter()
        .find_map(|arg| arg.strip_prefix(&prefix).map(PathBuf::from))
        .unwrap_or(fallback)
}

pub(crate) fn option_usize(args: &[String], name: &str, fallback: usize) -> usize {
    let prefix = format!("{name}=");
    args.iter()
        .find_map(|arg| arg.strip_prefix(&prefix).and_then(|raw| raw.parse::<usize>().ok()))
        .unwrap_or(fallback)
}

fn workspace_root(root: &Path) -> PathBuf {
    std::env::var_os("INFRING_WORKSPACE")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
        .unwrap_or_else(|| root.to_path_buf())
}

pub(crate) fn state_dir_from_args(root: &Path, args: &[String]) -> PathBuf {
    let explicit = option_path(args, "--state-dir", PathBuf::new());
    if !explicit.as_os_str().is_empty() {
        return explicit;
    }
    let state_root = option_path(args, "--state-root", PathBuf::new());
    if state_root.as_os_str().is_empty() {
        workspace_root(root).join("local/state/kernel_sentinel")
    } else {
        state_root.join("kernel_sentinel")
    }
}
