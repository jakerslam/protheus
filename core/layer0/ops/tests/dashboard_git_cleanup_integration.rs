use protheus_ops_core::dashboard_git_runtime::{
    cleanup_agent_git_artifacts, git_branch_exists, git_current_branch, switch_agent_worktree,
    workspace_for_agent_branch,
};
use serde_json::Value;
use std::fs;
use std::process::Command;

fn init_repo() -> tempfile::TempDir {
    let dir = tempfile::tempdir().expect("tempdir");
    let status = Command::new("git")
        .args(["init"])
        .current_dir(dir.path())
        .status()
        .expect("git init");
    assert!(status.success(), "git init should succeed");
    fs::write(dir.path().join("README.md"), "seed\n").expect("write seed file");
    let status = Command::new("git")
        .args(["add", "README.md"])
        .current_dir(dir.path())
        .status()
        .expect("git add");
    assert!(status.success(), "git add should succeed");
    let status = Command::new("git")
        .args([
            "-c",
            "user.name=Codex Test",
            "-c",
            "user.email=codex@test.local",
            "commit",
            "-m",
            "init",
        ])
        .current_dir(dir.path())
        .status()
        .expect("git commit");
    assert!(status.success(), "git commit should succeed");
    dir
}

#[test]
fn cleanup_removes_agent_worktree_and_branch() {
    let root = init_repo();
    let branch = "agent-test-feature";
    let switched = switch_agent_worktree(root.path(), "agent-test", branch, true);
    assert_eq!(switched.get("ok").and_then(Value::as_bool), Some(true));
    assert!(git_branch_exists(root.path(), branch));

    let workspace = workspace_for_agent_branch(root.path(), "agent-test", branch);
    assert!(
        workspace.exists(),
        "agent worktree should exist before cleanup"
    );

    let cleanup = cleanup_agent_git_artifacts(root.path(), "agent-test", Some(branch));
    assert_eq!(cleanup.get("ok").and_then(Value::as_bool), Some(true));
    assert!(
        !git_branch_exists(root.path(), branch),
        "cleanup should delete the agent branch"
    );
    assert!(
        !workspace.exists(),
        "cleanup should remove the agent worktree directory"
    );
}

#[test]
fn cleanup_preserves_protected_default_branch() {
    let root = init_repo();
    let main_branch = git_current_branch(root.path(), "main");
    let cleanup = cleanup_agent_git_artifacts(root.path(), "agent-main", Some(&main_branch));
    assert_eq!(cleanup.get("ok").and_then(Value::as_bool), Some(true));
    assert!(
        git_branch_exists(root.path(), &main_branch),
        "cleanup must not delete default branch"
    );
}
