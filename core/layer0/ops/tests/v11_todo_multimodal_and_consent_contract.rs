use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde_json::Value;

fn workspace_root() -> PathBuf {
    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    manifest_dir
        .parent()
        .and_then(Path::parent)
        .and_then(Path::parent)
        .expect("workspace_root")
        .to_path_buf()
}

fn read_json(path: &Path) -> Value {
    let raw = fs::read_to_string(path).expect("read_json_file");
    serde_json::from_str(&raw).expect("parse_json")
}

#[test]
fn multimodal_fixture_manifest_and_baselines_are_complete() {
    let fixtures_root = workspace_root().join("core/layer0/ops/tests/fixtures/multimodal");
    let manifest = read_json(&fixtures_root.join("manifest.json"));
    let baselines = read_json(&fixtures_root.join("baselines.json"));

    let rows = manifest
        .get("fixtures")
        .and_then(Value::as_array)
        .expect("fixtures_array");
    assert_eq!(rows.len(), 4, "fixture count should stay deterministic");

    let mut kinds = BTreeSet::new();
    for row in rows {
        let id = row.get("id").and_then(Value::as_str).expect("fixture_id");
        let kind = row
            .get("kind")
            .and_then(Value::as_str)
            .expect("fixture_kind");
        let rel = row
            .get("path")
            .and_then(Value::as_str)
            .expect("fixture_path");
        kinds.insert(kind.to_string());

        let file_path = fixtures_root.join(rel);
        assert!(
            file_path.exists(),
            "fixture file missing for id={id}: {}",
            file_path.display()
        );

        let baseline = baselines
            .get("baselines")
            .and_then(|b| b.get(id))
            .expect("baseline_for_fixture");
        assert_eq!(
            baseline.get("expected_kind").and_then(Value::as_str),
            Some(kind),
            "baseline kind mismatch for fixture {id}"
        );
        assert!(
            baseline
                .get("fallback_strategy")
                .and_then(Value::as_str)
                .map(|v| !v.trim().is_empty())
                .unwrap_or(false),
            "fallback strategy missing for fixture {id}"
        );
    }

    let expected: BTreeSet<String> = ["image", "audio", "pdf", "sheet"]
        .iter()
        .map(|v| v.to_string())
        .collect();
    assert_eq!(kinds, expected, "fixture kinds must stay complete");
}

#[test]
fn proactive_consent_templates_cover_required_placeholders() {
    let templates_path =
        workspace_root().join("client/runtime/config/proactive_consent_templates.json");
    let templates = read_json(&templates_path);
    let obj = templates
        .get("templates")
        .and_then(Value::as_object)
        .expect("templates_object");

    let required_sources = [
        "email_digest",
        "calendar_alert",
        "system_health_alert",
        "release_watch",
    ];
    let required_tokens = [
        "{{scope}}",
        "{{cadence}}",
        "{{quiet_hours}}",
        "{{opt_out_path}}",
    ];

    for source in required_sources {
        let row = obj
            .get(source)
            .unwrap_or_else(|| panic!("missing source template: {source}"));
        let consent = row
            .get("consent_template")
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("missing consent_template for source {source}"));
        let renewal = row
            .get("renewal_template")
            .and_then(Value::as_str)
            .unwrap_or_else(|| panic!("missing renewal_template for source {source}"));

        for token in required_tokens {
            assert!(
                consent.contains(token),
                "consent template missing token {token} for source {source}"
            );
            assert!(
                renewal.contains(token),
                "renewal template missing token {token} for source {source}"
            );
        }
    }
}
