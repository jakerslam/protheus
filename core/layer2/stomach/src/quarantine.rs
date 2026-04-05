// SPDX-License-Identifier: Apache-2.0
// Layer ownership: core/layer2/stomach (authoritative)

use crate::stable_hash;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IngestPolicy {
    pub allow_network_during_fetch: bool,
    pub allowed_hosts: Vec<String>,
    pub forbid_hooks: bool,
    pub forbid_submodules: bool,
    pub forbid_lfs_materialization: bool,
    pub forbid_symlinks: bool,
}

impl Default for IngestPolicy {
    fn default() -> Self {
        Self {
            allow_network_during_fetch: true,
            allowed_hosts: vec!["github.com".to_string(), "gitlab.com".to_string()],
            forbid_hooks: true,
            forbid_submodules: true,
            forbid_lfs_materialization: true,
            forbid_symlinks: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SnapshotMetadata {
    pub snapshot_id: String,
    pub origin_url: String,
    pub quarantine_root: String,
    pub tree_hash: String,
    pub file_count: usize,
    pub symlink_count: usize,
    pub captured_at: String,
}

fn now_iso() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("{secs}")
}

fn sanitize_segment(raw: &str) -> String {
    raw.chars()
        .filter(|ch| ch.is_ascii_alphanumeric() || *ch == '-' || *ch == '_')
        .collect::<String>()
}

fn extract_host(origin: &str) -> Option<String> {
    let value = origin.trim().to_ascii_lowercase();
    if let Some(rest) = value.strip_prefix("https://") {
        return rest
            .split('/')
            .next()
            .map(|row| row.to_string())
            .filter(|row| !row.is_empty());
    }
    if let Some(rest) = value.strip_prefix("http://") {
        return rest
            .split('/')
            .next()
            .map(|row| row.to_string())
            .filter(|row| !row.is_empty());
    }
    None
}

pub fn origin_allowed(policy: &IngestPolicy, origin_url: &str) -> bool {
    let Some(host) = extract_host(origin_url) else {
        return false;
    };
    policy
        .allowed_hosts
        .iter()
        .map(|row| row.trim().to_ascii_lowercase())
        .any(|row| row == host)
}

fn copy_tree_denying_symlinks(
    source: &Path,
    target: &Path,
    forbid_symlinks: bool,
) -> Result<(), String> {
    for entry in WalkDir::new(source).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        let rel = match path.strip_prefix(source) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if rel.as_os_str().is_empty() {
            continue;
        }
        let dest = target.join(rel);
        let meta = fs::symlink_metadata(path).map_err(|e| format!("quarantine_stat_failed:{e}"))?;
        if meta.file_type().is_symlink() {
            if forbid_symlinks {
                return Err(format!("quarantine_symlink_forbidden:{}", rel.display()));
            }
            continue;
        }
        if meta.is_dir() {
            fs::create_dir_all(&dest).map_err(|e| format!("quarantine_mkdir_failed:{e}"))?;
            continue;
        }
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)
                .map_err(|e| format!("quarantine_parent_mkdir_failed:{e}"))?;
        }
        fs::copy(path, &dest).map_err(|e| format!("quarantine_copy_failed:{e}"))?;
    }
    Ok(())
}

fn compute_tree_stats(root: &Path) -> Result<(String, usize, usize), String> {
    let mut rows = Vec::<(String, String)>::new();
    let mut file_count = 0usize;
    let mut symlink_count = 0usize;

    for entry in WalkDir::new(root).into_iter().filter_map(Result::ok) {
        let path = entry.path();
        if path
            .components()
            .any(|c| c.as_os_str() == ".git" || c.as_os_str() == ".github")
        {
            continue;
        }
        let rel = match path.strip_prefix(root) {
            Ok(v) => v,
            Err(_) => continue,
        };
        if rel.as_os_str().is_empty() {
            continue;
        }
        let rel_s = rel.display().to_string();
        let meta =
            fs::symlink_metadata(path).map_err(|e| format!("quarantine_walk_stat_failed:{e}"))?;
        if meta.file_type().is_symlink() {
            symlink_count += 1;
            rows.push((rel_s, "symlink".to_string()));
            continue;
        }
        if meta.is_file() {
            file_count += 1;
            let mut file =
                fs::File::open(path).map_err(|e| format!("quarantine_open_failed:{e}"))?;
            let mut buf = Vec::<u8>::new();
            file.read_to_end(&mut buf)
                .map_err(|e| format!("quarantine_read_failed:{e}"))?;
            let hash = stable_hash(&buf);
            rows.push((rel_s, hash));
        }
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0));
    Ok((stable_hash(&rows), file_count, symlink_count))
}

pub fn create_quarantine_snapshot(
    state_root: &Path,
    snapshot_id: &str,
    source_root: &Path,
    origin_url: &str,
    policy: &IngestPolicy,
) -> Result<SnapshotMetadata, String> {
    if !source_root.exists() || !source_root.is_dir() {
        return Err("quarantine_source_root_missing".to_string());
    }
    if !origin_allowed(policy, origin_url) {
        return Err("quarantine_origin_not_allowlisted".to_string());
    }
    let snapshot_key = sanitize_segment(snapshot_id);
    if snapshot_key.is_empty() {
        return Err("quarantine_snapshot_id_invalid".to_string());
    }
    let target_root: PathBuf = state_root
        .join("quarantine")
        .join(snapshot_key)
        .join("source");
    if target_root.exists() {
        fs::remove_dir_all(&target_root).map_err(|e| format!("quarantine_cleanup_failed:{e}"))?;
    }
    fs::create_dir_all(&target_root).map_err(|e| format!("quarantine_target_mkdir_failed:{e}"))?;
    copy_tree_denying_symlinks(source_root, &target_root, policy.forbid_symlinks)?;
    let (tree_hash, file_count, symlink_count) = compute_tree_stats(&target_root)?;

    Ok(SnapshotMetadata {
        snapshot_id: snapshot_id.to_string(),
        origin_url: origin_url.to_string(),
        quarantine_root: target_root.display().to_string(),
        tree_hash,
        file_count,
        symlink_count,
        captured_at: now_iso(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use tempfile::tempdir;

    #[test]
    fn blocks_non_allowlisted_origin() {
        let root = tempdir().expect("tmp");
        let src = root.path().join("src");
        fs::create_dir_all(&src).expect("mkdir");
        fs::write(src.join("a.txt"), "hello").expect("write");
        let out = create_quarantine_snapshot(
            root.path(),
            "demo",
            &src,
            "https://example.org/repo",
            &IngestPolicy::default(),
        );
        assert!(out.is_err());
    }

    #[test]
    fn creates_snapshot_for_allowlisted_origin() {
        let root = tempdir().expect("tmp");
        let src = root.path().join("src");
        fs::create_dir_all(&src).expect("mkdir");
        fs::write(src.join("a.txt"), "hello").expect("write");
        let out = create_quarantine_snapshot(
            root.path(),
            "demo",
            &src,
            "https://github.com/acme/repo",
            &IngestPolicy::default(),
        )
        .expect("snapshot");
        assert!(out.file_count >= 1);
        assert!(!out.tree_hash.is_empty());
    }
}
