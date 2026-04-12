use protheus_ops_core::github_repo_collector_kernel_support::{
    map_commit_items, map_pr_items, map_release_item,
};
use serde_json::json;
use std::collections::HashSet;

#[test]
fn github_repo_mapper_rewrites_offsite_urls_to_canonical_github_paths() {
    let seen = HashSet::new();
    let release = json!({
        "tag_name": "v1.2.3",
        "html_url": "https://evil.test/release",
        "name": "Release 1.2.3"
    });
    let release_item = map_release_item(
        "protheuslabs",
        "InfRing",
        release.as_object().expect("release object"),
        &seen,
    )
    .expect("release item");
    assert_eq!(
        release_item.get("url").and_then(|v| v.as_str()),
        Some("https://github.com/protheuslabs/InfRing/releases/tag/v1.2.3")
    );

    let commits = vec![json!({
        "sha": "abc123def456",
        "html_url": "javascript:alert(1)",
        "commit": {
            "message": "Ship fix\n\nbody",
            "author": { "name": "Jay", "date": "2026-04-11T00:00:00Z" }
        }
    })];
    let commit_items = map_commit_items("protheuslabs", "InfRing", &commits, &seen);
    assert_eq!(commit_items.len(), 1);
    assert_eq!(
        commit_items[0].get("url").and_then(|v| v.as_str()),
        Some("https://github.com/protheuslabs/InfRing/commit/abc123def456")
    );

    let pulls = vec![json!({
        "number": 42,
        "updated_at": "2026-04-11T00:00:00Z",
        "html_url": "https://notgithub.test/pr/42",
        "title": "Improve sync",
        "draft": false,
        "user": { "login": "jay" }
    })];
    let pr_items = map_pr_items("protheuslabs", "InfRing", &pulls, &seen);
    assert_eq!(pr_items.len(), 1);
    assert_eq!(
        pr_items[0].get("url").and_then(|v| v.as_str()),
        Some("https://github.com/protheuslabs/InfRing/pull/42")
    );
}
